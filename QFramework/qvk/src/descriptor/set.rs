use std::sync::Arc;

use log::info;
use crate::descriptor::{DescriptorLayoutStore, DescriptorPoolStore};

use crate::init::DeviceStore;

use super::Set;

impl<D:DeviceStore + Clone, L:DescriptorLayoutStore + Clone, P:DescriptorPoolStore + Clone> Set<D,L,P>{
    pub fn new(device_provider: &D, layout_provider: &L, pool_provider: &P) -> Arc<Set<D, L, P>> {
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