use std::sync::{Arc, Mutex, MutexGuard};

use ash::vk;

use crate::init::DeviceStore;


// Layouts will simply be built by specifying bindings using method level generics and T::fn() syntax
pub mod descriptorlayout;
pub trait DescriptorLayoutBindingFactory {
    fn binding(&self) -> vk::DescriptorSetLayoutBinding;

}
pub trait DescriptorLayoutStore{
    fn layout(&self) -> vk::DescriptorSetLayout;
    fn writes(&self) -> MutexGuard<Vec<Arc<WriteHolder>>>;
    fn bindings(&self) -> MutexGuard<Vec<vk::DescriptorSetLayoutBinding>>;
}
pub struct DescriptorLayout<D:DeviceStore>{
    device: D,
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
    device: D,
    layout: L,
    _pool: P,
    writes: Vec<Arc<WriteHolder>>,
    set: vk::DescriptorSet,
}

pub mod pool;
pub trait DescriptorPoolStore{
    fn allocate_set<L:DescriptorLayoutStore>(&self, layout: &L) -> vk::DescriptorSet;
    fn pool(&self) -> vk::DescriptorPool;
}
pub struct Pool<D:DeviceStore>{
    device: D,
    pool: vk::DescriptorPool,
    
}






