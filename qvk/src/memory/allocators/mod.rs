use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use ash::vk;

use crate::init::DeviceSource;

use self::partitionsystem::PartitionError;

pub const TRANSFER: fn() -> vk::BufferUsageFlags =
    || vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST;

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
pub enum MemoryExtensions {
    Flags(vk::MemoryAllocateFlagsInfo),
}
#[derive(Clone)]
pub enum BufferExtensions {}
#[derive(Clone)]
pub enum ImageExtensions {}

pub type MemAlloc = (vk::DeviceMemory, Mutex<PartitionSystem>);
pub type MemPart = (vk::DeviceMemory, Partition);
pub type BufAlloc = (
    (vk::DeviceMemory, Partition),
    vk::Buffer,
    Mutex<PartitionSystem>,
);
pub type BufPart = (vk::Buffer, Partition);

fn test_partition(
    partition: &Mutex<PartitionSystem>,
    size: u64,
    alignment: Option<u64>,
) -> Result<Partition, PartitionError> {
    let mut lock = partition.lock().unwrap();
    lock.partition(size, |offset| {
        if let Some(a) = alignment {
            return offset % a == 0;
        }
        true
    })
}

pub mod memallocator;
pub trait MemoryAllocatorFactory {
    type Memory: MemorySource;
    fn create_memory(
        &self,
        min_size: u64,
        type_index: u32,
        extensions: &[MemoryExtensions],
    ) -> Self::Memory;
    fn create_gpu_mem(&self, min_size: u64) -> Self::Memory;
    fn create_cpu_mem(&self, min_size: u64) -> Self::Memory;
}
pub trait MemorySource {
    fn get_space(&self, size: u64, alignment: Option<u64>) -> MemPart;
}
pub struct MemoryAllocator<D: DeviceSource> {
    device: D,
    min_size: u64,
    type_index: u32,
    extensions: Vec<MemoryExtensions>,
    allocations: Mutex<Vec<MemAlloc>>,
}

pub mod bufferallocator;
pub trait BufferAllocatorFactory {
    type Buffer: BufferSource;
    fn create_buffer(
        &self,
        min_size: u64,
        usage: vk::BufferUsageFlags,
        flags: Option<vk::BufferCreateFlags>,
        extensions: &[BufferExtensions],
        share: Option<Vec<u32>>,
    ) -> Self::Buffer;
    fn create_uniform_buffer(
        &self,
        min_size: u64,
        additional_usage: Option<vk::BufferUsageFlags>,
    ) -> Self::Buffer;
    fn create_storage_buffer(
        &self,
        min_size: u64,
        additional_usage: Option<vk::BufferUsageFlags>,
    ) -> Self::Buffer;
    fn create_dev_addr_storage_buffer(
        &self,
        min_size: u64,
        additional_usage: Option<vk::BufferUsageFlags>,
    ) -> Self::Buffer;
    fn create_vertex_buffer(
        &self,
        min_size: u64,
        additional_usage: Option<vk::BufferUsageFlags>,
    ) -> Self::Buffer;
    fn create_index_buffer(
        &self,
        min_size: u64,
        additional_usage: Option<vk::BufferUsageFlags>,
    ) -> Self::Buffer;
}
pub trait BufferSource {
    fn get_space(&self, size: u64, alignment: Option<u64>) -> (MemPart, BufPart);
    fn usage(&self) -> vk::BufferUsageFlags;
}
pub struct BufferAllocator<M: DeviceSource + MemorySource> {
    mem_alloc: M,
    min_size: u64,
    usage: vk::BufferUsageFlags,
    flags: Option<vk::BufferCreateFlags>,
    extensions: Vec<BufferExtensions>,
    share: Option<Vec<u32>>,
    buffers: Mutex<Vec<BufAlloc>>,
}

pub mod imageallocator;
pub trait ImageAllocatorFactory {
    type ImgAlloc: ImageAllocatorSource;
    fn create_image_allocator(
        &self,
        format: vk::Format,
        levels: u32,
        layers: u32,
        usage: vk::ImageUsageFlags,
        img_type: vk::ImageType,
        samples: vk::SampleCountFlags,
        tiling: vk::ImageTiling,
        share: Option<Vec<u32>>,
        flags: Option<vk::ImageCreateFlags>,
        extensions: &[ImageExtensions],
    ) -> Self::ImgAlloc;
    fn create_image_allocator_simple(
        &self,
        format: vk::Format,
        usage: vk::ImageUsageFlags,
    ) -> Self::ImgAlloc;
}
pub trait ImageAllocatorSource {
    fn get_image(&self, extent: vk::Extent3D) -> (vk::ImageCreateInfo, vk::Image, MemPart);
}
pub struct ImageAllocator<M: DeviceSource + MemorySource> {
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
