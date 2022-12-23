use std::sync::Arc;

use ash::vk;
use log::{debug, info};

use crate::init::{DeviceSource, DeviceSupplier};
use crate::sync::SemaphoreStore;

use super::Semaphore;

impl<D:DeviceSource + Clone> Semaphore<D>{
    pub fn new(device_provider: &D) -> Arc<Semaphore<D>>{
        let info = vk::SemaphoreCreateInfo::builder();
        let semaphore = unsafe{device_provider.device().create_semaphore(&info, None).unwrap()};
        info!("Created semaphore {:?}", semaphore);
        Arc::new(Semaphore{
            device: device_provider.clone(),
            semaphore,
        })
    }
}

impl<D:DeviceSource> Drop for Semaphore<D>{
    fn drop(&mut self) {
        debug!("Destroyed semaphore {:?}", self.semaphore);
        unsafe{
            self.device.device().destroy_semaphore(self.semaphore, None);
        }
    }
}

impl<D:DeviceSource> SemaphoreStore for Arc<Semaphore<D>>{
    fn semaphore(&self) -> &vk::Semaphore {
        &self.semaphore
    }

    fn submit_info(&self, stage: vk::PipelineStageFlags2) -> vk::SemaphoreSubmitInfo {
        vk::SemaphoreSubmitInfo::builder()
        .semaphore(self.semaphore)
        .value(0)
        .stage_mask(stage)
        .device_index(0)
        .build()
        
    }
}

impl <D:DeviceSource> DeviceSupplier<D> for Arc<Semaphore<D>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}