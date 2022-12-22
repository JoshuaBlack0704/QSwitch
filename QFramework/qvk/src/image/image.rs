use std::sync::{Arc, Mutex};

use ash::vk;
use log::{debug, info};

use crate::command::{CommandBufferFactory, CommandBufferStore, Executor};
use crate::image::ImageStore;
use crate::init::{DeviceStore, DeviceSupplier};
use crate::memory::{MemoryStore, partitionsystem,  Memory, PartitionSystem};

use super::Image;

pub trait ImageSettingsStore{
    fn extensions(&self) -> Option<Vec<ImageCreateExtensions>>;
    fn create_flags(&self) -> Option<vk::ImageCreateFlags>;
    fn image_type(&self) -> vk::ImageType;
    fn format(&self) -> vk::Format;
    fn extent(&self) -> vk::Extent3D;
    fn mip_levels(&self) -> u32;
    fn array_layers(&self) -> u32;
    fn samples(&self) -> vk::SampleCountFlags;
    fn tiling(&self) -> vk::ImageTiling;
    fn usage(&self) -> vk::ImageUsageFlags;
    fn share(&self) -> Option<&[u32]>;
    fn preload_layout(&self) -> Option<vk::ImageLayout>;
}

#[derive(Clone)]
pub enum ImageCreateExtensions{
    
}

#[derive(Debug)]
pub enum ImageCreateError{
    Memory(partitionsystem::PartitionError),
    Vulkan(vk::Result),
}

pub struct SettingsStore{
    extensions:  Option<Vec<ImageCreateExtensions>>,
    create_flags:  Option<vk::ImageCreateFlags>,
    image_type:  vk::ImageType,
    format:  vk::Format,
    extent:  vk::Extent3D,
    mip_levels:  u32,
    array_layers:  u32,
    samples:  vk::SampleCountFlags,
    tiling:  vk::ImageTiling,
    usage:  vk::ImageUsageFlags,
    share:  Option<Vec<u32>>,
    preload_layout:  Option<vk::ImageLayout>,
    
}

impl<D:DeviceStore + Clone, M:MemoryStore + Clone> Image<D,M>{
    pub fn new<S:ImageSettingsStore>(device_provider: &D, memory_provider: &M, settings: &S) -> Result<Arc<Image<D,M>>, ImageCreateError> {
        let mut info = vk::ImageCreateInfo::builder();
        let extensions = settings.extensions();
        if let Some(mut ext) = extensions{
            for ext in ext.iter_mut(){
                match ext{
                   _ => todo!() 
                }
                
            }
        }
        if let Some(flags) = settings.create_flags(){
            info = info.flags(flags);
        }
        info = info.image_type(settings.image_type());
        info = info.format(settings.format());
        info = info.extent(settings.extent());
        info = info.mip_levels(settings.mip_levels());
        info = info.array_layers(settings.array_layers());
        info = info.samples(settings.samples());
        info = info.tiling(settings.tiling());
        info = info.usage(settings.usage());
        info = info.sharing_mode(vk::SharingMode::EXCLUSIVE);
        if let Some(indices) = settings.share(){
            info = info.sharing_mode(vk::SharingMode::CONCURRENT);
            info = info.queue_family_indices(indices);
        }
        info = info.initial_layout(vk::ImageLayout::UNDEFINED);

        let device = device_provider.device();
        let image = unsafe{device.create_image(&info, None)};
        if let Err(e) = image{
            return Err(ImageCreateError::Vulkan(e));
        }
        let image = image.unwrap();
        info!("Created image {:?}", image);

        let reqs = unsafe{device.get_image_memory_requirements(image)};
        let memory_partition = memory_provider.partition(reqs.size, Some(reqs.alignment));
        if let Err(e) = memory_partition{
            return Err(ImageCreateError::Memory(e));
        }
        let memory_partition = memory_partition.unwrap();

        let res = unsafe{device.bind_image_memory(image, *memory_provider.memory(), memory_partition.offset)};
        if let Err(e) = res{
            return Err(ImageCreateError::Vulkan(e));
        }

        let image = Arc::new(Image{
            device: device_provider.clone(),
            memory: Some(memory_provider.clone()),
            _partition: Some(memory_partition),
            image,
            create_info: info.clone(),
            current_layout: Arc::new(Mutex::new(info.initial_layout)),
        });

        if let Some(layout) = settings.preload_layout(){
            image.internal_transistion(layout, None);
        }

        Ok(image)
    }

    pub fn from_swapchain_image(device_provider: &D, image: vk::Image, image_extent: vk::Extent2D) -> Arc<Image<D,Arc<Memory<D, PartitionSystem>>>>{
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

impl<D:DeviceStore + Clone, M:MemoryStore> ImageStore for Arc<Image<D,M>>{
    fn transition<C:CommandBufferStore>(
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
        let old_layout = *ImageStore::layout(self).lock().unwrap();
        if old_layout == new_layout{
            return;
        }
        let executor = Executor::new(&self.device, vk::QueueFlags::GRAPHICS);
        let cmd = executor.next_cmd();
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


impl<D:DeviceStore, M:MemoryStore> Drop for Image<D,M>{
    fn drop(&mut self) {
        debug!("Destroyed image {:?}", self.image);
        if let Some(_) = self.memory{
            unsafe{
                self.device.device().destroy_image(self.image, None);
            }
        }
    }
}

impl<D:DeviceStore, M:MemoryStore> DeviceSupplier<D> for Arc<Image<D,M>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}

impl SettingsStore{
    pub fn new(
        extensions:  Option<Vec<ImageCreateExtensions>>,
        create_flags:  Option<vk::ImageCreateFlags>,
        image_type:  vk::ImageType,
        format:  vk::Format,
        extent:  vk::Extent3D,
        mip_levels:  u32,
        array_layers:  u32,
        samples:  vk::SampleCountFlags,
        tiling:  vk::ImageTiling,
        usage:  vk::ImageUsageFlags,
        share:  Option<Vec<u32>>,
        preload_layout:  Option<vk::ImageLayout>,
    ) -> SettingsStore {
        Self{
            extensions,
            create_flags,
            image_type,
            format,
            extent,
            mip_levels,
            array_layers,
            samples,
            tiling,
            usage,
            share,
            preload_layout,
        }
        
    }

    pub fn new_simple(
        format:  vk::Format,
        extent:  vk::Extent3D,
        usage:  vk::ImageUsageFlags,
        preload_layout:  Option<vk::ImageLayout>,
    )
-> SettingsStore     {
        Self::new(None, None, vk::ImageType::TYPE_2D, format, extent, 1, 1, vk::SampleCountFlags::TYPE_1, vk::ImageTiling::OPTIMAL, usage, None, preload_layout)
    }
}

impl ImageSettingsStore for SettingsStore{
    fn extensions(&self) -> Option<Vec<ImageCreateExtensions>> {
        self.extensions.clone()
    }

    fn create_flags(&self) -> Option<vk::ImageCreateFlags> {
        self.create_flags
    }

    fn image_type(&self) -> vk::ImageType {
        self.image_type
    }

    fn format(&self) -> vk::Format {
        self.format
    }

    fn extent(&self) -> vk::Extent3D {
        self.extent
    }

    fn mip_levels(&self) -> u32 {
        self.mip_levels
    }

    fn array_layers(&self) -> u32 {
        self.array_layers
    }

    fn samples(&self) -> vk::SampleCountFlags {
        self.samples
    }

    fn tiling(&self) -> vk::ImageTiling {
        self.tiling
    }

    fn usage(&self) -> vk::ImageUsageFlags {
        self.usage
    }

    fn share(&self) -> Option<&[u32]> {
        if let Some(indices) = &self.share{
            return Some(&indices);
        }
        None
    }

    fn preload_layout(&self) -> Option<vk::ImageLayout> {
        self.preload_layout
    }
}
