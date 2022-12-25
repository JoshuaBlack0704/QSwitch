use std::sync::{Arc, MutexGuard};

use image::{self, EncodableLayout};

use ash::vk;

use crate::{command::{CommandBufferSource, ImageCopyFactory, BufferCopyFactory, Executor}, memory::{buffer::{BufferFactory, BufferSegmentSource}, MemoryFactory},  image::ImageResource};
use crate::command::CommandBufferFactory;
use crate::image::{ImageSource, ImageResourceSource, ImageSupplier};
use crate::init::{DeviceSource, InstanceSource, DeviceSupplier, InstanceSupplier};
use crate::memory::buffer::{BufferSource, BufferSupplier, BufferSegmentFactory};

use super::ImageFactory;


#[derive(Clone, Debug)]
pub enum ImageResourceCreateError{
    ResorcesDontExist,
}

#[derive(Debug)]
pub enum ImageResourceMemOpError{
    
}

impl<I:InstanceSource + Clone, D:DeviceSource + InstanceSupplier<I> + Clone + DeviceSupplier<D>, Img:ImageSource + DeviceSupplier<D> + Clone> ImageResource<I,D,Img>{
    pub fn new(image_provider: &Img, aspect: vk::ImageAspectFlags, miplevel: u32, array_layer: u32, layer_count: u32, offset: vk::Offset3D, extent: vk::Extent3D) -> Result<Arc<Self>, ImageResourceCreateError>{
        
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

        let dev_mem = tgt.image.create_memory(bytes.len() as u64 * 2, tgt.image.device_provider().device_memory_index(), None).unwrap();
        let image = dev_mem.create_image(vk::Format::R8G8B8A8_SRGB, image_extent, 1, 1, vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, None).unwrap();
        image.internal_transistion(vk::ImageLayout::TRANSFER_DST_OPTIMAL, None);
        let resource = ImageResource::new(&image, vk::ImageAspectFlags::COLOR, 0, 0, 1, vk::Offset3D::default(), image.extent()).unwrap();
        
        let host_mem = tgt.image.create_memory(bytes.len() as u64 * 2, tgt.image.device_provider().host_memory_index(), None).unwrap();
        let buf = host_mem.create_buffer(bytes.len() as u64 * 2, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, None, None).unwrap();
        let seg = buf.create_segment(bytes.len() as u64, None).unwrap();
        seg.copy_from_ram(&bytes).unwrap();
        seg.copy_to_image_internal(&resource, None).unwrap();

        image.internal_transistion(vk::ImageLayout::TRANSFER_SRC_OPTIMAL, None);
        resource.blit_to_image_internal(tgt, vk::Filter::LINEAR).unwrap();
        

        
    }
}

impl<I:InstanceSource, D:DeviceSource + InstanceSupplier<I> + Clone + DeviceSupplier<D>, Img:ImageSource + DeviceSupplier<D>> ImageResourceSource for Arc<ImageResource<I,D,Img>>{
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

    fn copy_to_buffer_internal<B:BufferSource, BP:BufferCopyFactory + BufferSupplier<B>>(&self, dst: &BP, buffer_addressing: Option<(u32,u32)>) -> Result<(), ImageResourceMemOpError> {
        
        let exe = Executor::new(self.image.device_provider(), vk::QueueFlags::GRAPHICS);
        
        let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        cmd.image_buffer_copy(self, dst, buffer_addressing).unwrap();
        cmd.end().unwrap();
        
        exe.wait_submit_internal();
        Ok(())
    }

    fn copy_to_image_internal<ImgExt:ImageSource, IR:ImageCopyFactory + ImageSupplier<ImgExt>>(&self, dst: &IR) -> Result<(), ImageResourceMemOpError> {
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
        
        let exe = Executor::new(self.image.device_provider(), vk::QueueFlags::GRAPHICS);
        
        let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        cmd.image_copy(self, dst).unwrap();
        cmd.end().unwrap();
        
        exe.wait_submit_internal();
        Ok(())
    }

    fn blit_to_image_internal<ImgExt:ImageSource, IR:ImageCopyFactory + ImageSupplier<ImgExt>>(&self, dst: &IR, scale_filter: vk::Filter) -> Result<(), ImageResourceMemOpError> {
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

        let exe = Executor::new(self.image.device_provider(), vk::QueueFlags::GRAPHICS);
        
        let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        cmd.image_blit(self, dst, scale_filter).unwrap();
        cmd.end().unwrap();
        
        exe.wait_submit_internal();
        Ok(())
    }


}

impl<I:InstanceSource, D:DeviceSource + InstanceSupplier<I>, Img:ImageSource + DeviceSupplier<D>> ImageCopyFactory for Arc<ImageResource<I,D,Img>>{
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

impl<I:InstanceSource, D:DeviceSource + InstanceSupplier<I>, Img:ImageSource + DeviceSupplier<D>> ImageSupplier<Img> for Arc<ImageResource<I,D,Img>>{
    fn image_provider(&self) -> &Img {
        &self.image
    }
}

