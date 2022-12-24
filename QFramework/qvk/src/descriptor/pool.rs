use std::{collections::HashMap, sync::Arc};

use ash::vk;
use log::{debug, info};
use crate::descriptor::{DescriptorLayoutSource, DescriptorPoolStore};

use crate::init::{DeviceSource, DeviceSupplier};

use super::{Pool, WriteStore};

impl<D:DeviceSource + Clone> Pool<D>{
    pub fn new<W:WriteStore, L:DescriptorLayoutSource<W> + Clone>(device_provider: &D, layout_set_count: &[(&L, u32)], flags: Option<vk::DescriptorPoolCreateFlags>) -> Arc<Pool<D>> {
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
            let device = device_provider.device();
            pool = device.create_descriptor_pool(&info, None).unwrap();
            info!("Created descriptor pool {:?} for {max_sets} sets", pool);
        }

        Arc::new(
            Self{
                device: device_provider.clone(),
                pool,
            }
        )
    }
}

impl<D:DeviceSource> DescriptorPoolStore for Arc<Pool<D>>{
    fn allocate_set<W:WriteStore, L:DescriptorLayoutSource<W>>(&self, layout: &L) -> vk::DescriptorSet {

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

impl<D:DeviceSource> DeviceSupplier<D> for Arc<Pool<D>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}