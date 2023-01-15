use std::sync::{Arc, Mutex};

use ash::vk;
use std::sync::MutexGuard;

use crate::command::{BufferCopyFactory, ImageCopyFactory};
use crate::init::{DeviceSource, InstanceSource};

use self::imageresource::{ImageResourceCreateError, ImageResourceMemOpError};

use super::allocators::MemPart;

pub mod image;
pub trait ImageFactory {
    type Image: ImageSource;
    fn create_image(&self, extent: vk::Extent3D) -> Self::Image;
}
pub trait ImageSource {
    fn internal_transistion(&self, new_layout: vk::ImageLayout);
    fn image(&self) -> &vk::Image;
    fn layout(&self) -> Arc<Mutex<vk::ImageLayout>>;
    fn mip_levels(&self) -> u32;
    fn array_layers(&self) -> u32;
    fn extent(&self) -> vk::Extent3D;
}

pub struct Image<D: DeviceSource> {
    device: D,
    _mem_part: MemPart,
    image: vk::Image,
    create_info: vk::ImageCreateInfo,
    current_layout: Arc<Mutex<vk::ImageLayout>>,
}

pub mod imageresource;
pub trait ImageResourceFactory<IR: ImageResourceSource> {
    fn create_resource(
        &self,
        offset: vk::Offset3D,
        extent: vk::Extent3D,
        level: u32,
        aspect: vk::ImageAspectFlags,
    ) -> Result<IR, ImageResourceCreateError>;
}
pub trait ImageResourceSource {
    fn subresource(&self) -> vk::ImageSubresourceLayers;
    fn offset(&self) -> vk::Offset3D;
    fn extent(&self) -> vk::Extent3D;
    fn layout(&self) -> MutexGuard<vk::ImageLayout>;
    fn copy_to_buffer_internal<BP: BufferCopyFactory>(
        &self,
        dst: &BP,
        buffer_addressing: Option<(u32, u32)>,
    ) -> Result<(), ImageResourceMemOpError>;
    fn copy_to_image_internal<IR: ImageCopyFactory + DeviceSource>(
        &self,
        dst: &IR,
    ) -> Result<(), ImageResourceMemOpError>;
    fn blit_to_image_internal<IR: ImageCopyFactory + DeviceSource>(
        &self,
        dst: &IR,
        scale_filter: vk::Filter,
    ) -> Result<(), ImageResourceMemOpError>;
    fn aspect(&self) -> vk::ImageAspectFlags;
    fn level(&self) -> u32;
}

pub struct ImageResource<Img: ImageSource + DeviceSource + InstanceSource> {
    image: Img,
    resorces: vk::ImageSubresourceLayers,
    offset: vk::Offset3D,
    extent: vk::Extent3D,
    layout: Arc<Mutex<vk::ImageLayout>>,
    _aspect: vk::ImageAspectFlags,
}

pub mod imageview;
pub trait ImageViewFactory<Iv: ImageViewSource> {
    fn create_image_view(
        &self,
        format: vk::Format,
        view_type: vk::ImageViewType,
        swizzle: Option<vk::ComponentMapping>,
        flags: Option<vk::ImageViewCreateFlags>,
    ) -> Iv;
}
pub trait ImageViewSource {
    fn format(&self) -> vk::Format;
    fn view(&self) -> vk::ImageView;
}
pub struct ImageView<IR: ImageResourceSource + ImageSource + DeviceSource> {
    _image_resource: IR,
    view: vk::ImageView,
    format: vk::Format,
}
