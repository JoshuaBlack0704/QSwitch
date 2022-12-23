use std::sync::Arc;

use ash::vk;
use log::debug;

use crate::init::{DeviceStore, DeviceSupplier};
use crate::sync::SemaphoreStore;

use super::Semaphore;


impl<D:DeviceStore + Clone> Semaphore<D>{
}

impl<D:DeviceStore> Drop for Semaphore<D>{
    fn drop(&mut self) {
        debug!("Destroyed semaphore {:?}", self.semaphore);
        unsafe{
            self.device.device().destroy_semaphore(self.semaphore, None);
        }
    }
}

impl<D:DeviceStore> SemaphoreStore for Arc<Semaphore<D>>{
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

impl <D:DeviceStore> DeviceSupplier<D> for Arc<Semaphore<D>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}