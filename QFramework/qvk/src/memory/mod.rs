use std::{sync::{Arc, Mutex}, collections::VecDeque};

use ash::vk;

use crate::device::{DeviceProvider, self};

use self::{memory::MemoryProvider, partitionsystem::PartitionProvider, buffer::BufferProvider};


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
    _memory: Arc<M>,
    memory_partition: Partition,
    partition_sys: Mutex<P>,
    buffer: vk::Buffer,
    alignment_type: buffer::BufferAlignmentType,
}

pub mod bufferpartition;
pub struct BufferPartition<D: device::DeviceProvider, B: BufferProvider, P:PartitionProvider>{
    device: Arc<D>,
    buffer: Arc<B>,
    _partition_sys: Mutex<P>,
    partition: Partition,
    _device_addr: Option<vk::DeviceAddress>,
}
