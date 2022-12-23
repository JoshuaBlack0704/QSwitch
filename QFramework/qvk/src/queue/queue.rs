use std::sync::Arc;

use ash::vk;

use crate::{command::CommandBufferStore, init::{DeviceSource, DeviceSupplier}, sync};
use crate::queue::{QueueStore, SubmitInfoStore};
use crate::sync::FenceStore;

use super::{Queue, QueueOps};

impl<D:DeviceSource + Clone> Queue<D>{
    pub fn new(device_provider: &D, flags: vk::QueueFlags) -> Option<Arc<Self>>{
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

impl<D:DeviceSource> DeviceSupplier<D> for Arc<Queue<D>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}

impl<D:DeviceSource + Clone> QueueOps for Arc<Queue<D>>{
    fn submit<C:CommandBufferStore + Clone, S:SubmitInfoStore<C>, F:FenceStore>(&self, submits: &[S], fence: Option<&F>) -> std::result::Result<(), ash::vk::Result> {
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
    fn wait_submit<C:CommandBufferStore + Clone, S:SubmitInfoStore<C>>(&self, submits: &[S]) -> Result<(), vk::Result> {
        let fence = sync::Fence::new(self.device_provider(), false);
        let res = self.submit(submits, Some(&fence));
        fence.wait(None);
        res
    }
    fn wait_idle(&self) {
        unsafe{self.device.device().queue_wait_idle(self.queue).unwrap()};
    }
}

impl<D:DeviceSource> QueueStore for Arc<Queue<D>>{

    fn queue(&self) -> &vk::Queue {
        &self.queue
    }

}