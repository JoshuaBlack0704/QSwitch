use std::{marker::PhantomData, sync::{Arc, Mutex}};

use ash::vk;
use std::sync::MutexGuard;

use crate::{init::{instance::{InstanceStore, InternalInstanceStore}}, memory::Partition};
use crate::command::CommandBufferStore;
use crate::image::imageresource::ImageResourceMemOpError;
use crate::init::{DeviceStore, InternalDeviceStore};
use crate::memory::buffer::BufferSegmentStore;
use crate::memory::buffer::{BufferStore, InternalBufferStore};
use crate::memory::MemoryStore;

pub mod image;
pub trait ImageStore{
    /// Returns the old layout
    fn transition<C:CommandBufferStore>(
        &self,
        cmd: &Arc<C>,
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
    fn image_provider(&self) -> &Arc<I>;
}
pub struct Image<D:DeviceStore, M:MemoryStore>{
    device: Arc<D>,
    memory: Option<Arc<M>>,
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
    fn copy_to_buffer<B:BufferStore, BP:BufferSegmentStore + InternalBufferStore<B>,C:CommandBufferStore>(&self, cmd: &Arc<C>, dst: &Arc<BP>, buffer_addressing: Option<(u32,u32)>) -> Result<(), ImageResourceMemOpError>;
    fn copy_to_buffer_internal<B:BufferStore, BP:BufferSegmentStore + InternalBufferStore<B>>(&self, dst: &Arc<BP>, buffer_addressing: Option<(u32,u32)>) -> Result<(), ImageResourceMemOpError>;
    fn copy_to_image<I:ImageStore, IR:ImageSubresourceStore + InternalImageStore<I>, C:CommandBufferStore>(&self, cmd: &Arc<C>, dst: &Arc<IR>) -> Result<(), ImageResourceMemOpError>;
    fn copy_to_image_internal<I:ImageStore, IR:ImageSubresourceStore + InternalImageStore<I>>(&self, dst: &Arc<IR>) -> Result<(), ImageResourceMemOpError>;
    fn blit_to_image<I:ImageStore, IR:ImageSubresourceStore + InternalImageStore<I>,C:CommandBufferStore>(&self, cmd: &Arc<C>, dst: &Arc<IR>, scale_filter: vk::Filter) -> Result<(), ImageResourceMemOpError>;
    fn blit_to_image_internal<I:ImageStore, IR:ImageSubresourceStore + InternalImageStore<I>>(&self, dst: &Arc<IR>, scale_filter: vk::Filter) -> Result<(), ImageResourceMemOpError>;
}
pub struct ImageResource<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, Img:ImageStore + InternalDeviceStore<D>>{
    image: Arc<Img>,
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
pub struct ImageView<D:DeviceStore, Img:ImageStore>{
    _device: Arc<D>,
    _image: Arc<Img>,
    _view: vk::ImageView,
}








