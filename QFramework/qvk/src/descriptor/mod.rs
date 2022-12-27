use std::sync::{Mutex, MutexGuard};


use ash::vk;

use crate::init::DeviceSource;


// Layouts will simply be built by specifying bindings using method level generics and T::fn() syntax
pub mod descriptorlayout;
pub trait DescriptorLayoutFactory<W:WriteSource, L:DescriptorLayoutSource<W>>{
    fn create_descriptor_layout(&self, flags: Option<vk::DescriptorSetLayoutCreateFlags>) -> L;
}
pub trait DescriptorLayoutBindingFactory {
    fn binding(&self) -> vk::DescriptorSetLayoutBinding;
}
pub trait DescriptorLayoutSource<W:WriteSource>{
    fn layout(&self) -> vk::DescriptorSetLayout;
    fn writes(&self) -> MutexGuard<Vec<W>>;
    fn bindings(&self) -> MutexGuard<Vec<vk::DescriptorSetLayoutBinding>>;
}
pub struct DescriptorLayout<D:DeviceSource,W:WriteSource>{
    device: D,
    bindings: Mutex<Vec<vk::DescriptorSetLayoutBinding>>,
    writes: Mutex<Vec<W>>,
    flags: Option<vk::DescriptorSetLayoutCreateFlags>,
    layout: Mutex<Option<vk::DescriptorSetLayout>>,
}

pub mod writeholder;
pub trait WriteSource{
    ///Binding type is set by layout struct
    ///The write holder class automatically applies dst binding information
    ///and the set struct automatically fills dst set information
    fn update(&self, write: vk::WriteDescriptorSet); 
    fn needs_write(&self) -> bool;
    fn get_write(&self) -> vk::WriteDescriptorSet;
}
pub trait ApplyWriteFactory{
    fn apply<W:WriteSource>(&self, write: &W);
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
pub trait SetFactory<S:SetSource, W:WriteSource, L:DescriptorLayoutSource<W>>{
    fn create_set(&self, layout_provider: &L) -> S;
}
pub trait SetSource{
    fn update(&self);
}
#[allow(unused)]
pub struct Set<P:DescriptorPoolSource + DeviceSource, W:WriteSource>{
    pool: P,
    writes: Vec<W>,
    set: vk::DescriptorSet,
}

pub mod pool;
pub trait DescriptorPoolFactory<P:DescriptorPoolSource>{
    fn create_descriptor_pool<W:WriteSource, L:DescriptorLayoutSource<W>>(&self, layout_set_count: &[(&L, u32)], flags: Option<vk::DescriptorPoolCreateFlags>) -> P;
}
#[allow(unused)]
pub trait DescriptorPoolSource{
    fn allocate_set<W:WriteSource, L:DescriptorLayoutSource<W>>(&self, layout: &L) -> vk::DescriptorSet;
    fn pool(&self) -> vk::DescriptorPool;
}
pub struct Pool<D:DeviceSource>{
    device: D,
    pool: vk::DescriptorPool,
    
}






