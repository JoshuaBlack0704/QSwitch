use std::sync::Arc;

use ash::vk;

use crate::{device::DeviceProvider, sync::semaphore::SemaphoreProvider};


pub mod submit;
pub struct SubmitSet<'a, S:SemaphoreProvider>{
    wait_semaphores: Vec<S>,
    cmds: Vec<&'a vk::CommandBuffer>,
    signal_semaphores: Vec<S>,
}


pub mod queue;
pub struct Queue<D:DeviceProvider>{
    device: Arc<D>,
    _queue_family: u32,
    queue: vk::Queue,
    
}