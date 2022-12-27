use std::sync::Arc;

use ash::vk;
use log::{debug, info};

use crate::init::{DeviceSource, InstanceSource};
use crate::sync::FenceSource;

use super::{Fence, FenceFactory};

impl<D:DeviceSource + Clone> FenceFactory<Arc<Fence<D>>> for D{
    fn create_fence(&self, signaled: bool) -> Arc<Fence<D>> {
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

impl<D:DeviceSource + InstanceSource> InstanceSource for Arc<Fence<D>>{
    
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}

impl<D:DeviceSource> DeviceSource for Arc<Fence<D>>{
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
