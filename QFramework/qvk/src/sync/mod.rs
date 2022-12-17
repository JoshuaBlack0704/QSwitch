use std::sync::{Arc, Mutex};

use ash::vk;

use crate::device::DeviceProvider;

pub mod semaphore;
pub struct Semaphore<D:DeviceProvider>{
    device: Arc<D>,
    semaphore: vk::Semaphore,
}

pub mod timelinesemaphore;
pub struct TimelineSemaphore<D:DeviceProvider>{
    device: Arc<D>,
    semaphore: vk::Semaphore,
    value: Mutex<(bool, u64)>,
}

pub mod fence;
pub struct Fence<D:DeviceProvider>{
    device: Arc<D>,
    fence: vk::Fence,
}
