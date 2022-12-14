use std::sync::Arc;

use ash::vk;

use crate::device;

pub mod semaphore;
pub struct Semaphore<D:device::DeviceProvider>{
    device: Arc<D>,
    semaphore: vk::Semaphore,
}
pub struct TimelineSemaphore<D:device::DeviceProvider>{
    _device: Arc<D>,
    _semaphore: vk::Semaphore,
}

pub mod fence;
pub struct Fence<D:device::DeviceProvider>{
    device: Arc<D>,
    fence: vk::Fence,
}
