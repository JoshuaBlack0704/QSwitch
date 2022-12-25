use std::{sync::{Arc, Mutex}, ffi::c_void};

use ash::vk;
use log::{debug, info};

use crate::{init::{DeviceSource, DeviceSupplier}, memory::{Partition, partitionsystem::{self, PartitionError}, PartitionSystem}};
use crate::memory::{MemorySupplier, MemorySource, PartitionSource};
use crate::memory::buffer::BufferSource;

use super::{Buffer, BufferFactory, BufferSupplier};


pub trait BufferSettingsStore{
    fn size(&self) -> vk::DeviceSize;
    fn flags(&self) -> Option<vk::BufferCreateFlags>;
    fn extensions(&self) -> Option<Vec<BufferCreateExtension>>;
    fn usage(&self) -> vk::BufferUsageFlags;
    fn share(&self) -> Option<Vec<u32>>;
}

#[derive(Clone)]
pub enum BufferCreateExtension{
    
}

#[derive(Clone, Copy)]
pub enum BufferAlignmentType{
    // No alignent, like for vertex buffers
    Free,
    // For example, if you have a storage buffer, you will need to align to minStorageBufferOffsetAlignment
    Aligned(u64),
}

#[derive(Debug)]
pub enum BufferCreateError{
    VulkanResult(vk::Result),
    ParitionError(partitionsystem::PartitionError)
}

pub struct SettingsStore{
    pub size: vk::DeviceSize,
    pub flags: Option<vk::BufferCreateFlags>,
    pub extensions: Option<Vec<BufferCreateExtension>>,
    pub usage: vk::BufferUsageFlags,
    pub share: Option<Vec<u32>>,
}

impl<D:DeviceSource + Clone , M:MemorySource + Clone, MS:MemorySupplier<M> + DeviceSupplier<D>> BufferFactory<Arc<Buffer<D,M,PartitionSystem>>> for MS{
    fn create_buffer(&self, size: u64, usage: vk::BufferUsageFlags, flags: Option<vk::BufferCreateFlags>, extensions: Option<*const c_void>) -> Result<Arc<Buffer<D,M,PartitionSystem>>, BufferCreateError> {
        // First we need to create the buffer
        let mut info = vk::BufferCreateInfo::builder();
        if let Some(ptr) = extensions{
           info.p_next = ptr; 
        }
        if let Some(flags) = flags{
            info = info.flags(flags);
        }
        info = info.size(size);
        info = info.usage(usage);
        info = info.sharing_mode(vk::SharingMode::EXCLUSIVE);
        let indices = self.share();
        if let Some(indices) = &indices{
            info = info.sharing_mode(vk::SharingMode::CONCURRENT);
            info = info.queue_family_indices(&indices);
        }
        
        let device = self.device_provider().device();
        let buffer = unsafe{device.create_buffer(&info, None)};
        
        if let Err(e) = buffer{
            return Err(BufferCreateError::VulkanResult(e));
        }
        
        let buffer = buffer.unwrap();
        let reqs:vk::MemoryRequirements = unsafe{device.get_buffer_memory_requirements(buffer)};
        let memory_partition = self.memory_source().partition(reqs.size, Some(reqs.alignment));
            
        if let Err(e) = memory_partition{
            return Err(BufferCreateError::ParitionError(e));
        }
        let memory_partition = memory_partition.unwrap();
        let offset = memory_partition.offset();
        info!("Created buffer {:?} of size {:?} on memory {:?} at offset {:?}", buffer, size, *self.memory_source().memory(), offset);
        
        // Now that we have a suitable memory partition we need to bind our buffer
        let result = unsafe{device.bind_buffer_memory(buffer, *self.memory_source().memory(), offset)};
        if let Err(e) = result {
            return Err(BufferCreateError::VulkanResult(e));
        }

        let mut alignment = BufferAlignmentType::Free;
        if usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER){
            alignment = BufferAlignmentType::Aligned(self.device_provider().physical_device().properties.limits.min_storage_buffer_offset_alignment);
        }
        else if usage.contains(vk::BufferUsageFlags::UNIFORM_BUFFER){
            alignment = BufferAlignmentType::Aligned(self.device_provider().physical_device().properties.limits.min_uniform_buffer_offset_alignment);
        }
        else if usage.contains(vk::BufferUsageFlags::UNIFORM_TEXEL_BUFFER){
            alignment = BufferAlignmentType::Aligned(self.device_provider().physical_device().properties.limits.min_texel_buffer_offset_alignment);
        }
        
        Ok(Arc::new(Buffer{
            device: self.device_provider().clone(),
            memory: self.memory_source().clone(),
            memory_partition,
            partition_sys: Mutex::new(PartitionSystem::new(size)),
            buffer,
            alignment_type: alignment,
            info: info.build(),
        }))
    }
}

impl<D:DeviceSource, M:MemorySource, P:PartitionSource> BufferSource for Arc<Buffer<D,M,P>>{

    fn buffer(&self) -> &vk::Buffer {
        &self.buffer
    }

    fn home_partition(&self) -> &Partition {
        &self.memory_partition
    }

    fn partition(&self, size: u64, alignment_type: BufferAlignmentType) -> Result<Partition, PartitionError> {
        self.partition_sys.lock().unwrap().partition(size, |offset|{
            if let BufferAlignmentType::Aligned(a) = alignment_type{
                return offset % a == 0;
            } 
            if let BufferAlignmentType::Aligned(a) = self.alignment_type{
                return offset % a == 0;
            } 
            true
        })
    }

    fn usage(&self) -> vk::BufferUsageFlags {
        self.info.usage
    }
}

impl<D:DeviceSource, M:MemorySource, P:PartitionSource> Drop for Buffer<D,M,P>{
    fn drop(&mut self) {
        debug!("Destroyed buffer {:?}", self.buffer);
        unsafe{
            self.device.device().destroy_buffer(self.buffer, None);
        }
    }
}

impl SettingsStore{
    pub fn new(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> SettingsStore {
        SettingsStore{
            size,
            flags: None,
            extensions: None,
            usage,
            share: None,
        }
    }
    
    pub fn set_create_flags(&mut self, flags: vk::BufferCreateFlags){
        self.flags = Some(flags);
    }
    
    pub fn add_extension(&mut self, ext: BufferCreateExtension){
        self.extensions.get_or_insert(vec![]).push(ext);
    }
    
    pub fn share(&mut self, indecies: &[u32]){
        self.share = Some(indecies.to_vec());
    }
}

impl BufferSettingsStore for SettingsStore{
    fn size(&self) -> vk::DeviceSize {
        self.size
    }

    fn flags(&self) -> Option<vk::BufferCreateFlags> {
        self.flags
    }

    fn extensions(&self) -> Option<Vec<BufferCreateExtension>> {
        self.extensions.clone()
    }

    fn usage(&self) -> vk::BufferUsageFlags {
        self.usage
    }

    fn share(&self) -> Option<Vec<u32>> {
        self.share.clone()
    }
}

impl<D:DeviceSource, P:PartitionSource, M:MemorySource> DeviceSupplier<D> for Arc<Buffer<D,M,P>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}

impl<D:DeviceSource, P:PartitionSource, M:MemorySource> MemorySupplier<M> for Arc<Buffer<D,M,P>>{
    fn memory_source(&self) -> &M {
        &self.memory
    }
}

impl <D:DeviceSource, M:MemorySource, P:PartitionSource> BufferSupplier<Self> for Arc<Buffer<D,M,P>>{
    fn buffer_provider(&self) -> &Self {
        self
    }
}
