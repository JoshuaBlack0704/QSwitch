use std::sync::{Arc, Mutex};

use ash::vk;
use log::{info, debug};
use tokio::sync::RwLock;

use crate::device;

use super::{Buffer, memory, PartitionSystem, partitionsystem::{self, PartitionProvider, PartitionError}, Partition, bufferpartition::{BufferPartitionProvider, BufferPartitioner}, BufferPartition};

pub trait BufferSettingsProvider{
    fn size(&self) -> vk::DeviceSize;
    fn flags(&self) -> Option<vk::BufferCreateFlags>;
    fn extensions(&self) -> Option<Vec<BufferCreateExtension>>;
    fn usage(&self) -> vk::BufferUsageFlags;
    fn share(&self) -> Option<Vec<u32>>;
    
}
pub trait BufferProvider{
    fn buffer(&self) -> &vk::Buffer;
    fn home_partition(&self) -> &Partition;
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
    size: vk::DeviceSize,
    flags: Option<vk::BufferCreateFlags>,
    extensions: Option<Vec<BufferCreateExtension>>,
    usage: vk::BufferUsageFlags,
    share: Option<Vec<u32>>,
}

impl<D:device::DeviceProvider, M:memory::MemoryProvider> Buffer<D,M,PartitionSystem>{
    pub fn new<S:BufferSettingsProvider>(settings: &S, device_provider: &Arc<D>, memory_provider: &Arc<M>) -> Result<Buffer<D,M,PartitionSystem>, BufferCreateError>{
        // First we need to create the buffer
        let mut b_cinfo = vk::BufferCreateInfo::builder();
        let mut extensions = settings.extensions();
        if let Some(extensions) = &mut extensions{
            for ext in extensions.iter_mut(){
                match ext {
                    _ => todo!()
                };
            }
        }
        if let Some(flags) = settings.flags(){
            b_cinfo = b_cinfo.flags(flags);
        }
        b_cinfo = b_cinfo.size(settings.size());
        b_cinfo = b_cinfo.usage(settings.usage());
        b_cinfo = b_cinfo.sharing_mode(vk::SharingMode::EXCLUSIVE);
        let share = settings.share();
        if let Some(indecies) = &share{
            b_cinfo = b_cinfo.sharing_mode(vk::SharingMode::CONCURRENT);
            b_cinfo = b_cinfo.queue_family_indices(&indecies);
        }
        
        let device = device_provider.device();
        let buffer = unsafe{device.create_buffer(&b_cinfo, None)};
        
        if let Err(e) = buffer{
            return Err(BufferCreateError::VulkanResult(e));
        }
        
        let buffer = buffer.unwrap();
        info!("Created buffer {:?}", buffer);
        let reqs:vk::MemoryRequirements = unsafe{device.get_buffer_memory_requirements(buffer)};
        let memory_partition = memory_provider.partition(reqs.size, Some(reqs.alignment));
            
        if let Err(e) = memory_partition{
            return Err(BufferCreateError::ParitionError(e));
        }
        let memory_partition = memory_partition.unwrap();
        let offset = memory_partition.offset();
        
        // Now that we have a suitable memory partition we need to bind our buffer
        let result = unsafe{device.bind_buffer_memory(buffer, *memory_provider.memory(), offset)};
        if let Err(e) = result {
            return Err(BufferCreateError::VulkanResult(e));
        }

        let alignment = BufferAlignmentType::Free;
        if settings.usage().contains(vk::BufferUsageFlags::STORAGE_BUFFER){
            alignment = BufferAlignmentType::Aligned(device_provider.physical_device().properties.limits.min_storage_buffer_offset_alignment);
        }
        else if settings.usage().contains(vk::BufferUsageFlags::UNIFORM_BUFFER){
            alignment = BufferAlignmentType::Aligned(device_provider.physical_device().properties.limits.min_uniform_buffer_offset_alignment);
        }
        else if settings.usage().contains(vk::BufferUsageFlags::UNIFORM_TEXEL_BUFFER){
            alignment = BufferAlignmentType::Aligned(device_provider.physical_device().properties.limits.min_texel_buffer_offset_alignment);
        }
        
        Ok(Buffer{
            device: device_provider.clone(),
            memory: memory_provider.clone(),
            memory_partition,
            partition_sys: Mutex::new(PartitionSystem::new(settings.size())),
            buffer,
            alignment_type: alignment,
        })
    }
}

impl <D:device::DeviceProvider, M:memory::MemoryProvider, P:PartitionProvider> BufferProvider for Buffer<D,M,P>{
    fn buffer(&self) -> &vk::Buffer {
        &self.buffer
    }

    fn home_partition(&self) -> &Partition {
        &self.memory_partition
    }
}

impl <D:device::DeviceProvider, M:memory::MemoryProvider, P:PartitionProvider> BufferPartitioner for Buffer<D,M,P>{
    type Provider = BufferPartition<D,Self,P>;

    fn partition(&self, size: u64) -> Result<Arc<Self::Provider>, PartitionError> {
        let alignmen_fn = |offset: u64|{
            if let BufferAlignmentType::Aligned(a) = self.alignment_type{
                return offset % a == 0;
            }
            true
        };

        let partition = self.partition_sys.lock().unwrap().partition(size, alignmen_fn);

        match partition{
            Ok(p) => {
                
            },
            Err(e) => todo!(),
        }

    }

    fn partition_aligned(&self, size: u64, alignment: BufferAlignmentType) -> Arc<Self::Provider> {
        todo!()
    }
}

impl <D:device::DeviceProvider, M:memory::MemoryProvider, P:PartitionProvider> BufferPartitionProvider for Buffer<D,M,P>{
    fn alignment_type(&self) -> BufferAlignmentType {
        todo!()
    }

    fn device_addr(&self) -> vk::DeviceSize {
        todo!()
    }
}



impl<D:device::DeviceProvider, M:memory::MemoryProvider, P:PartitionProvider> Drop for Buffer<D,M,P>{
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