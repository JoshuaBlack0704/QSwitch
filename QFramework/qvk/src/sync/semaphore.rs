use std::sync::Arc;

use ash::vk;
use log::{info, debug};

use crate::device;

use super::Semaphore;

pub trait SemaphoreProvider{
    fn semaphore(&self) -> &vk::Semaphore;
}

impl<D:device::DeviceProvider> Semaphore<D>{
    pub fn new(device_provider: &Arc<D>) -> Arc<Semaphore<D>>{
        let info = vk::SemaphoreCreateInfo::builder();
        let semaphore = unsafe{device_provider.device().create_semaphore(&info, None).unwrap()};
        info!("Created semaphore {:?}", semaphore);
        Arc::new(Semaphore{
            device: device_provider.clone(),
            semaphore,
        })
    }
}

impl<D:device::DeviceProvider> Drop for Semaphore<D>{
    fn drop(&mut self) {
        debug!("Destroyed semaphore {:?}", self.semaphore);
        unsafe{
            self.device.device().destroy_semaphore(self.semaphore, None);
        }
    }
}

impl<D:device::DeviceProvider> SemaphoreProvider for Semaphore<D>{
    fn semaphore(&self) -> &vk::Semaphore {
        &self.semaphore
    }
}