use std::sync::{Arc, Mutex};

use ash::vk;
use log::{info, debug};

use crate::{device::{DeviceProvider, UsesDeviceProvider}, memory::{Partition, partitionsystem::{PartitionError, self, PartitionProvider}, memory::{MemoryProvider, UsesMemoryProvider}, PartitionSystem}};

use super::Buffer;


pub trait BufferSettingsProvider{
    fn size(&self) -> vk::DeviceSize;
    fn flags(&self) -> Option<vk::BufferCreateFlags>;
    fn extensions(&self) -> Option<Vec<BufferCreateExtension>>;
    fn usage(&self) -> vk::BufferUsageFlags;
    fn share(&self) -> Option<Vec<u32>>;
}
pub trait BufferProvider{
    fn buffer(&self) -> &vk::Buffer;
    ///Gets the Allocation partition this buffer is stored at
    fn home_partition(&self) -> &Partition;
    ///Partitions this buffer
    fn partition(&self, size: u64, alignment_type: BufferAlignmentType) -> Result<Partition, PartitionError> ;
    fn usage(&self) -> vk::BufferUsageFlags;
}

pub trait UsesBufferProvider<B:BufferProvider>{
    fn buffer_provider(&self) -> &Arc<B>;
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

pub struct SettingsProvider{
    pub size: vk::DeviceSize,
    pub flags: Option<vk::BufferCreateFlags>,
    pub extensions: Option<Vec<BufferCreateExtension>>,
    pub usage: vk::BufferUsageFlags,
    pub share: Option<Vec<u32>>,
}

impl<D:DeviceProvider, M:MemoryProvider> Buffer<D,M,PartitionSystem>{
    pub fn new<S:BufferSettingsProvider>(settings: &S, device_provider: &Arc<D>, memory_provider: &Arc<M>) -> Result<Arc<Buffer<D,M,PartitionSystem>>, BufferCreateError>{
        // First we need to create the buffer
        let mut info = vk::BufferCreateInfo::builder();
        let mut extensions = settings.extensions();
        if let Some(extensions) = &mut extensions{
            for ext in extensions.iter_mut(){
                match ext {
                    _ => todo!()
                };
            }
        }
        if let Some(flags) = settings.flags(){
            info = info.flags(flags);
        }
        info = info.size(settings.size());
        info = info.usage(settings.usage());
        info = info.sharing_mode(vk::SharingMode::EXCLUSIVE);
        let share = settings.share();
        if let Some(indecies) = &share{
            info = info.sharing_mode(vk::SharingMode::CONCURRENT);
            info = info.queue_family_indices(&indecies);
        }
        
        let device = device_provider.device();
        let buffer = unsafe{device.create_buffer(&info, None)};
        
        if let Err(e) = buffer{
            return Err(BufferCreateError::VulkanResult(e));
        }
        
        let buffer = buffer.unwrap();
        let reqs:vk::MemoryRequirements = unsafe{device.get_buffer_memory_requirements(buffer)};
        let memory_partition = memory_provider.partition(reqs.size, Some(reqs.alignment));
            
        if let Err(e) = memory_partition{
            return Err(BufferCreateError::ParitionError(e));
        }
        let memory_partition = memory_partition.unwrap();
        let offset = memory_partition.offset();
        info!("Created buffer {:?} of size {:?} on memory {:?} at offset {:?}", buffer, settings.size(), *memory_provider.memory(), offset);
        
        // Now that we have a suitable memory partition we need to bind our buffer
        let result = unsafe{device.bind_buffer_memory(buffer, *memory_provider.memory(), offset)};
        if let Err(e) = result {
            return Err(BufferCreateError::VulkanResult(e));
        }

        let mut alignment = BufferAlignmentType::Free;
        if settings.usage().contains(vk::BufferUsageFlags::STORAGE_BUFFER){
            alignment = BufferAlignmentType::Aligned(device_provider.physical_device().properties.limits.min_storage_buffer_offset_alignment);
        }
        else if settings.usage().contains(vk::BufferUsageFlags::UNIFORM_BUFFER){
            alignment = BufferAlignmentType::Aligned(device_provider.physical_device().properties.limits.min_uniform_buffer_offset_alignment);
        }
        else if settings.usage().contains(vk::BufferUsageFlags::UNIFORM_TEXEL_BUFFER){
            alignment = BufferAlignmentType::Aligned(device_provider.physical_device().properties.limits.min_texel_buffer_offset_alignment);
        }
        
        Ok(Arc::new(Buffer{
            device: device_provider.clone(),
            memory: memory_provider.clone(),
            memory_partition,
            partition_sys: Mutex::new(PartitionSystem::new(settings.size())),
            buffer,
            alignment_type: alignment,
            info: info.build(),
        }))
    }
}

impl<D:DeviceProvider, M:MemoryProvider, P:PartitionProvider> BufferProvider for Buffer<D,M,P>{

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

impl<D:DeviceProvider, M:MemoryProvider, P:PartitionProvider> Drop for Buffer<D,M,P>{
    fn drop(&mut self) {
        debug!("Destroyed buffer {:?}", self.buffer);
        unsafe{
            self.device.device().destroy_buffer(self.buffer, None);
        }
    }
}

impl SettingsProvider{
    pub fn new(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> SettingsProvider {
        SettingsProvider{
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

impl BufferSettingsProvider for SettingsProvider{
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

impl<D:DeviceProvider, P:PartitionProvider, M:MemoryProvider> UsesDeviceProvider<D> for Buffer<D,M,P>{
    fn device_provider(&self) -> &Arc<D> {
        &self.device
    }
}

impl<D:DeviceProvider, P:PartitionProvider, M:MemoryProvider> UsesMemoryProvider<M> for Buffer<D,M,P>{
    fn memory_provider(&self) -> &Arc<M> {
        &self.memory
    }
}
