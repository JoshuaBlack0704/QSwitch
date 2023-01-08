use std::sync::{Arc, Mutex};

use ash::vk;
use log::info;

use crate::{init::DeviceSource, allocator::PartitionSystem};

use super::{MemoryAllocator, MemoryExtensions, test_partition};

impl<D:DeviceSource + Clone> MemoryAllocator<D>{
    pub fn new(device_source: &D, min_size: u64, type_index: u32, extensions: Vec<MemoryExtensions>) -> Arc<MemoryAllocator<D>> {
        Arc::new(
            Self{
                device: device_source.clone(),
                min_size,
                type_index,
                extensions,
                allocations: Mutex::new(vec![]),
            }
        )
    }

    pub fn get_space(&self, size: u64, alignment: Option<u64>) -> crate::allocator::Partition {
        //First we need to loop through all of the memory allocations
        //and attempt to find a block of space large enough
        let mut allocs = self.allocations.lock().unwrap();
        for (_,p) in allocs.iter(){
            if let Ok(p) = test_partition(p, size, alignment){
                return p;
            }
        }

        //If we made it here, we have no space in any of our allocations
        //and must create a new one
        let new_size;
        if size < self.min_size{
            new_size = self.min_size;
        }
        else{
            new_size = size * 2;
        }

        let mut info = vk::MemoryAllocateInfo::builder()
        .allocation_size(new_size)
        .memory_type_index(self.type_index);

        let mut extensions = self.extensions.clone();
        for ext in extensions.iter_mut(){
            info = ext.push(info);
        }

        let memory = unsafe{self.device.device().allocate_memory(&info, None).unwrap()};
        info!("Allocated memory {:?}", memory);

        let p_sys = Mutex::new(PartitionSystem::new(new_size));
        let p = test_partition(&p_sys, size, alignment).unwrap();
        let allocation = (memory, p_sys);
        allocs.push(allocation);
        p

    }
}

impl MemoryExtensions{
    fn push<'a>(&'a mut self, mut builder: vk::MemoryAllocateInfoBuilder<'a>) -> vk::MemoryAllocateInfoBuilder<'a> {
        match self{
            MemoryExtensions::Flags(f) => builder = builder.push_next(f),
        }
        builder
    }
}