use std::sync::Arc;

use ash::vk;

use crate::init::InstanceSource;
use crate::queue::{QueueSource, SubmitInfoSource};
use crate::sync::FenceSource;
use crate::{command::CommandBufferSource, init::DeviceSource, sync::FenceFactory};

use super::{Queue, QueueFactory, QueueOps};

impl<D: DeviceSource + Clone> QueueFactory<Arc<Queue<D>>> for D {
    fn create_queue(&self, flags: vk::QueueFlags) -> Option<Arc<Queue<D>>> {
        let device_source = self;
        let q = device_source.get_queue(flags);
        match q {
            Some(q) => {
                return Some(Arc::new(Queue {
                    device: device_source.clone(),
                    _queue_family: q.1,
                    queue: q.0,
                }));
            }
            None => None,
        }
    }
}

impl<D: DeviceSource + InstanceSource> InstanceSource for Arc<Queue<D>> {
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}

impl<D: DeviceSource> DeviceSource for Arc<Queue<D>> {
    fn device(&self) -> &ash::Device {
        self.device.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.device.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.device.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.device.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.device.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.device.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.device.host_memory_index()
    }
}

impl<D: DeviceSource + Clone> QueueOps for Arc<Queue<D>> {
    fn submit<C: CommandBufferSource + Clone, S: SubmitInfoSource<C>, F: FenceSource>(
        &self,
        submits: &[S],
        fence: Option<&F>,
    ) -> std::result::Result<(), ash::vk::Result> {
        let submits: Vec<vk::SubmitInfo2> = submits.iter().map(|s| s.info()).collect();

        let device = self.device.device();

        unsafe {
            let mut _fence = vk::Fence::null();
            if let Some(f) = fence {
                _fence = *f.fence();
            }
            device.queue_submit2(self.queue, &submits, _fence)
        }
    }
    fn wait_submit<C: CommandBufferSource + Clone, S: SubmitInfoSource<C>>(
        &self,
        submits: &[S],
    ) -> Result<(), vk::Result> {
        let fence = self.create_fence(false);
        let res = self.submit(submits, Some(&fence));
        fence.wait(None);
        res
    }
    fn wait_idle(&self) {
        unsafe { self.device.device().queue_wait_idle(self.queue).unwrap() };
    }
}

impl<D: DeviceSource> QueueSource for Arc<Queue<D>> {
    fn queue(&self) -> &vk::Queue {
        &self.queue
    }
}
