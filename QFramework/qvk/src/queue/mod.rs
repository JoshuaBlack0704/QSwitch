use std::sync::Arc;

use ash::vk;

use crate::{command::CommandBufferStore, init::DeviceSource};
use crate::sync::FenceStore;
use crate::sync::SemaphoreStore;

pub mod submit;
pub trait SubmitInfoStore<C:CommandBufferStore + Clone>{
    fn info(&self) -> vk::SubmitInfo2;
    fn add_cmd(&mut self, cmd: &C);
    fn add_wait<S:SemaphoreStore>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2);
    fn add_signal<S:SemaphoreStore>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2);
}
pub struct SubmitSet<C:CommandBufferStore + Clone>{
    wait_semaphores: Vec<vk::SemaphoreSubmitInfo>,
    cmds: Vec<C>,
    live_cmds: Vec<vk::CommandBufferSubmitInfo>,
    signal_semaphores: Vec<vk::SemaphoreSubmitInfo>,
}


pub mod queue;
pub trait QueueStore{
    fn queue(&self) -> &vk::Queue;
}
pub trait QueueOps{
    fn submit<C:CommandBufferStore + Clone, S:SubmitInfoStore<C>, F:FenceStore>(&self, submits: &[S], fence: Option<&F>) -> Result<(), vk::Result>;
    ///Will create an internal fence to wait on the operation
    fn wait_submit<C:CommandBufferStore + Clone, S:SubmitInfoStore<C>>(&self, submits: &[S]) -> Result<(), vk::Result>;
    fn wait_idle(&self);
}
pub struct Queue<D:DeviceSource>{
    device: D,
    _queue_family: u32,
    queue: vk::Queue,
    
}




