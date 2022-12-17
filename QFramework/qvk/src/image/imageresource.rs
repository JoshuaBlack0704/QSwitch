use std::sync::{MutexGuard, Arc};

use ash::vk;
use log::debug;

use crate::{device::{UsesDeviceProvider, DeviceProvider}, memory::buffer::{buffer::{BufferProvider, UsesBufferProvider}, bufferpartition::BufferPartitionProvider}, commandpool, CommandPool, commandbuffer::{self, CommandBufferProvider}, CommandBufferSet, queue::{SubmitSet, submit::SubmitInfoProvider, Queue, queue::QueueProvider}};

use super::{image::{ImageProvider, UsesImageProvider}, ImageResource};

pub trait ImageSubresourceProvider{
    fn subresource(&self) -> vk::ImageSubresourceLayers;
    fn offset(&self) -> vk::Offset3D;
    fn extent(&self) -> vk::Extent3D;
    fn layout(&self) -> MutexGuard<vk::ImageLayout>;
    fn copy_to_buffer<B:BufferProvider, BP:BufferPartitionProvider + UsesBufferProvider<B>>(&self, cmd: &Arc<vk::CommandBuffer>, dst: &Arc<BP>, buffer_addressing: Option<(u32,u32)>) -> Result<(), ImageResourceMemOpError>;
    fn copy_to_buffer_internal<B:BufferProvider, BP:BufferPartitionProvider + UsesBufferProvider<B>>(&self, dst: &Arc<BP>, buffer_addressing: Option<(u32,u32)>) -> Result<(), ImageResourceMemOpError>;
    fn copy_to_image<I:ImageProvider, IR:ImageSubresourceProvider + UsesImageProvider<I>>(&self, cmd: &Arc<vk::CommandBuffer>, dst: &Arc<IR>) -> Result<(), ImageResourceMemOpError>;
    fn copy_to_image_internal<I:ImageProvider, IR:ImageSubresourceProvider + UsesImageProvider<I>>(&self, dst: &Arc<IR>) -> Result<(), ImageResourceMemOpError>;
}

#[derive(Clone, Debug)]
pub enum ImageResourceCreateError{
    ResorcesDontExist,
}

#[derive(Debug)]
pub enum ImageResourceMemOpError{
    
}

impl<D:DeviceProvider, I:ImageProvider + UsesDeviceProvider<D>> ImageResource<D,I>{
    pub fn new(image_provider: &Arc<I>, aspect: vk::ImageAspectFlags, miplevel: u32, array_layer: u32, layer_count: u32, offset: vk::Offset3D, extent: vk::Extent3D) -> Result<Arc<Self>, ImageResourceCreateError>{
        
        if miplevel > image_provider.mip_levels(){
            return Err(ImageResourceCreateError::ResorcesDontExist);
        }
        if array_layer + layer_count > image_provider.array_layers(){
            return Err(ImageResourceCreateError::ResorcesDontExist);
        }
        let req_size = vk::Extent3D::builder()
        .width(offset.x as u32 + extent.width)
        .height(offset.y as u32 + extent.height)
        .depth(offset.z as u32 + extent.depth)
        .build();
        let act_size = image_provider.extent();
        if req_size.width > act_size.width || req_size.height > act_size.height || req_size.depth > act_size.depth{
            return Err(ImageResourceCreateError::ResorcesDontExist);
        }

        let layer = vk::ImageSubresourceLayers::builder()
        .aspect_mask(aspect)
        .mip_level(miplevel)
        .base_array_layer(array_layer)
        .layer_count(layer_count)
        .build();

        let layout = image_provider.layout();

        Ok(
            Arc::new(
                Self{
                    image: image_provider.clone(),
                    resorces: layer,
                    offset,
                    extent,
                    layout,
                    _device: std::marker::PhantomData,
                }
            )
        )

    }
}

impl<D:DeviceProvider, I:ImageProvider + UsesDeviceProvider<D>> ImageSubresourceProvider for ImageResource<D,I>{
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

    fn copy_to_buffer<B:BufferProvider, BP:BufferPartitionProvider + UsesBufferProvider<B>>(&self, cmd: &Arc<vk::CommandBuffer>, dst: &Arc<BP>, buffer_addressing: Option<(u32,u32)>) -> Result<(), ImageResourceMemOpError> {
        let buffer_offset = dst.get_partition().offset();
        let mut addressing = (0,0);
        if let Some(a) = buffer_addressing{
            addressing = a;
        }

        let subresource = self.resorces;
        let offset = self.offset;
        let extent = self.extent;
        let image = self.image.image();
        let layout = self.layout();
        
        let info = [vk::BufferImageCopy::builder()
        .buffer_offset(buffer_offset)
        .buffer_row_length(addressing.0)
        .buffer_image_height(addressing.1)
        .image_subresource(subresource)
        .image_offset(offset)
        .image_extent(extent)
        .build()];

        unsafe{
            let device = self.image.device_provider().device();
            let buffer = dst.buffer_provider().buffer();
            debug!("Copying image layer {:?} from image {:?} to buffer {:?}", self.resorces, *image, *buffer);
            device.cmd_copy_image_to_buffer(**cmd, *image, *layout, *buffer, &info);
        }
        Ok(())
    }

    fn copy_to_buffer_internal<B:BufferProvider, BP:BufferPartitionProvider + UsesBufferProvider<B>>(&self, dst: &Arc<BP>, buffer_addressing: Option<(u32,u32)>) -> Result<(), ImageResourceMemOpError> {
        let settings = commandpool::SettingsProvider::new(self.image.device_provider().transfer_queue().unwrap().1);
        let pool = CommandPool::new(&settings, self.image.device_provider()).unwrap();
        let mut settings = commandbuffer::SettingsProvider::default(); settings.batch_size = 1;
        let cmd_set = CommandBufferSet::new(&settings, self.image.device_provider(), &pool);
        let cmd = cmd_set.next_cmd();
        unsafe{
            let device = self.image.device_provider().device();
            device.begin_command_buffer(*cmd, &vk::CommandBufferBeginInfo::default()).unwrap();
            self.copy_to_buffer(&cmd, dst, buffer_addressing)?;
            device.end_command_buffer(*cmd).unwrap();
        }
        let mut submit = SubmitSet::new();
        submit.add_cmd(cmd);
        let submit = [submit];
        let queue = Queue::new(self.image.device_provider(), vk::QueueFlags::TRANSFER).unwrap();
        queue.wait_submit(&submit).expect("Could not execute transfer");
        Ok(())
    }

    fn copy_to_image<Img:ImageProvider, IR:ImageSubresourceProvider + UsesImageProvider<Img>>(&self, cmd: &Arc<vk::CommandBuffer>, dst: &Arc<IR>) -> Result<(), ImageResourceMemOpError> {
        let src_layout = self.layout();
        let dst_layout = dst.layout();
        let op = [vk::ImageCopy::builder()
        .src_subresource(self.resorces)
        .dst_subresource(dst.subresource())
        .src_offset(self.offset)
        .dst_offset(dst.offset())
        .extent(self.extent)
        .build()];

        let src_image = self.image.image();
        let dst_image = dst.image_provider().image();
        debug!("Copying layer {:?} for image {:?} to image {:?}", self.resorces, *src_image, *dst_image);

        unsafe{
            let device = self.image.device_provider().device();
            device.cmd_copy_image(**cmd, *src_image, *src_layout, *dst_image, *dst_layout, &op);
        }
        Ok(())
    }

    fn copy_to_image_internal<Img:ImageProvider, IR:ImageSubresourceProvider + UsesImageProvider<Img>>(&self, dst: &Arc<IR>) -> Result<(), ImageResourceMemOpError> {
        let settings = commandpool::SettingsProvider::new(self.image.device_provider().transfer_queue().unwrap().1);
        let pool = CommandPool::new(&settings, self.image.device_provider()).unwrap();
        let mut settings = commandbuffer::SettingsProvider::default(); settings.batch_size = 1;
        let cmd_set = CommandBufferSet::new(&settings, self.image.device_provider(), &pool);
        let cmd = cmd_set.next_cmd();
        unsafe{
            let device = self.image.device_provider().device();
            device.begin_command_buffer(*cmd, &vk::CommandBufferBeginInfo::default()).unwrap();
            self.copy_to_image(&cmd, dst)?;
            device.end_command_buffer(*cmd).unwrap();
        }
        let mut submit = SubmitSet::new();
        submit.add_cmd(cmd);
        let submit = [submit];
        let queue = Queue::new(self.image.device_provider(), vk::QueueFlags::TRANSFER).unwrap();
        queue.wait_submit(&submit).expect("Could not execute transfer");
        Ok(())
    }

}

impl<D:DeviceProvider, I:ImageProvider + UsesDeviceProvider<D>> UsesImageProvider<I> for ImageResource<D,I>{
    fn image_provider(&self) -> &Arc<I> {
        &self.image
    }
}
