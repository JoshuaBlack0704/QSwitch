use std::ffi::c_void;
use std::{marker::PhantomData, sync::Mutex};

use ash::vk;
use crate::command::{ImageCopyFactory, BufferCopyFactory};
use crate::image::{ImageSource, ImageSupplier};

use crate::init::{DeviceSource, InstanceSupplier, InstanceSource, DeviceSupplier};
use crate::memory::{MemorySupplier, MemorySource, PartitionSource};
use crate::memory::buffer::buffer::BufferAlignmentType;
use crate::memory::buffer::buffersegment::BufferSegmentMemOpError;
use crate::memory::partitionsystem::PartitionError;

use self::buffer::BufferCreateError;

use super::Partition;

pub mod buffer;
pub trait BufferFactory<B:BufferSource>{
    fn share(&self) -> Option<Vec<u32>> {None}
    fn create_buffer(&self, size: u64, usage: vk::BufferUsageFlags, flags: Option<vk::BufferCreateFlags>, extensions: Option<*const c_void>) -> Result<B, BufferCreateError>;
}
pub trait BufferSource{
    fn buffer(&self) -> &vk::Buffer;
    ///Gets the Allocation partition this buffer is stored at
    fn home_partition(&self) -> &Partition;
    ///Partitions this buffer
    fn partition(&self, size: u64, alignment_type: BufferAlignmentType) -> Result<Partition, PartitionError> ;
    fn usage(&self) -> vk::BufferUsageFlags;
}

pub trait BufferSupplier<B:BufferSource>{
    fn buffer_provider(&self) -> &B;
}
pub struct Buffer<D: DeviceSource, M: MemorySource, P: PartitionSource>{

    device: D,
    memory: M,
    memory_partition: Partition,
    partition_sys: Mutex<P>,
    buffer: vk::Buffer,
    info: vk::BufferCreateInfo,
    alignment_type: buffer::BufferAlignmentType,
}

pub mod buffersegment;
pub trait BufferSegmentFactory<BSeg: BufferSegmentSource>{
    fn create_segment(&self, size: u64, alignment: Option<u64>) -> Result<BSeg, PartitionError>;
}
pub trait BufferSegmentSource{
    fn get_partition(&self) -> &Partition;
    fn device_addr(&self) -> vk::DeviceSize;
    fn copy_from_ram<T>(&self, src: &[T]) -> Result<(), BufferSegmentMemOpError>;
    fn copy_to_ram<T>(&self, dst: &mut [T]) -> Result<(), BufferSegmentMemOpError>;
    fn copy_to_segment_internal<B:BufferSource, BP:BufferCopyFactory + BufferSupplier<B>>(&self, dst: &BP) -> Result<(), BufferSegmentMemOpError>;
    ///Addressing is (bufferRowLength, bufferImageHeight)
    fn copy_to_image_internal<I:ImageSource, IS: ImageSupplier<I> + ImageCopyFactory>(&self,dst: &IS, buffer_addressing: Option<(u32, u32)>) -> Result<(), vk::Result>;
}
pub struct BufferSegment<I:InstanceSource, D: DeviceSource + InstanceSupplier<I>, M:MemorySource, B: BufferSource + MemorySupplier<M> + DeviceSupplier<D>>{

    buffer: B,
    partition: Partition,
    desc_buffer_info: [vk::DescriptorBufferInfo;1],
    _device_addr: Option<vk::DeviceAddress>,
    _instance: PhantomData<I>,
    _memory: PhantomData<M>,
    _device: PhantomData<D>
}




