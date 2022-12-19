use std::sync::{Arc, Mutex};

use ash::vk;
use log::{info, debug};

use crate::device::DeviceStore;

use super::{TimelineSemaphore, semaphore::SemaphoreStore};

impl<D:DeviceStore> TimelineSemaphore<D>{
    pub fn new(device_provider: &Arc<D>, starting_value: u64) -> Arc<TimelineSemaphore<D>> {
        let mut timeline_ext = vk::SemaphoreTypeCreateInfo::builder()
        .semaphore_type(vk::SemaphoreType::TIMELINE)
        .initial_value(starting_value);
        let info = vk::SemaphoreCreateInfo::builder()
        .push_next(&mut timeline_ext);

        let semaphore = unsafe{device_provider.device().create_semaphore(&info, None).expect("Could not create semaphore")};
        info!("Created timeline semaphore {:?}", semaphore);
        Arc::new(
            Self{
                device: device_provider.clone(),
                semaphore,
                value: Mutex::new((false, starting_value)),
            }
        )
    }

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

impl<D:DeviceStore> SemaphoreStore for TimelineSemaphore<D>{
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