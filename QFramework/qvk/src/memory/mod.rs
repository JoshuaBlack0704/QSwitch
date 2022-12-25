use std::{collections::VecDeque, sync::{Arc, Mutex}, ffi::c_void};

use ash::vk;

use crate::init::DeviceSource;
use crate::memory::partitionsystem::PartitionError;

pub mod partitionsystem;
/// (start_addr, size, tracker)
pub trait PartitionSource{
    /// The alignment fn takes and offset and returns if the offset is aligned
    fn partition<F:Fn(u64) -> bool>(&mut self, size: u64, alignment_fn: F) -> Result<Partition, PartitionError>;
}

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
pub trait MemoryFactory<M:MemorySource>{
    fn create_memory(&self, size: u64, type_index: u32, extensions: Option<*const c_void>) -> Result<M, vk::Result>;
}
pub trait MemorySource{
    fn partition(&self, size: u64, alignment: Option<u64>) -> Result<Partition, partitionsystem::PartitionError>;
    fn memory(&self) -> &vk::DeviceMemory;
}
pub trait MemorySupplier<M:MemorySource>{
    fn memory_source(&self) -> &M;
}
pub struct Memory<D: DeviceSource, P: PartitionSource>{
    device: D,
    partition_sys: Mutex<P>,
    memory: vk::DeviceMemory,
}

pub mod buffer;

