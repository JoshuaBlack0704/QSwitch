use std::{marker::PhantomData, sync::{Arc, Mutex}};

use ash::vk;
use std::sync::MutexGuard;

use crate::{memory::Partition, command::{ImageCopyFactory, BufferCopyFactory}};
use crate::command::CommandBufferStore;
use crate::image::imageresource::ImageResourceMemOpError;
use crate::init::{DeviceSource, InstanceSource, DeviceSupplier, InstanceSupplier};
use crate::memory::buffer::{BufferStore, InternalBufferStore};
use crate::memory::MemoryStore;

pub mod image;
pub trait ImageStore{
    /// Returns the old layout
    fn transition<C:CommandBufferStore>(
        &self,
        cmd: &C,
        new_layout: vk::ImageLayout,
        src_stage: Option<vk::PipelineStageFlags2>,
        dst_stage: Option<vk::PipelineStageFlags2>,
        src_access: Option<vk::AccessFlags2>,
        dst_access: Option<vk::AccessFlags2>,
        subresources: Option<vk::ImageSubresourceRange>,
    );
    /// Creates and uses an internal command pool and buffer
    fn internal_transistion(&self, new_layout: vk::ImageLayout, subresources: Option<vk::ImageSubresourceRange>);
    fn image(&self) -> &vk::Image;
    fn layout(&self) -> Arc<Mutex<vk::ImageLayout>>;
    fn mip_levels(&self) -> u32;
    fn array_layers(&self) -> u32;
    fn extent(&self) -> vk::Extent3D;
}
pub trait InternalImageStore<I:ImageStore>{
    fn image_provider(&self) -> &I;
}
pub struct Image<D:DeviceSource, M:MemoryStore>{
    device: D,
    memory: Option<M>,
    _partition: Option<Partition>,
    image: vk::Image,
    create_info: vk::ImageCreateInfo,
    current_layout: Arc<Mutex<vk::ImageLayout>>,
}


pub mod imageresource;
pub trait ImageSubresourceStore{
    fn subresource(&self) -> vk::ImageSubresourceLayers;
    fn offset(&self) -> vk::Offset3D;
    fn extent(&self) -> vk::Extent3D;
    fn layout(&self) -> MutexGuard<vk::ImageLayout>;
    fn copy_to_buffer_internal<B:BufferStore, BP:BufferCopyFactory + InternalBufferStore<B>>(&self, dst: &BP, buffer_addressing: Option<(u32,u32)>) -> Result<(), ImageResourceMemOpError>;
    fn copy_to_image_internal<I:ImageStore, IR:ImageCopyFactory+ InternalImageStore<I>>(&self, dst: &IR) -> Result<(), ImageResourceMemOpError>;
    fn blit_to_image_internal<I:ImageStore, IR:ImageCopyFactory + InternalImageStore<I>>(&self, dst: &IR, scale_filter: vk::Filter) -> Result<(), ImageResourceMemOpError>;
}

pub struct ImageResource<I:InstanceSource, D:DeviceSource + InstanceSupplier<I>, Img:ImageStore + DeviceSupplier<D>>{
    image: Img,
    resorces: vk::ImageSubresourceLayers,
    offset: vk::Offset3D,
    extent: vk::Extent3D,
    layout: Arc<Mutex<vk::ImageLayout>>,
    _device: PhantomData<D>,
    _instance: PhantomData<I>
}

pub mod imageview;
pub trait ImageViewStore{

}
pub struct ImageView<D:DeviceSource, Img:ImageStore>{
    _device: D,
    _image: Img,
    _view: vk::ImageView,
}








