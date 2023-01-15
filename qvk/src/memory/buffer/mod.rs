use ash::vk;

use crate::{
    command::{BufferCopyFactory, ImageCopyFactory},
    init::DeviceSource,
};

use super::{
    allocators::{BufPart, BufferSource, MemPart},
    image::ImageSource,
};

pub mod buffersegment;
pub trait BufferSegmentFactory {
    type Segment: BufferSegmentSource;
    fn get_segment(&self, size: u64, alignment: Option<u64>) -> Self::Segment;
}
#[derive(Clone, Debug)]
pub enum BufferSegmentMemOpError {
    NoSpace,
    VulkanError(vk::Result),
}
pub trait BufferSegmentSource {
    fn size(&self) -> u64;
    fn offset(&self) -> u64;
    fn copy_from_ram<T>(&self, src: &[T]) -> Result<(), BufferSegmentMemOpError>;
    fn copy_to_ram<T>(&self, dst: &mut [T]) -> Result<(), BufferSegmentMemOpError>;
    fn copy_to_segment_internal<BP: BufferCopyFactory + BufferSegmentSource>(
        &self,
        dst: &BP,
    ) -> Result<(), BufferSegmentMemOpError>;
    ///Addressing is (bufferRowLength, bufferImageHeight)
    fn copy_to_image_internal<I: ImageSource + ImageCopyFactory>(
        &self,
        dst: &I,
        buffer_addressing: Option<(u32, u32)>,
    ) -> Result<(), vk::Result>;
}
pub struct BufferSegment<B: DeviceSource + BufferSource> {
    buffer: B,
    desc_buffer_info: [vk::DescriptorBufferInfo; 1],
    mem_part: MemPart,
    buf_part: BufPart,
}
