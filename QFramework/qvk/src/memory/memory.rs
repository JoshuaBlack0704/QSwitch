use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use ash::vk;
use log::{debug, info};

use crate::init::{DeviceSource, DeviceSupplier};
use crate::memory::{MemorySource, PartitionSource};

use super::{Memory, Partition, partitionsystem, PartitionSystem, MemoryFactory, MemorySupplier};

impl<D:DeviceSource + Clone, DS:DeviceSupplier<D>> MemoryFactory<Arc<Memory<D, PartitionSystem>>> for DS{
    fn create_memory(&self, size: u64, type_index: u32, extensions: Option<*const c_void>) -> Result<Arc<Memory<D, PartitionSystem>>, vk::Result> {
        // We need to create the initial memory from our settings
        
        let mut info = vk::MemoryAllocateInfo::builder();
        info = info.allocation_size(size);
        info = info.memory_type_index(type_index);
        
        if let Some(ptr) = extensions{
            info.p_next = ptr;
        }
        
        let partition = PartitionSystem::new(size);
        let memory = unsafe{self.device_provider().device().allocate_memory(&info, None)};
        
        match memory{
            Ok(m) => {
                info!("Created device memory {:?} of size {:?}", m, size);
                let memory = Memory{ 
                    device: self.device_provider().clone(),
                    partition_sys: Mutex::new(partition),
                    memory: m };
                return Ok(Arc::new(memory));
            },
            Err(e) => Err(e),
        }
    }
}

impl<D: DeviceSource, P: PartitionSource> MemorySource for Arc<Memory<D,P>>{
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

impl<D: DeviceSource, P: PartitionSource> Drop for Memory<D,P>{
    fn drop(&mut self) {
        debug!("Destroyed device memory {:?}", self.memory);
        unsafe{
            self.device.device().free_memory(self.memory, None);
        }
    }
}

impl<D:DeviceSource, P:PartitionSource> DeviceSupplier<D> for Arc<Memory<D,P>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}

impl<D:DeviceSource, P:PartitionSource> MemorySupplier<Self> for Arc<Memory<D,P>>{
    fn memory_source(&self) -> &Self {
        self
    }
}
