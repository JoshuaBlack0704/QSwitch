use std::sync::{Mutex, MutexGuard};


use ash::vk;

use crate::init::DeviceStore;


// Layouts will simply be built by specifying bindings using method level generics and T::fn() syntax
pub mod descriptorlayout;
pub trait DescriptorLayoutBindingFactory {
    fn binding(&self) -> vk::DescriptorSetLayoutBinding;

}
pub trait DescriptorLayoutStore<W:WriteStore>{
    fn layout(&self) -> vk::DescriptorSetLayout;
    fn writes(&self) -> MutexGuard<Vec<W>>;
    fn bindings(&self) -> MutexGuard<Vec<vk::DescriptorSetLayoutBinding>>;
}
pub struct DescriptorLayout<D:DeviceStore,W:WriteStore>{
    device: D,
    bindings: Mutex<Vec<vk::DescriptorSetLayoutBinding>>,
    writes: Mutex<Vec<W>>,
    flags: Option<vk::DescriptorSetLayoutCreateFlags>,
    layout: Mutex<Option<vk::DescriptorSetLayout>>,
}

pub mod writeholder;
pub trait WriteStore{
    ///Binding type is set by layout struct
    ///The write holder class automatically applies dst binding information
    ///and the set struct automatically fills dst set information
    fn update(&self, write: vk::WriteDescriptorSet); 
    fn needs_write(&self) -> bool;
    fn get_write(&self) -> vk::WriteDescriptorSet;
}
pub trait ApplyWriteFactory{
    fn apply<W:WriteStore>(&self, write: &W);
}
#[allow(unused)]
pub struct WriteHolder{
    needs_update: Mutex<bool>,
    ty: vk::DescriptorType,
    dst_binding: u32,
    write: Mutex<vk::WriteDescriptorSet>,
}
 
// Upon creation the descriptor set will make available a set of arc mutexed writes that can be given to other structs for updates
pub mod set;

#[allow(unused)]
pub struct Set<D:DeviceStore, L:DescriptorLayoutStore<W>,P:DescriptorPoolStore, W:WriteStore>{
    device: D,
    layout: L,
    _pool: P,
    writes: Vec<W>,
    set: vk::DescriptorSet,
}

pub mod pool;
#[allow(unused)]
pub trait DescriptorPoolStore{
    fn allocate_set<W:WriteStore, L:DescriptorLayoutStore<W>>(&self, layout: &L) -> vk::DescriptorSet;
    fn pool(&self) -> vk::DescriptorPool;
}
pub struct Pool<D:DeviceStore>{
    device: D,
    pool: vk::DescriptorPool,
    
}






