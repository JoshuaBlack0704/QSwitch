use std::sync::Arc;

use ash::vk;

use crate::{sync::{fence::FenceStore, self}, init::device::{DeviceStore, UsesDeviceStore}, command::CommandBufferStore};

use super::{Queue, submit::SubmitInfoStore};

pub trait QueueStore{
    fn submit<C:CommandBufferStore, S:SubmitInfoStore<C>, F:FenceStore>(&self, submits: &[S], fence: Option<&Arc<F>>) -> Result<(), vk::Result>;
    ///Will create an internal fence to wait on the operation
    fn wait_submit<C:CommandBufferStore, S:SubmitInfoStore<C>>(&self, submits: &[S]) -> Result<(), vk::Result>;
    fn queue(&self) -> &vk::Queue;
    fn wait_idle(&self);
}

impl<D:DeviceStore> Queue<D>{
    pub fn new(device_provider: &Arc<D>, flags: vk::QueueFlags) -> Option<Arc<Self>>{
        let q = device_provider.get_queue(flags);
        match q{
            Some(q) => {
                return Some(
                Arc::new(
                    Self{
                        device: device_provider.clone(),
                        _queue_family: q.1,
                        queue: q.0,
                    }                       
                )
                );
            },
            None => {
                None
            },
        }
    }
}

impl<D:DeviceStore> UsesDeviceStore<D> for Queue<D>{
    fn device_provider(&self) -> &Arc<D> {
        &self.device
    }
}

impl<D:DeviceStore> QueueStore for Queue<D>{
    fn submit<C:CommandBufferStore, S:SubmitInfoStore<C>, F:FenceStore>(&self, submits: &[S], fence: Option<&Arc<F>>) -> std::result::Result<(), ash::vk::Result> {
        let submits:Vec<vk::SubmitInfo2> = submits.iter().map(|s| s.info()).collect();

        let device = self.device.device();

        unsafe{
            let mut _fence = vk::Fence::null();
            if let Some(f) = fence{
                _fence = *f.fence();
            }
            device.queue_submit2(self.queue, &submits, _fence)
        }
    }

    fn wait_submit<C:CommandBufferStore, S:SubmitInfoStore<C>>(&self, submits: &[S]) -> Result<(), vk::Result> {
        let fence = sync::Fence::new(self.device_provider(), false);
        let res = self.submit(submits, Some(&fence));
        fence.wait(None);
        res
    }

    fn queue(&self) -> &vk::Queue {
        &self.queue
    }

    fn wait_idle(&self) {
        unsafe{self.device.device().queue_wait_idle(self.queue).unwrap()};
    }
}