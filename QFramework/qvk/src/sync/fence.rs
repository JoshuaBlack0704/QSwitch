use std::sync::Arc;

use ash::vk;
use log::{debug, info};

use crate::init::{DeviceSource, DeviceSupplier};
use crate::sync::FenceSource;

use super::{Fence, FenceFactory};

impl<D:DeviceSource + Clone, DS: DeviceSupplier<D>> FenceFactory<Arc<Fence<D>>> for DS{
    fn create_fence(&self, signaled: bool) -> Arc<Fence<D>> {
        let mut info = vk::FenceCreateInfo::builder();
        if signaled{
            info = info.flags(vk::FenceCreateFlags::SIGNALED);
        }

        let fence = unsafe{self.device_provider().device().create_fence(&info, None).unwrap()};
        info!("Created fence {:?}", fence);

        Arc::new(
            Fence{
                device: self.device_provider().clone(),
                fence,
            }
        )
    }
}

impl<D:DeviceSource> FenceSource for Arc<Fence<D>>{
    fn fence(&self) -> &vk::Fence {
        &self.fence
    }

    fn wait(&self, timeout: Option<u64>) {
        let fence = [self.fence];
        unsafe{
            if let Some(timeout) = timeout{
                self.device.device().wait_for_fences(&fence, true, timeout).expect("Could not wait on fence");
            }
            else{
                self.device.device().wait_for_fences(&fence, true, u64::MAX).expect("Could not wait on fence");
            }
        }
    }

    fn reset(&self) {
        unsafe{
            let fence = [self.fence];
            self.device.device().reset_fences(&fence).unwrap();
        }
    }
}

impl<D:DeviceSource> Drop for Fence<D>{
    fn drop(&mut self) {
        debug!{"Destroyed fence {:?}", self.fence};
        unsafe{
            self.device.device().destroy_fence(self.fence, None);
        }
    }
}

impl<D:DeviceSource> DeviceSupplier<D> for Arc<Fence<D>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}
