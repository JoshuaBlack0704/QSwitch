use std::sync::{Arc, Mutex};

use ash::vk;
use log::{debug, info};

use crate::init::{DeviceSource, DeviceSupplier};
use crate::sync::SemaphoreSource;

use super::{TimelineSemaphore, TimelineSemaphoreFactory};

impl<D:DeviceSource + Clone, DS:DeviceSupplier<D>> TimelineSemaphoreFactory<Arc<TimelineSemaphore<D>>> for DS{
    fn create_timeline_semaphore(&self, starting_value: u64) -> Arc<TimelineSemaphore<D>> {
        let mut timeline_ext = vk::SemaphoreTypeCreateInfo::builder()
        .semaphore_type(vk::SemaphoreType::TIMELINE)
        .initial_value(starting_value);
        let info = vk::SemaphoreCreateInfo::builder()
        .push_next(&mut timeline_ext);

        let semaphore = unsafe{self.device_provider().device().create_semaphore(&info, None).expect("Could not create semaphore")};
        info!("Created timeline semaphore {:?}", semaphore);
        Arc::new(
            TimelineSemaphore{
                device: self.device_provider().clone(),
                semaphore,
                value: Mutex::new((false, starting_value)),
            }
        )
    }
}

#[allow(unused)]
impl<D:DeviceSource + Clone> TimelineSemaphore<D>{
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

impl<D:DeviceSource> SemaphoreSource for Arc<TimelineSemaphore<D>>{
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



impl<D:DeviceSource> Drop for TimelineSemaphore<D>{
    fn drop(&mut self) {
        debug!("Destroyed timeline semaphore {:?}", self.semaphore);
        unsafe{
            self.device.device().destroy_semaphore(self.semaphore, None);
        }
    }
}