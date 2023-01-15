use std::sync::Mutex;

use ash::vk;

use crate::init::DeviceSource;

pub mod semaphore;
pub trait SemaphoreFactory<S: SemaphoreSource> {
    fn create_semaphore(&self) -> S;
}
pub trait SemaphoreSource {
    fn semaphore(&self) -> &vk::Semaphore;
    fn submit_info(&self, stage: vk::PipelineStageFlags2) -> vk::SemaphoreSubmitInfo;
}
pub struct Semaphore<D: DeviceSource> {
    device: D,
    semaphore: vk::Semaphore,
}

pub mod timelinesemaphore;
pub trait TimelineSemaphoreFactory<S: SemaphoreSource> {
    fn create_timeline_semaphore(&self, starting_value: u64) -> S;
}
pub struct TimelineSemaphore<D: DeviceSource> {
    device: D,
    semaphore: vk::Semaphore,
    value: Mutex<(bool, u64)>,
}

pub mod fence;
pub trait FenceFactory<F: FenceSource> {
    fn create_fence(&self, signaled: bool) -> F;
}
pub trait FenceSource {
    fn fence(&self) -> &vk::Fence;
    fn wait(&self, timeout: Option<u64>);
    fn reset(&self);
}
pub struct Fence<D: DeviceSource> {
    device: D,
    fence: vk::Fence,
}
