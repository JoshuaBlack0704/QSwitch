use std::{sync::{Arc, Mutex}, marker::PhantomData};

use ash::vk;

use crate::{device::{DeviceStore, UsesDeviceStore}, instance::{InstanceStore, UsesInstanceStore}};

use self::buffer::BufferStore;

use super::{memory::{MemoryStore, UsesMemoryStore}, partitionsystem::PartitionStore, Partition};

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
pub struct BufferSegment<I:InstanceStore, D: DeviceStore + UsesInstanceStore<I>, M:MemoryStore, B: BufferStore + UsesMemoryStore<M> + UsesDeviceStore<D>, P:PartitionStore>{

    buffer: Arc<B>,
    _partition_sys: Mutex<P>,
    partition: Partition,
    _device_addr: Option<vk::DeviceAddress>,
    _instance: PhantomData<I>,
    _memory: PhantomData<M>,
    _device: PhantomData<D>
}
