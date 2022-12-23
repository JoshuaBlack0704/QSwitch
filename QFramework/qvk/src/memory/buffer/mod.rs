use std::{marker::PhantomData, sync::Mutex};

use ash::vk;
use crate::command::{ImageCopyFactory, BufferCopyFactory};
use crate::image::{ImageStore, InternalImageStore};

use crate::init::{DeviceStore, InstanceSupplier, InstanceSource, InternalDeviceStore};
use crate::memory::{InternalMemoryStore, MemoryStore, PartitionStore};
use crate::memory::buffer::buffer::BufferAlignmentType;
use crate::memory::buffer::buffersegment::BufferSegmentMemOpError;
use crate::memory::partitionsystem::PartitionError;

use super::Partition;

pub mod buffer;
pub trait BufferStore{
    fn buffer(&self) -> &vk::Buffer;
    ///Gets the Allocation partition this buffer is stored at
    fn home_partition(&self) -> &Partition;
    ///Partitions this buffer
    fn partition(&self, size: u64, alignment_type: BufferAlignmentType) -> Result<Partition, PartitionError> ;
    fn usage(&self) -> vk::BufferUsageFlags;
}

pub trait InternalBufferStore<B:BufferStore>{
    fn buffer_provider(&self) -> &B;
}
pub struct Buffer<D: DeviceStore, M: MemoryStore, P: PartitionStore>{

    device: D,
    memory: M,
    memory_partition: Partition,
    partition_sys: Mutex<P>,
    buffer: vk::Buffer,
    info: vk::BufferCreateInfo,
    alignment_type: buffer::BufferAlignmentType,
}

pub mod buffersegment;
pub trait BufferSegmentStore{
    fn get_partition(&self) -> &Partition;
    fn device_addr(&self) -> vk::DeviceSize;
    fn copy_from_ram<T>(&self, src: &[T]) -> Result<(), BufferSegmentMemOpError>;
    fn copy_to_ram<T>(&self, dst: &mut [T]) -> Result<(), BufferSegmentMemOpError>;
    fn copy_to_segment_internal<B:BufferStore, BP:BufferCopyFactory + InternalBufferStore<B>>(&self, dst: &BP) -> Result<(), BufferSegmentMemOpError>;
    ///Addressing is (bufferRowLength, bufferImageHeight)
    fn copy_to_image_internal<I:ImageStore, IS: InternalImageStore<I> + ImageCopyFactory>(&self,dst: &IS, buffer_addressing: Option<(u32, u32)>) -> Result<(), vk::Result>;
}
pub struct BufferSegment<I:InstanceSource, D: DeviceStore + InstanceSupplier<I>, M:MemoryStore, B: BufferStore + InternalMemoryStore<M> + InternalDeviceStore<D>, P:PartitionStore>{

    buffer: B,
    _partition_sys: Mutex<P>,
    partition: Partition,
    desc_buffer_info: [vk::DescriptorBufferInfo;1],
    _device_addr: Option<vk::DeviceAddress>,
    _instance: PhantomData<I>,
    _memory: PhantomData<M>,
    _device: PhantomData<D>
}




