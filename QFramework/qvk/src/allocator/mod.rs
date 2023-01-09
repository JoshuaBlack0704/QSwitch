use std::{sync::{Arc, Mutex}, collections::VecDeque};

use ash::vk;

use crate::init::DeviceSource;

use self::partitionsystem::PartitionError;

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
    Flags(vk::MemoryAllocateFlagsInfo)
}
#[derive(Clone)]
pub enum BufferExtensions{
    
}
#[derive(Clone)]
pub enum ImageExtensions{
    
}

type MemAlloc = (vk::DeviceMemory, Mutex<PartitionSystem>);
type MemPart = (vk::DeviceMemory, Partition);
type BufAlloc = ((vk::DeviceMemory, Partition), vk::Buffer, Mutex<PartitionSystem>);
type BufPart = (vk::Buffer, Partition);

fn test_partition(partition: &Mutex<PartitionSystem>, size:u64, alignment: Option<u64>) -> Result<Partition, PartitionError>{
    let mut lock = partition.lock().unwrap();
    lock.partition(size, |offset| {
            if let Some(a) = alignment{
                return offset % a == 0;
            }
            true
    })
}

pub mod memallocator;
pub trait MemoryALlocatorFactory{
    type Memory: MemorySource;
    fn create_memory(&self, min_size: u64, type_index: u32, extensions: &[MemoryExtensions]) -> Self::Memory;
}
pub trait MemorySource{
    fn get_space(&self, size: u64, alignment: Option<u64>) -> MemPart;
}
pub struct MemoryAllocator<D:DeviceSource>{
    device: D,
    min_size: u64,
    type_index: u32,
    extensions: Vec<MemoryExtensions>,
    allocations: Mutex<Vec<MemAlloc>>,
}

pub mod bufferallocator;
pub trait BufferAllocatorFactory{
    type Buffer: BufferSource;
    fn create_buffer(
        &self, 
        min_size: u64, 
        usage: vk::BufferUsageFlags, 
        flags: Option<vk::BufferCreateFlags>, 
        extensions: &[BufferExtensions], 
        share: Option<Vec<u32>>) -> Self::Buffer;
}
pub trait BufferSource{
    fn get_space(&self, size: u64, alignment: Option<u64>) -> (MemPart,BufPart);
}
pub struct BufferAllocator<M:DeviceSource + MemorySource>{
    mem_alloc: M,
    min_size: u64,
    usage: vk::BufferUsageFlags,
    flags: Option<vk::BufferCreateFlags>,
    extensions: Vec<BufferExtensions>,
    share: Option<Vec<u32>>,
    buffers: Mutex<Vec<BufAlloc>>,
}

pub mod buffersegment;
pub trait BufferSegmentFactory{
    type Segment: BufferSegmentSource;
    fn get_segment(&self, size: u64, alignment: Option<u64>) -> Self::Segment;
}
pub trait BufferSegmentSource{
    
}
pub struct BufferSegment<B:DeviceSource + BufferSource>{
    buffer: B,
    mem_part: MemPart,
    buf_part: BufPart,
}

pub mod imageallocator;
pub trait ImageAllocatorFactory{
    
}
pub struct ImageAllocator<M:DeviceSource + MemorySource>{
    mem_alloc: M,
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
}
