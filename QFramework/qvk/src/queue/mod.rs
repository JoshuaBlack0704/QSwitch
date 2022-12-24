use std::sync::Arc;

use ash::vk;

use crate::{command::CommandBufferSource, init::DeviceSource};
use crate::sync::FenceSource;
use crate::sync::SemaphoreSource;

pub mod submit;
pub trait SubmitInfoSource<C:CommandBufferSource + Clone>{
    fn info(&self) -> vk::SubmitInfo2;
    fn add_cmd(&mut self, cmd: &C);
    fn add_wait<S:SemaphoreSource>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2);
    fn add_signal<S:SemaphoreSource>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2);
}
pub struct SubmitSet<C:CommandBufferSource + Clone>{
    wait_semaphores: Vec<vk::SemaphoreSubmitInfo>,
    cmds: Vec<C>,
    live_cmds: Vec<vk::CommandBufferSubmitInfo>,
    signal_semaphores: Vec<vk::SemaphoreSubmitInfo>,
}


pub mod queue;
pub trait QueueFactory<Q:QueueSource>{
    fn create_queue(&self, flags: vk::QueueFlags) -> Option<Q>;
}
pub trait QueueSource{
    fn queue(&self) -> &vk::Queue;
}
pub trait QueueOps{
    fn submit<C:CommandBufferSource + Clone, S:SubmitInfoSource<C>, F:FenceSource>(&self, submits: &[S], fence: Option<&F>) -> Result<(), vk::Result>;
    ///Will create an internal fence to wait on the operation
    fn wait_submit<C:CommandBufferSource + Clone, S:SubmitInfoSource<C>>(&self, submits: &[S]) -> Result<(), vk::Result>;
    fn wait_idle(&self);
}
pub struct Queue<D:DeviceSource>{
    device: D,
    _queue_family: u32,
    queue: vk::Queue,
    
}




