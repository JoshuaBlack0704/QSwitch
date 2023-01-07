use std::{sync::{Arc, Mutex}, collections::VecDeque};

use ash::vk;

use crate::init::DeviceSource;

pub mod partitionsystem;
#[derive(Clone)]
pub struct Partition {
    pub tracker: Arc<bool>,
    pub offset: u64,
    pub size: u64,
}
pub struct PartitionSystem {
    partitions: VecDeque<Partition>,
}

#[derive(Clone)]
pub enum MemoryExtensions{
    
}
#[derive(Clone)]
pub enum BufferExtensions{
    
}
#[derive(Clone)]
pub enum ImageExtensions{
    
}

type MemAlloc = (vk::DeviceMemory, PartitionSystem);
type BufAlloc = (vk::Buffer, PartitionSystem);

pub struct MemoryAllocator<D:DeviceSource>{
    device: D,
    min_size: u64,
    type_index: u32,
    extensions: Vec<MemoryExtensions>,
    allocations: Mutex<Vec<MemAlloc>>,
}
pub struct BufferAllocator<D:DeviceSource>{
    device: D,
    min_size: u64,
    usage: vk::BufferUsageFlags,
    flags: Option<vk::BufferCreateFlags>,
    extensions: Vec<BufferExtensions>,
    share: Option<Vec<u32>>,
    mem: Arc<MemoryAllocator<D>>,
    buffers: Mutex<Vec<BufAlloc>>,
}
pub struct ImageAllocator<D:DeviceSource>{
    device: D,
    format: vk::Format,
    levels: u32,
    layers: u32,
    usage: vk::ImageUsageFlags,
    img_type: vk::ImageType,
    samples: vk::SampleCountFlags,
    tiling: vk::ImageTiling,
    share: Option<Vec<u32>>,
    flags: Option<vk::ImageCreateFlags>,
    extensions: Vec<ImageExtensions>,
    mem: Arc<MemoryAllocator<D>>,

}
