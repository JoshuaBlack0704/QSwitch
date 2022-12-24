use std::sync::Arc;

use ash::vk;

use crate::{command::CommandBufferStore, init::{DeviceSource, DeviceSupplier}, sync::FenceFactory};
use crate::queue::{QueueSource, SubmitInfoSource};
use crate::sync::FenceSource;

use super::{Queue, QueueOps, QueueFactory};

impl<D:DeviceSource + Clone, DS:DeviceSupplier<D>> QueueFactory<Arc<Queue<D>>> for DS{
    fn create_queue(&self, flags: vk::QueueFlags) -> Option<Arc<Queue<D>>> {
        let device_source = self.device_provider();
        let q = device_source.get_queue(flags);
        match q{
            Some(q) => {
                return Some(
                Arc::new(
                    Queue{
                        device: device_source.clone(),
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
    fn submit<C:CommandBufferStore + Clone, S:SubmitInfoSource<C>, F:FenceSource>(&self, submits: &[S], fence: Option<&F>) -> std::result::Result<(), ash::vk::Result> {
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
    fn wait_submit<C:CommandBufferStore + Clone, S:SubmitInfoSource<C>>(&self, submits: &[S]) -> Result<(), vk::Result> {
        let fence = self.create_fence(false);
        let res = self.submit(submits, Some(&fence));
        fence.wait(None);
        res
    }
    fn wait_idle(&self) {
        unsafe{self.device.device().queue_wait_idle(self.queue).unwrap()};
    }
}

impl<D:DeviceSource> QueueSource for Arc<Queue<D>>{

    fn queue(&self) -> &vk::Queue {
        &self.queue
    }

}