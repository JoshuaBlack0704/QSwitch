use std::sync::{Arc, Mutex};

use ash::vk;

use crate::device::DeviceProvider;

use self::{descriptorlayout::DescriptorLayoutProvider, pool::DescriptorPoolProvider};

// Layouts will simply be built by specifying bindings using method level generics and T::fn() syntax
pub mod descriptorlayout;
pub struct DescriptorLayout<D:DeviceProvider>{
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
pub struct Set<D:DeviceProvider,L:DescriptorLayoutProvider,P:DescriptorPoolProvider>{
    device: Arc<D>,
    layout: Arc<L>,
    _pool: Arc<P>,
    writes: Vec<Arc<WriteHolder>>,
    set: vk::DescriptorSet,
}

pub mod pool;
pub struct Pool<D:DeviceProvider>{
    device: Arc<D>,
    pool: vk::DescriptorPool,
    
}