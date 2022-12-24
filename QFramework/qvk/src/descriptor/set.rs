use std::sync::Arc;

use ash::vk;
use log::{info, debug};
use crate::command::BindSetFactory;
use crate::descriptor::{DescriptorLayoutSource, DescriptorPoolStore};

use crate::init::{DeviceSource, DeviceSupplier};

use super::{Set, WriteStore, SetSource, SetFactory};

impl<D:DeviceSource + Clone, W:WriteStore + Clone, L:DescriptorLayoutSource<W> + Clone, P:DescriptorPoolStore + DeviceSupplier<D> + Clone> SetFactory<Arc<Set<D,L,P,W>>, W, L> for P{
    fn create_set(&self, layout_provider: &L) -> Arc<Set<D,L,P,W>> {
        let set = self.allocate_set(layout_provider);
        info!("Created descriptor set {:?} using layout {:?} from pool {:?}", set, layout_provider.layout(), self.pool());
        let writes = layout_provider.writes().clone();
        Arc::new(
            Set {
                device: self.device_provider().clone(),
                layout: layout_provider.clone(),
                _pool: self.clone(),
                writes,
                set,
            }
        )
    }
}

impl<D:DeviceSource, W:WriteStore, L:DescriptorLayoutSource<W>, P:DescriptorPoolStore> SetSource for Arc<Set<D,L,P,W>>{
    ///Will perform any writes needed to make the set current
    fn update(&self){
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

impl<D:DeviceSource, L:DescriptorLayoutSource<W>, P:DescriptorPoolStore, W:WriteStore> BindSetFactory for  Arc<Set<D,L,P,W>>{
    fn set(&self) -> vk::DescriptorSet {
        self.update();
        self.set
    }

    fn dynamic_offsets(&self) -> Option<Vec<u32>> {
        None
    }
}
