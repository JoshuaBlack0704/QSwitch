use std::{sync::{Arc, Mutex}, collections::VecDeque, marker::PhantomData};

use ash::vk;

use crate::{device::{DeviceProvider, UsesDeviceProvider}, instance::{InstanceProvider, UsesInstanceProvider}};

use self::{memory::{MemoryProvider, UsesMemoryProvider}, partitionsystem::PartitionProvider, buffer::BufferProvider};


pub mod partitionsystem;
#[derive(Clone)]
pub struct Partition{
    pub tracker: Arc<bool>,
    pub offset: u64,
    pub size: u64,
}
pub struct PartitionSystem{
    partitions: VecDeque<Partition>,    
}

pub mod memory;
pub struct Memory<D: DeviceProvider, P: PartitionProvider>{
    device: Arc<D>,
    partition_sys: Mutex<P>,
    memory: vk::DeviceMemory,
}

pub mod buffer;
pub struct Buffer<D: DeviceProvider, M: MemoryProvider, P: PartitionProvider>{
    device: Arc<D>,
    memory: Arc<M>,
    memory_partition: Partition,
    partition_sys: Mutex<P>,
    buffer: vk::Buffer,
    alignment_type: buffer::BufferAlignmentType,
}

pub mod bufferpartition;
pub struct BufferPartition<I:InstanceProvider, D: DeviceProvider + UsesInstanceProvider<I>, M:MemoryProvider, B: BufferProvider + UsesMemoryProvider<M> + UsesDeviceProvider<D>, P:PartitionProvider>{
    buffer: Arc<B>,
    _partition_sys: Mutex<P>,
    partition: Partition,
    _device_addr: Option<vk::DeviceAddress>,
    _instance: PhantomData<I>,
    _memory: PhantomData<M>,
    _device: PhantomData<D>
}
