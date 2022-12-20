use std::sync::Arc;

use ash::vk;

use crate::{command::CommandBufferStore, init::DeviceStore};
use crate::sync::FenceStore;
use crate::sync::SemaphoreStore;

pub mod submit;
pub trait SubmitInfoStore<C:CommandBufferStore>{
    fn info(&self) -> vk::SubmitInfo2;
    fn add_cmd(&mut self, cmd: &Arc<C>);
    fn add_wait<S:SemaphoreStore>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2);
    fn add_signal<S:SemaphoreStore>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2);
}
pub struct SubmitSet<C:CommandBufferStore>{
    wait_semaphores: Vec<vk::SemaphoreSubmitInfo>,
    cmds: Vec<Arc<C>>,
    live_cmds: Vec<vk::CommandBufferSubmitInfo>,
    signal_semaphores: Vec<vk::SemaphoreSubmitInfo>,
}


pub mod queue;
pub trait QueueStore{
    fn submit<C:CommandBufferStore, S:SubmitInfoStore<C>, F:FenceStore>(&self, submits: &[S], fence: Option<&Arc<F>>) -> Result<(), vk::Result>;
    ///Will create an internal fence to wait on the operation
    fn wait_submit<C:CommandBufferStore, S:SubmitInfoStore<C>>(&self, submits: &[S]) -> Result<(), vk::Result>;
    fn queue(&self) -> &vk::Queue;
    fn wait_idle(&self);
}
pub struct Queue<D:DeviceStore>{
    device: Arc<D>,
    _queue_family: u32,
    queue: vk::Queue,
    
}




