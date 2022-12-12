use std::{sync::Arc, collections::VecDeque};
use tokio::sync::RwLock;

use ash::vk;

use crate::device::{DeviceProvider, self};

use self::{memory::{MemorySettingsProvider, MemoryProvider}, partitionsystem::PartitionProvider, bufferarea::{BufferSettingsProvider, BufferProvider}};


pub mod partitionsystem;
#[derive(Clone)]
pub struct Partition{
    tracker: Arc<bool>,
    offset: u64,
    size: u64,
}
pub struct PartitionSystem{
    partitions: VecDeque<Partition>,    
}

pub mod memory;
pub struct Memory<D: DeviceProvider, P: PartitionProvider>{
    device: Arc<D>,
    partition_sys: RwLock<P>,
    memory: vk::DeviceMemory,
}

pub mod bufferarea;
pub struct Buffer<D: DeviceProvider, M: MemoryProvider, P: PartitionProvider>{
    device: Arc<D>,
    memory: Arc<M>,
    memory_partition: Partition,
    partition_sys: RwLock<P>,
    buffer: vk::Buffer,
}

pub mod bufferpartition;
pub struct BufferPartition<D: device::DeviceProvider, B: BufferProvider>{
    device: Arc<D>,
    buffer: Arc<B>,
}
