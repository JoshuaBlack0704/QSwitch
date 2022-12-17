use std::sync::Arc;

use ash::vk;
use log::{info, debug};

use crate::device::{DeviceProvider, UsesDeviceProvider};

use super::Semaphore;

pub trait SemaphoreProvider{
    fn semaphore(&self) -> &vk::Semaphore;
    fn submit_info(&self, stage: vk::PipelineStageFlags2) -> vk::SemaphoreSubmitInfo;
}

impl<D:DeviceProvider> Semaphore<D>{
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

impl<D:DeviceProvider> Drop for Semaphore<D>{
    fn drop(&mut self) {
        debug!("Destroyed semaphore {:?}", self.semaphore);
        unsafe{
            self.device.device().destroy_semaphore(self.semaphore, None);
        }
    }
}

impl<D:DeviceProvider> SemaphoreProvider for Semaphore<D>{
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

impl <D:DeviceProvider> UsesDeviceProvider<D> for Semaphore<D>{
    fn device_provider(&self) -> &Arc<D> {
        &self.device
    }
}