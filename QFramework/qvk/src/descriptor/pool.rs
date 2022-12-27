use std::{collections::HashMap, sync::Arc};

use ash::vk;
use log::{debug, info};
use crate::descriptor::{DescriptorLayoutSource, DescriptorPoolSource};

use crate::init::{DeviceSource, InstanceSource};

use super::{Pool, WriteSource, DescriptorPoolFactory};

impl<D:DeviceSource + Clone> DescriptorPoolFactory<Arc<Pool<D>>> for D{
    fn create_descriptor_pool<W:WriteSource, L:DescriptorLayoutSource<W>>(&self, layout_set_count: &[(&L, u32)], flags: Option<vk::DescriptorPoolCreateFlags>) -> Arc<Pool<D>> {
        let mut pool_sizes:HashMap<vk::DescriptorType, vk::DescriptorPoolSize> = HashMap::new();
        let mut max_sets = 0;

        for (layout_provider, set_count) in layout_set_count.iter(){
            let bindings = layout_provider.bindings();
            for binding in bindings.iter(){
                if let Some(size) = pool_sizes.get_mut(&binding.descriptor_type){
                    size.descriptor_count += binding.descriptor_count * set_count;
                }
                else{
                    let _ = pool_sizes
                        .insert(
                            binding.descriptor_type, 
                            vk::DescriptorPoolSize::builder()
                                .ty(binding.descriptor_type)
                                .descriptor_count(binding.descriptor_count * set_count)
                                .build());
                }
                max_sets += set_count;
            }
        }

        let pool_sizes:Vec<vk::DescriptorPoolSize> = pool_sizes.values().map(|s| *s).collect();
        
        let mut info = vk::DescriptorPoolCreateInfo::builder()
        .pool_sizes(&pool_sizes)
        .max_sets(max_sets);
        
        if let Some(flags) = flags{
            info = info.flags(flags);
        }

        let pool;
        unsafe{
            let device = self.device();
            pool = device.create_descriptor_pool(&info, None).unwrap();
            info!("Created descriptor pool {:?} for {max_sets} sets", pool);
        }

        Arc::new(
            Pool{
                device: self.clone(),
                pool,
            }
        )
    }
}

impl<D:DeviceSource> DescriptorPoolSource for Arc<Pool<D>>{
    fn allocate_set<W:WriteSource, L:DescriptorLayoutSource<W>>(&self, layout: &L) -> vk::DescriptorSet {

        let requests = [layout.layout()];
        
        let info = vk::DescriptorSetAllocateInfo::builder()
        .descriptor_pool(self.pool)
        .set_layouts(&requests);

        unsafe{
            let device = self.device.device();
            device.allocate_descriptor_sets(&info).unwrap()[0]
        }
        
    }

    fn pool(&self) -> vk::DescriptorPool {
        self.pool
    }
}

impl<D:DeviceSource> Drop for Pool<D>{
    fn drop(&mut self) {
        debug!("Destroyed descriptor pool {:?}", self.pool);
        unsafe{
            self.device.device().destroy_descriptor_pool(self.pool, None);
        }
    }
}

impl<D:DeviceSource + InstanceSource> InstanceSource for Arc<Pool<D>>{
    
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}

impl<D:DeviceSource> DeviceSource for Arc<Pool<D>>{
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