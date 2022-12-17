use std::sync::Arc;

use ash::vk;

use crate::{device::{DeviceProvider, UsesDeviceProvider}, sync::{self, fence::FenceProvider}};

use super::{Queue, submit::SubmitInfoProvider};

pub trait QueueProvider{
    fn submit<S:SubmitInfoProvider, F:FenceProvider>(&self, submits: &[S], fence: Option<&Arc<F>>) -> Result<(), vk::Result>;
    ///Will create an internal fence to wait on the operation
    fn wait_submit<S:SubmitInfoProvider>(&self, submits: &[S]) -> Result<(), vk::Result>;
}

impl<D:DeviceProvider> Queue<D>{
    pub fn new(device_provider: &Arc<D>, flags: vk::QueueFlags) -> Option<Self>{
        let q = device_provider.get_queue(flags);
        match q{
            Some(q) => {
                return Some(Self{
                    device: device_provider.clone(),
                    queue_family: q.1,
                    queue: q.0,
                });
            },
            None => {
                None
            },
        }
    }
}

impl<D:DeviceProvider> UsesDeviceProvider<D> for Queue<D>{
    fn device_provider(&self) -> &Arc<D> {
        &self.device
    }
}

impl<D:DeviceProvider> QueueProvider for Queue<D>{
    fn submit<S:SubmitInfoProvider, F:FenceProvider>(&self, submits: &[S], fence: Option<&Arc<F>>) -> std::result::Result<(), ash::vk::Result> {
        let submits:Vec<vk::SubmitInfo> = submits.iter().map(|s| s.info()).collect();

        let device = self.device.device();

        unsafe{
            let mut _fence = vk::Fence::null();
            if let Some(f) = fence{
                _fence = *f.fence();
            }
            device.queue_submit(self.queue, &submits, _fence)
        }
    }

    fn wait_submit<S:SubmitInfoProvider>(&self, submits: &[S]) -> Result<(), vk::Result> {
        let fence = sync::Fence::new(self.device_provider(), false);
        let res = self.submit(submits, Some(&fence));
        fence.wait(None);
        res
    }
}