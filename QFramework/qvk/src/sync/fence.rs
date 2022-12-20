use std::sync::Arc;

use ash::vk;
use log::{debug, info};

use crate::init::{DeviceStore, InternalDeviceStore};
use crate::sync::FenceStore;

use super::Fence;

impl<D:DeviceStore + Clone> Fence<D>{
    pub fn new(device_provider: &D, signaled: bool) -> Arc<Fence<D>> {
        let mut info = vk::FenceCreateInfo::builder();
        if signaled{
            info = info.flags(vk::FenceCreateFlags::SIGNALED);
        }

        let fence = unsafe{device_provider.device().create_fence(&info, None).unwrap()};
        info!("Created fence {:?}", fence);

        Arc::new(
            Fence{
                device: device_provider.clone(),
                fence,
            }
        )
    }
}

impl<D:DeviceStore> FenceStore for Arc<Fence<D>>{
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

impl<D:DeviceStore> Drop for Fence<D>{
    fn drop(&mut self) {
        debug!{"Destroyed fence {:?}", self.fence};
        unsafe{
            self.device.device().destroy_fence(self.fence, None);
        }
    }
}

impl<D:DeviceStore> InternalDeviceStore<D> for Arc<Fence<D>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}
