use std::sync::Arc;

use ash::vk;
use log::{info, debug};
use crate::command::BindSetFactory;
use crate::descriptor::{DescriptorLayoutSource, DescriptorPoolSource};

use crate::init::{DeviceSource, InstanceSource};

use super::{Set, WriteSource, SetSource, SetFactory};

impl<W:WriteSource + Clone, L:DescriptorLayoutSource<W>, P:DescriptorPoolSource + DeviceSource + Clone> SetFactory<Arc<Set<P,W>>, W, L> for P{
    fn create_set(&self, layout_provider: &L) -> Arc<Set<P,W>> {
        let set = self.allocate_set(layout_provider);
        info!("Created descriptor set {:?} using layout {:?} from pool {:?}", set, layout_provider.layout(), self.pool());
        let writes = layout_provider.writes().clone();
        Arc::new(
            Set {
                pool: self.clone(),
                writes,
                set,
            }
        )
    }
}

impl<W:WriteSource, P:DescriptorPoolSource + DeviceSource> SetSource for Arc<Set<P,W>>{
    ///Will perform any writes needed to make the set current
    fn update(&self){
        let mut updates:Vec<vk::WriteDescriptorSet> = self.writes.iter().filter(|w| w.needs_write()).map(|w| w.get_write()).collect();

        for u in updates.iter_mut(){
            u.dst_set = self.set;
            debug!("Writing binding {:?} in descriptor set {:?}", u.dst_binding, u.dst_set);
        }

        unsafe{
            let device = self.device();
            device.update_descriptor_sets(&updates, &[]);
        }
    }
    
}

impl<P:DescriptorPoolSource + DeviceSource, W:WriteSource> BindSetFactory for  Arc<Set<P,W>>{
    fn set(&self) -> vk::DescriptorSet {
        self.update();
        self.set
    }

    fn dynamic_offsets(&self) -> Option<Vec<u32>> {
        None
    }
}

impl<P:DescriptorPoolSource + DeviceSource + InstanceSource, W:WriteSource> InstanceSource for  Arc<Set<P,W>>{
    
    fn instance(&self) -> &ash::Instance {
        self.pool.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.pool.entry()
    }
}

impl<P:DescriptorPoolSource + DeviceSource, W:WriteSource> DeviceSource for  Arc<Set<P,W>>{
    
    fn device(&self) -> &ash::Device {
        self.pool.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.pool.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.pool.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.pool.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.pool.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.pool.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.pool.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.pool.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.pool.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.pool.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.pool.host_memory_index()
    }
}
