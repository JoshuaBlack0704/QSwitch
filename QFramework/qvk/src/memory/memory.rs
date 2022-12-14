use std::sync::{Arc, Mutex};
use ash::vk;
use log::{info, debug};

use crate::device::DeviceProvider;

use super::{Memory, partitionsystem::{PartitionProvider, self}, PartitionSystem, Partition};

#[derive(Clone)]
pub enum MemoryAllocateExtension{
    Flags(vk::MemoryAllocateFlagsInfo),
}

pub trait MemorySettingsProvider{
    fn size(&self) -> vk::DeviceSize;
    fn memory_type_index(&self) -> u32;
    fn extensions(&self) -> Option<Vec<MemoryAllocateExtension>>;
}
pub trait MemoryProvider{
    fn partition(&self, size: u64, alignment: Option<u64>) -> Result<Partition, partitionsystem::PartitionError>;
    fn memory(&self) -> &vk::DeviceMemory;
}

#[derive(Clone)]
pub struct SettingsProvider{
    size: vk::DeviceSize,
    memory_type_index: u32,
    extensions: Option<Vec<MemoryAllocateExtension>>,
}

impl<D: DeviceProvider> Memory<D,PartitionSystem>{
    pub fn new<S:MemorySettingsProvider>(settings: &S, device_provider: &Arc<D>) -> Result<Arc<Memory<D,PartitionSystem>>, vk::Result>{
        // We need to create the initial memory from our settings
        
        let mut memory_cinfo = vk::MemoryAllocateInfo::builder();
        memory_cinfo = memory_cinfo.allocation_size(settings.size());
        memory_cinfo = memory_cinfo.memory_type_index(settings.memory_type_index());
        let mut extentions = settings.extensions();
        
        if let Some(extentions) = &mut extentions{
            for ext in extentions.iter_mut(){
                match ext{
                    MemoryAllocateExtension::Flags(e) => {
                        memory_cinfo = memory_cinfo.push_next(e);
                    },
                }
            }
        }
        
        let partition = PartitionSystem::new(settings.size());
        let memory = unsafe{device_provider.device().allocate_memory(&memory_cinfo, None)};
        
        match memory{
            Ok(m) => {
                info!("Created device memory {:?}", m);
                let memory = Memory{ 
                    device: device_provider.clone(),
                    partition_sys: Mutex::new(partition),
                    memory: m };
                return Ok(Arc::new(memory));
            },
            Err(e) => Err(e),
        }
    }
}
impl<D: DeviceProvider, P: PartitionProvider> MemoryProvider for Memory<D,P>{
    fn partition(&self, size: u64, alignment: Option<u64>) -> Result<Partition, partitionsystem::PartitionError> {
        self.partition_sys.lock().unwrap().partition(size, move |offset| {
            if let Some(alignment) = alignment{
                return offset % alignment == 0;
            }
            else{
                return true;
            }
        })
    }

    fn memory(&self) -> &vk::DeviceMemory {
        &self.memory
    }
}

impl<D: DeviceProvider, P: PartitionProvider> Drop for Memory<D,P>{
    fn drop(&mut self) {
        debug!("Destroyed device memory {:?}", self.memory);
        unsafe{
            self.device.device().free_memory(self.memory, None);
        }
    }
}

impl SettingsProvider{
    pub fn new(size: vk::DeviceSize, memory_type_index: u32) -> SettingsProvider {
        SettingsProvider{ size, memory_type_index, extensions: None }
    }
    
    pub fn add_extension(&mut self, ext: MemoryAllocateExtension){
       self.extensions.get_or_insert(vec![]).push(ext); 
    }
    
    pub fn use_alloc_flags(&mut self, flags: vk::MemoryAllocateFlags){
        let info = vk::MemoryAllocateFlagsInfo::builder().flags(flags).build();
        self.add_extension(MemoryAllocateExtension::Flags(info));
    }
}

impl MemorySettingsProvider for SettingsProvider{
    fn size(&self) -> vk::DeviceSize {
        self.size
    }

    fn memory_type_index(&self) -> u32 {
        self.memory_type_index
    }

    fn extensions(&self) -> Option<Vec<MemoryAllocateExtension>> {
        self.extensions.clone()
    }

}