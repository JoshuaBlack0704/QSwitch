use std::sync::{Arc, Mutex};

use ash::vk;

use crate::device::DeviceStore;

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
