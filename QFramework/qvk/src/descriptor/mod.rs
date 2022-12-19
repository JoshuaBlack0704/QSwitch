use std::sync::{Arc, Mutex};

use ash::vk;

use crate::device::DeviceStore;

use self::{descriptorlayout::DescriptorLayoutStore, pool::DescriptorPoolStore};

// Layouts will simply be built by specifying bindings using method level generics and T::fn() syntax
pub mod descriptorlayout;
pub struct DescriptorLayout<D:DeviceStore>{
    device: Arc<D>,
    bindings: Mutex<Vec<vk::DescriptorSetLayoutBinding>>,
    writes: Mutex<Vec<Arc<WriteHolder>>>,
    flags: Option<vk::DescriptorSetLayoutCreateFlags>,
    layout: Mutex<Option<vk::DescriptorSetLayout>>,
}

pub mod writeholder;
pub struct WriteHolder{
    write: Mutex<vk::WriteDescriptorSet>,
}
 
// Upon creation the descriptor set will make available a set of arc mutexed writes that can be given to other structs for updates
pub mod set;
pub struct Set<D:DeviceStore,L:DescriptorLayoutStore,P:DescriptorPoolStore>{
    device: Arc<D>,
    layout: Arc<L>,
    _pool: Arc<P>,
    writes: Vec<Arc<WriteHolder>>,
    set: vk::DescriptorSet,
}

pub mod pool;
pub struct Pool<D:DeviceStore>{
    device: Arc<D>,
    pool: vk::DescriptorPool,
    
}