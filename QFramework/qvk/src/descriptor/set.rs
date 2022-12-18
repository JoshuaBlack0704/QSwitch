use std::sync::Arc;

use log::info;

use crate::device::DeviceProvider;

use super::{descriptorlayout::DescriptorLayoutProvider, pool::DescriptorPoolProvider, Set};

impl<D:DeviceProvider, L:DescriptorLayoutProvider, P:DescriptorPoolProvider> Set<D,L,P>{
    pub fn new(device_provider: &Arc<D>, layout_provider: &Arc<L>, pool_provider: &Arc<P>) -> Arc<Set<D, L, P>> {
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