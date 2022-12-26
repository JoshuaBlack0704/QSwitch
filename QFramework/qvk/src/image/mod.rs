use std::{marker::PhantomData, sync::{Arc, Mutex}, ffi::c_void};

use ash::vk;
use std::sync::MutexGuard;

use crate::{memory::Partition, command::{ImageCopyFactory, BufferCopyFactory}};
use crate::command::CommandBufferSource;
use crate::image::imageresource::ImageResourceMemOpError;
use crate::init::{DeviceSource, InstanceSource, DeviceSupplier};
use crate::memory::buffer::{BufferSource, BufferSupplier};
use crate::memory::MemorySource;

use self::{image::ImageCreateError, imageresource::ImageResourceCreateError};

pub mod image;
pub trait ImageFactory<Img:ImageSource>{
    fn image_type(&self) -> vk::ImageType {vk::ImageType::TYPE_2D}
    fn samples(&self) -> vk::SampleCountFlags {vk::SampleCountFlags::TYPE_1}
    fn tiling(&self) -> vk::ImageTiling {vk::ImageTiling::OPTIMAL}
    fn share(&self) -> Option<Vec<u32>> {None}
    fn create_flags(&self) -> Option<vk::ImageCreateFlags> {None}
    fn create_image(&self, format: vk::Format, extent: vk::Extent3D, levels: u32, layers: u32, usage: vk::ImageUsageFlags, extensions: Option<*const c_void>) -> Result<Img, ImageCreateError>;
}
pub trait ImageSource{
    /// Returns the old layout
    fn transition<C:CommandBufferSource>(
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
pub trait ImageSupplier<I:ImageSource>{
    fn image_provider(&self) -> &I;
}
pub struct Image<D:DeviceSource, M:MemorySource>{
    device: D,
    memory: Option<M>,
    _partition: Option<Partition>,
    image: vk::Image,
    create_info: vk::ImageCreateInfo,
    current_layout: Arc<Mutex<vk::ImageLayout>>,
}


pub mod imageresource;
pub trait ImageResourceFactory<IR:ImageResourceSource>{
    fn create_resource(&self, offset: vk::Offset3D, extent: vk::Extent3D, level: u32, aspect: vk::ImageAspectFlags) -> Result<IR, ImageResourceCreateError>;
}
pub trait ImageResourceSource{
    fn subresource(&self) -> vk::ImageSubresourceLayers;
    fn offset(&self) -> vk::Offset3D;
    fn extent(&self) -> vk::Extent3D;
    fn layout(&self) -> MutexGuard<vk::ImageLayout>;
    fn copy_to_buffer_internal<B:BufferSource, BP:BufferCopyFactory + BufferSupplier<B>>(&self, dst: &BP, buffer_addressing: Option<(u32,u32)>) -> Result<(), ImageResourceMemOpError>;
    fn copy_to_image_internal<I:ImageSource, IR:ImageCopyFactory+ ImageSupplier<I>>(&self, dst: &IR) -> Result<(), ImageResourceMemOpError>;
    fn blit_to_image_internal<I:ImageSource, IR:ImageCopyFactory + ImageSupplier<I>>(&self, dst: &IR, scale_filter: vk::Filter) -> Result<(), ImageResourceMemOpError>;
    fn aspect(&self) -> vk::ImageAspectFlags;
    fn level(&self) -> u32;
}
pub trait ImageResourceSupplier<IR:ImageResourceSource>{
    fn image_resource(&self) -> &IR;
}

pub struct ImageResource<D:DeviceSource + InstanceSource, Img:ImageSource + DeviceSupplier<D>>{
    image: Img,
    resorces: vk::ImageSubresourceLayers,
    offset: vk::Offset3D,
    extent: vk::Extent3D,
    layout: Arc<Mutex<vk::ImageLayout>>,
    _aspect: vk::ImageAspectFlags,
    _device: PhantomData<D>,
}

pub mod imageview;
pub trait ImageViewFactory<Iv:ImageViewSource>{
    fn create_image_view(&self, format: vk::Format, view_type: vk::ImageViewType, swizzle: Option<vk::ComponentMapping>, flags: Option<vk::ImageViewCreateFlags>) -> Iv;
}
pub trait ImageViewSource{

}
pub struct ImageView<D:DeviceSource, Img:ImageSource, IR:ImageResourceSource>{
    _device: D,
    _image_resource: IR,
    _image: PhantomData<Img>,
    _view: vk::ImageView,
}








