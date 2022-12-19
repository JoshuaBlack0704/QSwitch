use std::sync::Arc;

use ash::vk;

use crate::{init::device::DeviceStore, command::CommandBufferStore};

pub mod submit;
pub struct SubmitSet<C:CommandBufferStore>{
    wait_semaphores: Vec<vk::SemaphoreSubmitInfo>,
    cmds: Vec<Arc<C>>,
    live_cmds: Vec<vk::CommandBufferSubmitInfo>,
    signal_semaphores: Vec<vk::SemaphoreSubmitInfo>,
}


pub mod queue;
pub struct Queue<D:DeviceStore>{
    device: Arc<D>,
    _queue_family: u32,
    queue: vk::Queue,
    
}