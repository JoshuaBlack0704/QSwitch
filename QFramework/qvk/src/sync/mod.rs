use std::sync::{Arc, Mutex};

use ash::vk;

use crate::init::DeviceStore;

pub mod semaphore;
pub struct Semaphore<D:DeviceStore>{
    device: Arc<D>,
    semaphore: vk::Semaphore,
}

pub mod timelinesemaphore;
pub struct TimelineSemaphore<D:DeviceStore>{
    device: Arc<D>,
    semaphore: vk::Semaphore,
    value: Mutex<(bool, u64)>,
}

pub mod fence;
pub struct Fence<D:DeviceStore>{
    device: Arc<D>,
    fence: vk::Fence,
}

pub trait FenceStore{
    fn fence(&self) -> &vk::Fence;
    fn wait(&self, timeout: Option<u64>);
    fn reset(&self);
}

pub trait SemaphoreStore{
    fn semaphore(&self) -> &vk::Semaphore;
    fn submit_info(&self, stage: vk::PipelineStageFlags2) -> vk::SemaphoreSubmitInfo;
}
