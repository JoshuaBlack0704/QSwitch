use std::ffi::c_void;
use std::sync::{Arc, Mutex};
use ash::vk;
use log::{debug, info};

use crate::init::{DeviceSource, InstanceSource};
use crate::memory::{MemorySource, PartitionSource};

use super::{Memory, Partition, partitionsystem, PartitionSystem, MemoryFactory};

impl<D:DeviceSource + Clone> MemoryFactory<Arc<Memory<D, PartitionSystem>>> for D{
    fn create_memory(&self, size: u64, type_index: u32, extensions: Option<*const c_void>) -> Result<Arc<Memory<D, PartitionSystem>>, vk::Result> {
        // We need to create the initial memory from our settings
        
        let mut info = vk::MemoryAllocateInfo::builder();
        info = info.allocation_size(size);
        info = info.memory_type_index(type_index);
        
        if let Some(ptr) = extensions{
            info.p_next = ptr;
        }
        
        let partition = PartitionSystem::new(size);
        let memory = unsafe{self.device().allocate_memory(&info, None)};
        
        match memory{
            Ok(m) => {
                info!("Created device memory {:?} of size {:?}", m, size);
                let memory = Memory{ 
                    device: self.clone(),
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

impl<D:DeviceSource + InstanceSource, P:PartitionSource> InstanceSource for Arc<Memory<D,P>>{
    
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}

impl<D:DeviceSource, P:PartitionSource> DeviceSource for Arc<Memory<D,P>>{
    fn device(&self) -> &ash::Device {
        self.device.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.device.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.device.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.device.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.device.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.device.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.device.host_memory_index()
    }
}

