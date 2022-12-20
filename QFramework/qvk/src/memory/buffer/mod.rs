use std::{sync::{Arc, Mutex}, marker::PhantomData};

use ash::vk;

use crate::init::{device::{DeviceStore, InternalDeviceStore}, instance::{InstanceStore, InternalInstanceStore}};

use self::buffer::BufferStore;

use super::{memory::{MemoryStore, InternalMemoryStore}, partitionsystem::PartitionStore, Partition};

pub mod buffer;
pub struct Buffer<D: DeviceStore, M: MemoryStore, P: PartitionStore>{

    device: Arc<D>,
    memory: Arc<M>,
    memory_partition: Partition,
    partition_sys: Mutex<P>,
    buffer: vk::Buffer,
    info: vk::BufferCreateInfo,
    alignment_type: buffer::BufferAlignmentType,
}

pub mod buffersegment;
pub struct BufferSegment<I:InstanceStore, D: DeviceStore + InternalInstanceStore<I>, M:MemoryStore, B: BufferStore + InternalMemoryStore<M> + InternalDeviceStore<D>, P:PartitionStore>{

    buffer: Arc<B>,
    _partition_sys: Mutex<P>,
    partition: Partition,
    _device_addr: Option<vk::DeviceAddress>,
    _instance: PhantomData<I>,
    _memory: PhantomData<M>,
    _device: PhantomData<D>
}
