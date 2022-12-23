use std::sync::{Mutex, Arc};

use ash::vk;
use log::info;

use crate::init::DeviceStore;

pub mod semaphore;
pub trait SemaphoreFactory<S:SemaphoreStore>{
    fn create_semaphore(&self) -> S;
}
impl<D:DeviceStore + Clone> SemaphoreFactory<Arc<Semaphore<D>>> for D{
    fn create_semaphore(&self) -> Arc<Semaphore<D>> {
        let info = vk::SemaphoreCreateInfo::builder();
        let semaphore = unsafe{self.device().create_semaphore(&info, None).unwrap()};
        info!("Created semaphore {:?}", semaphore);
        Arc::new(Semaphore{
            device: self.clone(),
            semaphore,
        })
    }
}
pub trait SemaphoreStore{
    fn semaphore(&self) -> &vk::Semaphore;
    fn submit_info(&self, stage: vk::PipelineStageFlags2) -> vk::SemaphoreSubmitInfo;
}
pub struct Semaphore<D:DeviceStore>{
    device: D,
    semaphore: vk::Semaphore,
}

pub mod timelinesemaphore;
pub trait TimelineSemaphoreFactory<S:SemaphoreStore>{
    fn create_timeline_semaphore(&self, starting_value: u64) -> S;
}
impl<D:DeviceStore + Clone> TimelineSemaphoreFactory<Arc<TimelineSemaphore<D>>> for D{
    fn create_timeline_semaphore(&self, starting_value: u64) -> Arc<TimelineSemaphore<Self>> {
        let mut timeline_ext = vk::SemaphoreTypeCreateInfo::builder()
        .semaphore_type(vk::SemaphoreType::TIMELINE)
        .initial_value(starting_value);
        let info = vk::SemaphoreCreateInfo::builder()
        .push_next(&mut timeline_ext);

        let semaphore = unsafe{self.device().create_semaphore(&info, None).expect("Could not create semaphore")};
        info!("Created timeline semaphore {:?}", semaphore);
        Arc::new(
            TimelineSemaphore{
                device: self.clone(),
                semaphore,
                value: Mutex::new((false, starting_value)),
            }
        )
    }
}
pub struct TimelineSemaphore<D:DeviceStore>{
    device: D,
    semaphore: vk::Semaphore,
    value: Mutex<(bool, u64)>,
}

pub mod fence;
pub trait FenceFactory<F:FenceStore>{
    fn create_fence(&self, signaled: bool) -> F;
}
impl<D:DeviceStore + Clone> FenceFactory<Arc<Fence<D>>> for D{
    fn create_fence(&self, signaled: bool) -> Arc<Fence<Self>> {
        let mut info = vk::FenceCreateInfo::builder();
        if signaled{
            info = info.flags(vk::FenceCreateFlags::SIGNALED);
        }

        let fence = unsafe{self.device().create_fence(&info, None).unwrap()};
        info!("Created fence {:?}", fence);

        Arc::new(
            Fence{
                device: self.clone(),
                fence,
            }
        )
    }
}
pub trait FenceStore{
    fn fence(&self) -> &vk::Fence;
    fn wait(&self, timeout: Option<u64>);
    fn reset(&self);
}

pub struct Fence<D:DeviceStore>{
    device: D,
    fence: vk::Fence,
}

