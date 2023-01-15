use std::sync::{Arc, Mutex};

use ash::vk;
use log::{debug, info};

use crate::{
    init::{DeviceSource, InstanceSource},
    memory::allocators::PartitionSystem,
};

use super::{
    test_partition, MemoryAllocator, MemoryAllocatorFactory, MemoryExtensions, MemorySource,
    Partition,
};

impl<D: DeviceSource + Clone> MemoryAllocatorFactory for D {
    type Memory = Arc<MemoryAllocator<D>>;

    fn create_memory(
        &self,
        min_size: u64,
        type_index: u32,
        extensions: &[MemoryExtensions],
    ) -> Self::Memory {
        Arc::new(MemoryAllocator {
            device: self.clone(),
            min_size,
            type_index,
            extensions: extensions.to_vec(),
            allocations: Mutex::new(vec![]),
        })
    }

    fn create_gpu_mem(&self, min_size: u64) -> Self::Memory {
        let type_index = self.device_memory_index();
        let extensions = [];
        self.create_memory(min_size, type_index, &extensions)
    }

    fn create_cpu_mem(&self, min_size: u64) -> Self::Memory {
        let type_index = self.host_memory_index();
        let extensions = [];
        self.create_memory(min_size, type_index, &extensions)
    }
}
impl<D: DeviceSource + Clone> MemorySource for Arc<MemoryAllocator<D>> {
    fn get_space(&self, size: u64, alignment: Option<u64>) -> (vk::DeviceMemory, Partition) {
        //First we need to loop through all of the memory allocations
        //and attempt to find a block of space large enough
        let mut allocs = self.allocations.lock().unwrap();
        for (m, p) in allocs.iter() {
            if let Ok(p) = test_partition(p, size, alignment) {
                return (*m, p);
            }
        }

        //If we made it here, we have no space in any of our allocations
        //and must create a new one
        let new_size;
        if size < self.min_size {
            new_size = self.min_size;
        } else {
            new_size = size * 2;
        }

        let mut info = vk::MemoryAllocateInfo::builder()
            .allocation_size(new_size)
            .memory_type_index(self.type_index);

        let mut extensions = self.extensions.clone();
        for ext in extensions.iter_mut() {
            info = ext.push(info);
        }

        let memory = unsafe { self.device.device().allocate_memory(&info, None).unwrap() };
        info!("Allocated memory {:?}", memory);

        let p_sys = Mutex::new(PartitionSystem::new(new_size));
        let p = test_partition(&p_sys, size, alignment).unwrap();
        let allocation = (memory, p_sys);
        allocs.push(allocation);
        (memory, p)
    }
}

impl MemoryExtensions {
    fn push<'a>(
        &'a mut self,
        mut builder: vk::MemoryAllocateInfoBuilder<'a>,
    ) -> vk::MemoryAllocateInfoBuilder<'a> {
        match self {
            MemoryExtensions::Flags(f) => builder = builder.push_next(f),
        }
        builder
    }
}

impl<D: DeviceSource> Drop for MemoryAllocator<D> {
    fn drop(&mut self) {
        let allocs = self.allocations.lock().unwrap();
        for (m, _) in allocs.iter() {
            debug!("Freed memory {:?}", m);
            unsafe {
                self.device.device().free_memory(*m, None);
            }
        }
    }
}

impl<D: DeviceSource> DeviceSource for Arc<MemoryAllocator<D>> {
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

impl<D: DeviceSource + InstanceSource> InstanceSource for Arc<MemoryAllocator<D>> {
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}
