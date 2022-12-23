use std::sync::Arc;

use ash::vk;
use log::debug;

use crate::init::DeviceStore;
use crate::sync::SemaphoreStore;

use super::TimelineSemaphore;

#[allow(unused)]
impl<D:DeviceStore + Clone> TimelineSemaphore<D>{
    fn increment(&self){
        let mut lock = self.value.lock().unwrap();
        let (frozen, mut value) = *lock;
        if !frozen{
            value += 1;
        }
        *lock = (frozen, value);
    }
    fn freeze(&self){
        let mut lock = self.value.lock().unwrap();
        let (mut frozen, value) = *lock;
        frozen = true;
        *lock = (frozen, value);
    }
    fn thaw(&self){
        let mut lock = self.value.lock().unwrap();
        let (mut frozen, value) = *lock;
        frozen = false;
        *lock = (frozen, value);
    }
    fn value(&self) -> u64 {
        self.value.lock().unwrap().1
    }
}

impl<D:DeviceStore> SemaphoreStore for Arc<TimelineSemaphore<D>>{
    fn semaphore(&self) -> &vk::Semaphore {
        &self.semaphore
    }

    fn submit_info(&self, stage: vk::PipelineStageFlags2) -> vk::SemaphoreSubmitInfo {
        let lock = self.value.lock().unwrap();
        vk::SemaphoreSubmitInfo::builder()
        .semaphore(self.semaphore)
        .value(lock.1)
        .stage_mask(stage)
        .device_index(0)
        .build()
    }
}



impl<D:DeviceStore> Drop for TimelineSemaphore<D>{
    fn drop(&mut self) {
        debug!("Destroyed timeline semaphore {:?}", self.semaphore);
        unsafe{
            self.device.device().destroy_semaphore(self.semaphore, None);
        }
    }
}