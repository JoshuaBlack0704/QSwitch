use std::sync::Arc;

use ash::vk;
use log::{info, debug};
use crate::command::BindSetFactory;
use crate::descriptor::{DescriptorLayoutStore, DescriptorPoolStore};

use crate::init::DeviceSource;

use super::{Set, WriteStore};

impl<D:DeviceSource + Clone, W:WriteStore + Clone, L:DescriptorLayoutStore<W> + Clone, P:DescriptorPoolStore + Clone> Set<D,L,P,W>{
    pub fn new(device_provider: &D, layout_provider: &L, pool_provider: &P) -> Arc<Self> {
        let set = pool_provider.allocate_set(layout_provider);
        info!("Created descriptor set {:?} using layout {:?} from pool {:?}", set, layout_provider.layout(), pool_provider.pool());
        let writes = layout_provider.writes().clone();
        Arc::new(
            Self{
                device: device_provider.clone(),
                layout: layout_provider.clone(),
                _pool: pool_provider.clone(),
                writes,
                set,
            }
        )
    }
}

impl<D:DeviceSource, W:WriteStore, L:DescriptorLayoutStore<W>, P:DescriptorPoolStore> Set<D,L,P,W>{
    ///Will perform any writes needed to make the set current
    pub fn update(self: &Arc<Self>){
        let mut updates:Vec<vk::WriteDescriptorSet> = self.writes.iter().filter(|w| w.needs_write()).map(|w| w.get_write()).collect();

        for u in updates.iter_mut(){
            u.dst_set = self.set;
            debug!("Writing binding {:?} in descriptor set {:?}", u.dst_binding, u.dst_set);
        }

        unsafe{
            let device = self.device.device();
            device.update_descriptor_sets(&updates, &[]);
        }
    }
    
}

impl<D:DeviceSource, L:DescriptorLayoutStore<W>, P:DescriptorPoolStore, W:WriteStore> BindSetFactory for  Arc<Set<D,L,P,W>>{
    fn set(&self) -> vk::DescriptorSet {
        self.update();
        self.set
    }

    fn dynamic_offsets(&self) -> Option<Vec<u32>> {
        None
    }
}
