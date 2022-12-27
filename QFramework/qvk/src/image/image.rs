use std::ffi::c_void;
use std::sync::{Arc, Mutex};

use ash::vk;
use log::{debug, info};

use crate::command::{CommandBufferFactory, CommandBufferSource, Executor};
use crate::image::ImageSource;
use crate::init::{DeviceSource, InstanceSource};
use crate::memory::{MemorySource, partitionsystem,  Memory, PartitionSystem};

use super::{Image, ImageFactory};

impl<D:DeviceSource + Clone, M: DeviceSource + MemorySource + Clone> ImageFactory<D, Arc<Image<D, M>>> for M{
    fn create_image(&self, device_source: &D, format: vk::Format, extent: vk::Extent3D, levels: u32, layers: u32, usage: vk::ImageUsageFlags, extensions: Option<*const c_void>) -> Result<Arc<Image<D,M>>, ImageCreateError> {
        let mut info = vk::ImageCreateInfo::builder();
        if let Some(ptr) = extensions{
           info.p_next = ptr; 
        }
        if let Some(flags) = ImageFactory::<D,Arc<Image<D,M>>>::create_flags(self){
            info = info.flags(flags);
        }
        info = info.image_type(ImageFactory::<D,Arc<Image<D,M>>>::image_type(self));
        info = info.format(format);
        info = info.extent(extent);
        info = info.mip_levels(levels);
        info = info.array_layers(layers);
        info = info.samples(ImageFactory::<D,Arc<Image<D,M>>>::samples(self));
        info = info.tiling(ImageFactory::<D,Arc<Image<D,M>>>::tiling(self));
        info = info.usage(usage);
        info = info.sharing_mode(vk::SharingMode::EXCLUSIVE);
        let indices = ImageFactory::<D,Arc<Image<D,M>>>::share(self);
        if let Some(indices) = &indices{
            info = info.sharing_mode(vk::SharingMode::CONCURRENT);
            info = info.queue_family_indices(indices);
        }
        info = info.initial_layout(vk::ImageLayout::UNDEFINED);

        let device = self.device();
        let image = unsafe{device.create_image(&info, None)};
        if let Err(e) = image{
            return Err(ImageCreateError::Vulkan(e));
        }
        let image = image.unwrap();
        info!("Created image {:?}", image);

        let reqs = unsafe{device.get_image_memory_requirements(image)};
        let memory_partition = self.partition(reqs.size, Some(reqs.alignment));
        if let Err(e) = memory_partition{
            return Err(ImageCreateError::Memory(e));
        }
        let memory_partition = memory_partition.unwrap();

        let res = unsafe{device.bind_image_memory(image, *self.memory(), memory_partition.offset)};
        if let Err(e) = res{
            return Err(ImageCreateError::Vulkan(e));
        }

        let image = Arc::new(Image{
            device: device_source.clone(),
            memory: Some(self.clone()),
            _partition: Some(memory_partition),
            image,
            create_info: info.clone(),
            current_layout: Arc::new(Mutex::new(info.initial_layout)),
        });

        Ok(image)
    }
}

#[derive(Debug)]
pub enum ImageCreateError{
    Memory(partitionsystem::PartitionError),
    Vulkan(vk::Result),
}

impl<D:DeviceSource + Clone, M:MemorySource + DeviceSource + Clone> Image<D,M>{
    pub fn from_swapchain_image(device_provider: &D, image: vk::Image, image_extent: vk::Extent2D) -> Arc<Image<D, Arc<Memory<D, PartitionSystem>>>>{
        let extent = vk::Extent3D::builder().width(image_extent.width).height(image_extent.height).depth(1).build();
        Arc::new(
            Image{
                device: device_provider.clone(),
                memory: None,
                _partition: None,
                image,
                create_info: vk::ImageCreateInfo::builder()
                    .mip_levels(1)
                    .array_layers(1)
                    .extent(extent)
                    .build(),
                current_layout: Arc::new(Mutex::new(vk::ImageLayout::UNDEFINED)),
            }
        )
    }
}

impl<D:DeviceSource + Clone, M:MemorySource + DeviceSource> ImageSource for Arc<Image<D,M>>{
    fn transition<C:CommandBufferSource>(
        &self, 
        cmd: &C, 
        new_layout: vk::ImageLayout, 
        src_stage: Option<vk::PipelineStageFlags2>,
        dst_stage: Option<vk::PipelineStageFlags2>,
        src_access: Option<vk::AccessFlags2>,
        dst_access: Option<vk::AccessFlags2>,
        subresources: Option<vk::ImageSubresourceRange>,
    ) {
        let mut lock = self.current_layout.lock().unwrap();
        if *lock == new_layout{
            return;
        }
        let mut image_transition = vk::ImageMemoryBarrier2::builder();
        if let Some(stage) = src_stage{
            image_transition = image_transition.src_stage_mask(stage);
        }
        else{
            image_transition = image_transition.src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE);
        }
        
        if let Some(stage) = dst_stage{
            image_transition = image_transition.dst_stage_mask(stage);
        }
        else{
            image_transition = image_transition.dst_stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE);
        }
        
        if let Some(access) = src_access{
            image_transition = image_transition.src_access_mask(access);
        }
        else{
            image_transition = image_transition.src_access_mask(vk::AccessFlags2::MEMORY_WRITE);
        }
        
        if let Some(access) = dst_access{
            image_transition = image_transition.dst_access_mask(access);
        }
        else{
            image_transition = image_transition.dst_access_mask(vk::AccessFlags2::MEMORY_READ);
        }

        if let Some(range) = subresources{
            image_transition = image_transition.subresource_range(range);
        }
        else{
            image_transition = image_transition.subresource_range(vk::ImageSubresourceRange{
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_mip_level: 0,
                level_count: self.create_info.mip_levels,
                base_array_layer: 0,
                layer_count: self.create_info.array_layers,
            })
        }

        image_transition = image_transition.old_layout(*lock);
        image_transition = image_transition.new_layout(new_layout);
        image_transition = image_transition.image(self.image);

        let image_transition = [image_transition.build()];

        let info = vk::DependencyInfo::builder()
        .image_memory_barriers(&image_transition);

        debug!("Transitioning layer range {:?} of image {:?} from layout {:?} to layout {:?}", image_transition[0].subresource_range, self.image, *lock, new_layout);

        unsafe{self.device.device().cmd_pipeline_barrier2(cmd.cmd(), &info)};

        *lock = new_layout;
    }

    fn internal_transistion(&self, new_layout: vk::ImageLayout, subresources: Option<vk::ImageSubresourceRange>) {
        let old_layout = *ImageSource::layout(self).lock().unwrap();
        if old_layout == new_layout{
            return;
        }
        let executor = Executor::new(&self.device, vk::QueueFlags::GRAPHICS);
        let cmd = executor.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        
        if let Some(range) = subresources{
            self.transition(&cmd, new_layout, None, None, None, None,Some(range));
        }
        else{
            self.transition(&cmd, new_layout, None, None, None, None, None);
        }
        
        cmd.end().unwrap();
        executor.wait_submit_internal();
    }

    fn image(&self) -> &vk::Image {
        &self.image
    }

    fn layout(&self) -> Arc<Mutex<vk::ImageLayout>> {
        self.current_layout.clone()
    }

    fn mip_levels(&self) -> u32 {
        self.create_info.mip_levels
    }

    fn array_layers(&self) -> u32 {
        self.create_info.array_layers
    }

    fn extent(&self) -> vk::Extent3D {
        self.create_info.extent
    }

}


impl<D:DeviceSource, M:MemorySource + DeviceSource> Drop for Image<D,M>{
    fn drop(&mut self) {
        debug!("Destroyed image {:?}", self.image);
        if let Some(_) = self.memory{
            unsafe{
                self.device.device().destroy_image(self.image, None);
            }
        }
    }
}

impl<D:DeviceSource + InstanceSource, M:MemorySource + DeviceSource> InstanceSource for Arc<Image<D,M>>{
    
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}

impl<D:DeviceSource, M:MemorySource + DeviceSource> MemorySource for Arc<Image<D,M>>{
    fn partition(&self, size: u64, alignment: Option<u64>) -> Result<crate::memory::Partition, partitionsystem::PartitionError> {
        if let Some(m) = &self.memory{
            return m.partition(size, alignment);
        }

        Err(partitionsystem::PartitionError::NoSpace)

    }

    fn memory(&self) -> &vk::DeviceMemory {
        if let Some(m) = &self.memory{
            return m.memory();
        }
        panic!("Image not created from application created memory");
    }
}

impl<D:DeviceSource, M:MemorySource + DeviceSource> DeviceSource for Arc<Image<D,M>>{
    fn device(&self) -> &ash::Device {
        self.device.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.device.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.device.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.device.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.device.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.device.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.device.host_memory_index()
    }
}


