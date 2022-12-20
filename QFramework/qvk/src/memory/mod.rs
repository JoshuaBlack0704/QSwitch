use std::{collections::VecDeque, sync::{Arc, Mutex}};

use ash::vk;

use crate::init::DeviceStore;
use crate::memory::partitionsystem::PartitionError;

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
pub struct Memory<D: DeviceStore, P: PartitionStore>{
    device: Arc<D>,
    partition_sys: Mutex<P>,
    memory: vk::DeviceMemory,
}

pub mod buffer;

/// (start_addr, size, tracker)
pub trait PartitionStore{
    /// The alignment fn takes and offset and returns if the offset is aligned
    fn partition<F:Fn(u64) -> bool>(&mut self, size: u64, alignment_fn: F) -> Result<Partition, PartitionError>;
}

pub trait MemoryStore{
    fn partition(&self, size: u64, alignment: Option<u64>) -> Result<Partition, partitionsystem::PartitionError>;
    fn memory(&self) -> &vk::DeviceMemory;
}

pub trait InternalMemoryStore<M:MemoryStore>{
    fn memory_provider(&self) -> &Arc<M>;
}
