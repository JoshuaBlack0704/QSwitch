use std::{sync::{Arc, Mutex}, collections::VecDeque};

use ash::vk;

use crate::device::DeviceProvider;

use self::partitionsystem::PartitionProvider;



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
