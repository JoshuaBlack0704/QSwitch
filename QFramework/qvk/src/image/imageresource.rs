use std::sync::{Arc, MutexGuard, Mutex};

use image::{self, EncodableLayout};

use ash::vk;

use crate::{command::{CommandBufferSource, ImageCopyFactory, BufferCopyFactory, Executor, ImageTransitionFactory}, memory::{buffer::{BufferFactory, BufferSegmentSource}, MemoryFactory, MemorySource},  image::ImageResource};
use crate::command::CommandBufferFactory;
use crate::image::{ImageSource, ImageResourceSource};
use crate::init::{DeviceSource, InstanceSource};
use crate::memory::buffer::{BufferSource, BufferSegmentFactory};

use super::{ImageFactory, ImageResourceFactory};

#[derive(Debug)]
pub enum ImageResourceCreateError{
    ResourcesDontExist,
}

#[derive(Debug)]
pub enum ImageResourceMemOpError{
    
}

impl<Img:ImageSource + DeviceSource + InstanceSource + Clone> ImageResourceFactory<Arc<ImageResource<Img>>> for Img{
    fn create_resource(&self, offset: vk::Offset3D, extent: vk::Extent3D, level: u32, aspect: vk::ImageAspectFlags) -> Result<Arc<ImageResource<Img>>, ImageResourceCreateError> {
        let image_provider = self;
        if level > image_provider.mip_levels(){
            return Err(ImageResourceCreateError::ResourcesDontExist);
        }
        let req_size = vk::Extent3D::builder()
        .width(offset.x as u32 + extent.width)
        .height(offset.y as u32 + extent.height)
        .depth(offset.z as u32 + extent.depth)
        .build();
        let act_size = image_provider.extent();
        if req_size.width > act_size.width || req_size.height > act_size.height || req_size.depth > act_size.depth{
            return Err(ImageResourceCreateError::ResourcesDontExist);
        }

        let layer = vk::ImageSubresourceLayers::builder()
        .aspect_mask(aspect)
        .mip_level(level)
        .base_array_layer(0)
        .layer_count(1)
        .build();

        let layout = image_provider.layout();

        Ok(
            Arc::new(
                ImageResource{
                    image: image_provider.clone(),
                    resorces: layer,
                    offset,
                    extent,
                    layout,
                    _aspect: aspect,
                }
            )
        )
    }
}

impl<Img:ImageSource + DeviceSource + InstanceSource + Clone> ImageResource<Img>{
    pub fn load_image(tgt: &Arc<Self>, file: &String){
        let reader = image::io::Reader::open(file).unwrap();
        let data = reader.decode().unwrap();
        let image = data.to_rgba8();
        let bytes = image.as_bytes();
        let image_extent = vk::Extent3D::builder().width(image.width()).height(image.height()).depth(1).build();

        let dev_mem = tgt.image.create_memory(bytes.len() as u64 * 2, tgt.image.device_memory_index(), None).unwrap();
        let image = dev_mem.create_image(&tgt.image, vk::Format::R8G8B8A8_SRGB, image_extent, 1, 1, vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, None).unwrap();
        image.internal_transistion(vk::ImageLayout::TRANSFER_DST_OPTIMAL, None);
        let resource = image.create_resource(vk::Offset3D::default(), image.extent(), 0, vk::ImageAspectFlags::COLOR).unwrap();
        
        let host_mem = tgt.image.create_memory(bytes.len() as u64 * 2, tgt.image.host_memory_index(), None).unwrap();
        let buf = host_mem.create_buffer(bytes.len() as u64 * 2, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, None, None).unwrap();
        let seg = buf.create_segment(bytes.len() as u64, None).unwrap();
        seg.copy_from_ram(&bytes).unwrap();
        seg.copy_to_image_internal(&resource, None).unwrap();

        image.internal_transistion(vk::ImageLayout::TRANSFER_SRC_OPTIMAL, None);
        resource.blit_to_image_internal(tgt, vk::Filter::LINEAR).unwrap();
        

        
    }
}

impl<Img:ImageSource + DeviceSource + InstanceSource + Clone> ImageResourceSource for Arc<ImageResource<Img>>{
    fn subresource(&self) -> vk::ImageSubresourceLayers {
        self.resorces.clone()
    }

    fn offset(&self) -> vk::Offset3D {
        self.offset
    }

    fn extent(&self) -> vk::Extent3D {
        self.extent
    }

    fn layout(&self) -> MutexGuard<vk::ImageLayout> {
        self.layout.lock().unwrap()
    }

    fn copy_to_buffer_internal<BP:BufferCopyFactory + BufferSource>(&self, dst: &BP, buffer_addressing: Option<(u32,u32)>) -> Result<(), ImageResourceMemOpError> {
        
        let exe = Executor::new(&self.image, vk::QueueFlags::GRAPHICS);
        
        let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        cmd.image_buffer_copy(self, dst, buffer_addressing).unwrap();
        cmd.end().unwrap();
        
        exe.wait_submit_internal();
        Ok(())
    }

    fn copy_to_image_internal<IR:ImageCopyFactory + DeviceSource>(&self, dst: &IR) -> Result<(), ImageResourceMemOpError> {
        if self.extent.width == 0{
            return Ok(());
        }
        if self.extent.height== 0{
            return Ok(());
        }
        if self.extent.depth == 0{
            return Ok(());
        }
        if dst.extent().width == 0{
            return Ok(());
        }
        if dst.extent().height== 0{
            return Ok(());
        }
        if dst.extent().depth == 0{
            return Ok(());
        }
        
        let exe = Executor::new(&self.image, vk::QueueFlags::GRAPHICS);
        
        let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        cmd.image_copy(self, dst).unwrap();
        cmd.end().unwrap();
        
        exe.wait_submit_internal();
        Ok(())
    }

    fn blit_to_image_internal<IR:ImageCopyFactory>(&self, dst: &IR, scale_filter: vk::Filter) -> Result<(), ImageResourceMemOpError> {
        if self.extent.width == 0{
            return Ok(());
        }
        if self.extent.height== 0{
            return Ok(());
        }
        if self.extent.depth == 0{
            return Ok(());
        }
        if dst.extent().width == 0{
            return Ok(());
        }
        if dst.extent().height== 0{
            return Ok(());
        }
        if dst.extent().depth == 0{
            return Ok(());
        }

        let exe = Executor::new(&self.image, vk::QueueFlags::GRAPHICS);
        
        let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        cmd.image_blit(self, dst, scale_filter).unwrap();
        cmd.end().unwrap();
        
        exe.wait_submit_internal();
        Ok(())
    }

    fn aspect(&self) -> vk::ImageAspectFlags {
        self._aspect
    }

    fn level(&self) -> u32 {
        self.resorces.mip_level
    }


}

impl<Img:ImageSource + DeviceSource + InstanceSource> ImageCopyFactory for Arc<ImageResource<Img>>{
    fn extent(&self) -> vk::Extent3D {
        self.extent
    }

    fn subresource(&self) -> vk::ImageSubresourceLayers {
        self.resorces
    }

    fn offset(&self) -> vk::Offset3D {
        self.offset
    }

    fn layout(&self) -> MutexGuard<vk::ImageLayout> {
        self.layout.lock().unwrap()
    }

    fn image(&self) -> vk::Image {
        *self.image.image()
    }
}

impl<Img:ImageSource + DeviceSource + InstanceSource> ImageSource for Arc<ImageResource<Img>>{
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
        self.image.transition(cmd,new_layout,src_stage,dst_stage,src_access,dst_access,subresources);
    }

    fn internal_transistion(&self, new_layout: vk::ImageLayout, subresources: Option<vk::ImageSubresourceRange>) {
        self.image.internal_transistion(new_layout,subresources);
    }

    fn image(&self) -> &vk::Image {
        self.image.image()
    }

    fn layout(&self) -> Arc<Mutex<vk::ImageLayout>> {
        self.image.layout()
    }

    fn mip_levels(&self) -> u32 {
        self.image.mip_levels()
    }

    fn array_layers(&self) -> u32 {
        self.image.array_layers()
    }

    fn extent(&self) -> vk::Extent3D {
        self.image.extent()
    }
}

impl<Img:ImageSource + DeviceSource + InstanceSource> ImageTransitionFactory for Arc<ImageResource<Img>>{
    fn image(&self) -> vk::Image {
        *self.image.image()
    }

    fn range(&self) -> vk::ImageSubresourceRange {

        vk::ImageSubresourceRange{
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
        }
    }

    fn old_layout(&self) -> Arc<Mutex<vk::ImageLayout>> {
        self.image.layout()
    }
}

impl<Img:ImageSource + DeviceSource + InstanceSource> InstanceSource for Arc<ImageResource<Img>>{
    
    fn instance(&self) -> &ash::Instance {
        self.image.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.image.entry()
    }
}

impl<Img:ImageSource + DeviceSource + InstanceSource + MemorySource> MemorySource for Arc<ImageResource<Img>>{
    fn partition(&self, size: u64, alignment: Option<u64>) -> Result<crate::memory::Partition, crate::memory::partitionsystem::PartitionError> {
        self.image.partition(size, alignment)
    }

    fn memory(&self) -> &vk::DeviceMemory {
        self.image.memory()
    }
}

impl<Img:ImageSource + DeviceSource + InstanceSource> DeviceSource for Arc<ImageResource<Img>>{
    
    fn device(&self) -> &ash::Device {
        self.image.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.image.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.image.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.image.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.image.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.image.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.image.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.image.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.image.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.image.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.image.host_memory_index()
    }
}
