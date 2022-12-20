use std::sync::{Arc, MutexGuard};

use image::{self, EncodableLayout};

use ash::vk;
use log::debug;

use crate::{command::{CommandBufferStore, commandpool, CommandPool, commandset, CommandSet, ImageCopyFactory, BufferCopyFactory}, init::{instance::{InstanceStore, InternalInstanceStore}}, memory::{buffer::{buffer::{self}, Buffer, BufferSegment}, Memory, memory}, queue::{Queue, SubmitSet, QueueOps}};
use crate::command::CommandBufferFactory;
use crate::image::{ImageStore, ImageSubresourceStore, InternalImageStore};
use crate::init::{DeviceStore, InternalDeviceStore};
use crate::memory::buffer::{BufferSegmentStore, BufferStore, InternalBufferStore};

use super::ImageResource;

#[derive(Clone, Debug)]
pub enum ImageResourceCreateError{
    ResorcesDontExist,
}

#[derive(Debug)]
pub enum ImageResourceMemOpError{
    
}

impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, Img:ImageStore + InternalDeviceStore<D>> ImageResource<I,D,Img>{
    pub fn new(image_provider: &Arc<Img>, aspect: vk::ImageAspectFlags, miplevel: u32, array_layer: u32, layer_count: u32, offset: vk::Offset3D, extent: vk::Extent3D) -> Result<Arc<Self>, ImageResourceCreateError>{
        
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
                    _instance: std::marker::PhantomData,
                }
            )
        )

    }

    pub fn load_image(tgt: &Arc<Self>, file: &String){
        let reader = image::io::Reader::open(file).unwrap();
        let data = reader.decode().unwrap();
        let image = data.to_rgba8();
        let bytes = image.as_bytes();
        let image_extent = vk::Extent3D::builder().width(image.width()).height(image.height()).depth(1).build();

        let settings = memory::SettingsStore::new(bytes.len() as u64 * 2, tgt.image.device_provider().device_memory_index());
        let dev_mem = Memory::new(&settings, tgt.image.device_provider()).expect("Could not allocate memory");
        let image_settings = crate::image::image::SettingsStore::new_simple(vk::Format::R8G8B8A8_SRGB, image_extent, vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, Some(vk::ImageLayout::TRANSFER_DST_OPTIMAL));    
        let image = crate::image::Image::new(tgt.image.device_provider(), &dev_mem, &image_settings).unwrap();
        let resource = crate::image::ImageResource::new(&image, vk::ImageAspectFlags::COLOR, 0, 0, 1, vk::Offset3D::default(), ImageStore::extent(&*image)).unwrap();
        
        let settings = memory::SettingsStore::new(bytes.len() as u64 * 2, tgt.image.device_provider().host_memory_index());
        let host_mem = Memory::new(&settings, tgt.image.device_provider()).unwrap();
        let settings = buffer::SettingsStore::new(bytes.len() as u64 * 2, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST);
        let buf = Buffer::new(&settings, tgt.image.device_provider(), &host_mem).expect("Could not bind buffer");
        let part = BufferSegment::new(&buf, bytes.len() as u64, None).unwrap();
        part.copy_from_ram(&bytes).unwrap();
        part.copy_to_image_internal(&resource, None).unwrap();

        image.internal_transistion(vk::ImageLayout::TRANSFER_SRC_OPTIMAL, None);
        resource.blit_to_image_internal(tgt, vk::Filter::LINEAR).unwrap();
        

        
    }
}

impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, Img:ImageStore + InternalDeviceStore<D>> ImageSubresourceStore for ImageResource<I,D,Img>{
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

    fn copy_to_buffer_internal<B:BufferStore, BP:BufferCopyFactory + InternalBufferStore<B>>(&self, dst: &BP, buffer_addressing: Option<(u32,u32)>) -> Result<(), ImageResourceMemOpError> {
        let settings = commandpool::SettingsStore::new(self.image.device_provider().transfer_queue().unwrap().1);
        let pool = CommandPool::new(&settings, self.image.device_provider()).unwrap();
        let mut settings = commandset::SettingsStore::default(); settings.batch_size = 1;
        let cmd_set = CommandSet::new(&settings, self.image.device_provider(), &pool);
        let cmd = cmd_set.next_cmd();
        
        cmd.begin(None).unwrap();
        cmd.image_buffer_copy(self, dst, None);
        // self.copy_to_buffer(&cmd, dst, buffer_addressing)?;
        cmd.end().unwrap();
        
        let submit = SubmitSet::new(&cmd);
        let submit = [submit];
        let queue = Queue::new(self.image.device_provider(), vk::QueueFlags::TRANSFER).unwrap();
        queue.wait_submit(&submit).expect("Could not execute transfer");
        Ok(())
    }

    fn copy_to_image_internal<ImgExt:ImageStore, IR:ImageCopyFactory + InternalImageStore<ImgExt>>(&self, dst: &IR) -> Result<(), ImageResourceMemOpError> {
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
        let settings = commandpool::SettingsStore::new(self.image.device_provider().transfer_queue().unwrap().1);
        let pool = CommandPool::new(&settings, self.image.device_provider()).unwrap();
        let mut settings = commandset::SettingsStore::default(); settings.batch_size = 1;
        let cmd_set = CommandSet::new(&settings, self.image.device_provider(), &pool);
        let cmd = cmd_set.next_cmd();
        
        cmd.begin(None).unwrap();
        cmd.image_copy(self, dst);
        // self.copy_to_image(&cmd, dst)?;
        cmd.end().unwrap();
        
        let submit = SubmitSet::new(&cmd);
        let submit = [submit];
        let queue = Queue::new(self.image.device_provider(), vk::QueueFlags::TRANSFER).unwrap();
        queue.wait_submit(&submit).expect("Could not execute transfer");
        Ok(())
    }

    fn blit_to_image_internal<ImgExt:ImageStore, IR:ImageCopyFactory + InternalImageStore<ImgExt>>(&self, dst: &IR, scale_filter: vk::Filter) -> Result<(), ImageResourceMemOpError> {
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
        let settings = commandpool::SettingsStore::new(self.image.device_provider().grahics_queue().unwrap().1);
        let pool = CommandPool::new(&settings, self.image.device_provider()).unwrap();
        let mut settings = commandset::SettingsStore::default(); settings.batch_size = 1;
        let cmd_set = CommandSet::new(&settings, self.image.device_provider(), &pool);
        let cmd = cmd_set.next_cmd();
        
        cmd.begin(None).unwrap();
        cmd.image_blit(self, dst, scale_filter);
        // self.blit_to_image(&cmd, dst, scale_filter)?;
        cmd.end().unwrap();
        
        let submit = SubmitSet::new(&cmd);
        let submit = [submit];
        let queue = Queue::new(self.image.device_provider(), vk::QueueFlags::GRAPHICS).unwrap();
        queue.wait_submit(&submit).expect("Could not execute transfer");
        Ok(())
    }


}

impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, Img:ImageStore + InternalDeviceStore<D>> ImageCopyFactory for ImageResource<I,D,Img>{
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
}

impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, Img:ImageStore + InternalDeviceStore<D>> InternalImageStore<Img> for ImageResource<I,D,Img>{
    fn image_provider(&self) -> &Arc<Img> {
        &self.image
    }
}
impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, Img:ImageStore + InternalDeviceStore<D>> ImageCopyFactory for Arc<ImageResource<I,D,Img>>{
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
}
impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, Img:ImageStore + InternalDeviceStore<D>> InternalImageStore<Img> for Arc<ImageResource<I,D,Img>>{
    fn image_provider(&self) -> &Arc<Img> {
        &self.image
    }
}

