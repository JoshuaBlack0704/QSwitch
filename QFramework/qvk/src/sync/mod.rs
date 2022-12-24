use std::sync::Mutex;

use ash::vk;

use crate::init::DeviceSource;

pub mod semaphore;
pub trait SemaphoreFactory<S:SemaphoreStore>{
    fn create_semaphore(&self) -> S;
}
pub struct Semaphore<D:DeviceSource>{
    device: D,
    semaphore: vk::Semaphore,
}

pub mod timelinesemaphore;
pub trait TimelineSemaphoreFactory<S:SemaphoreStore>{
    fn create_timeline_semaphore(&self, starting_value: u32) -> S;
}
pub struct TimelineSemaphore<D:DeviceSource>{
    device: D,
    semaphore: vk::Semaphore,
    value: Mutex<(bool, u64)>,
}

pub mod fence;
pub struct Fence<D:DeviceSource>{
    device: D,
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
