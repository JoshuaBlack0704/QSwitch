use std::sync::{Arc, Mutex};

use ash::vk;

use crate::init::DeviceStore;

use self::{commandpool::CommandPoolSettingsStore, commandset::CommandSetSettingsStore};

pub mod commandpool;
pub trait CommandPoolStore{
    fn cmdpool(&self) -> &vk::CommandPool;
    fn reset_cmdpool(&self);
}
pub struct CommandPool<D: DeviceStore, S: CommandPoolSettingsStore>{
    device: Arc<D>,
    settings: S,
    command_pool: vk::CommandPool,
}

pub mod commandset;
pub struct CommandSet<D: DeviceStore, P: CommandPoolStore, S: CommandSetSettingsStore>{
    device: Arc<D>,
    cmdpool: Arc<P>,
    settings: S,
    cmds: Mutex<Vec<Arc<CommandBuffer<D>>>>,
}

pub mod commandbuffer;
pub trait CommandBufferFactory<D:DeviceStore>{
    fn next_cmd(&self) -> Arc<CommandBuffer<D>>;
    fn reset_cmd(&self, cmd: &Arc<CommandBuffer<D>>);
}
pub trait BindPipelineFactory{
    fn layout(&self) -> vk::PipelineLayout;
    fn bind_point(&self) -> vk::PipelineBindPoint;
    fn pipeline(&self) -> vk::Pipeline;
}
pub trait BindSetFactory{
    fn bind_point(&self) -> vk::PipelineBindPoint;
    fn set(&self) -> vk::DescriptorSet;
    fn dynamic_offsets(&self) -> Option<Vec<u32>>;
    
}

pub trait CommandBufferStore{
    fn cmd(&self) -> vk::CommandBuffer;
    fn begin(&self, info: Option<vk::CommandBufferBeginInfo>) -> Result<(), vk::Result>;
    fn end(&self) -> Result<(), vk::Result>;
    fn barrier(&self, info: vk::DependencyInfo);
    fn bind_pipeline<BP: BindPipelineFactory>(&self, pipeline: &Arc<BP>);
    fn bind_set<BP:BindPipelineFactory, BS: BindSetFactory>(&self, set: &Arc<BS>, set_index: u32, pipeline: &Arc<BP>);
}
pub struct CommandBuffer<D:DeviceStore>{
    device: Arc<D>,
    cmd: vk::CommandBuffer,
}




