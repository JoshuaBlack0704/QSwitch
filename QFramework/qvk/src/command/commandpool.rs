use std::sync::Arc;

use ash::vk::{self, CommandPoolCreateFlags};
use log::debug;
use crate::command::CommandPoolStore;

use crate::init::{DeviceStore, DeviceSupplier};
use super::{CommandPool, CommandPoolOps};

#[derive(Clone)]
pub struct SettingsStore{
    pub queue_family_index: u32,
    pub create_flags: Option<CommandPoolCreateFlags>,
    pub reset_flags: Option<vk::CommandPoolResetFlags>,
}

impl<D:DeviceStore> CommandPoolOps for Arc<CommandPool<D>>{
    fn reset_cmdpool(&self) {
        match self.reset_flags{
            Some(f) => {
                unsafe{self.device.device().reset_command_pool(self.command_pool, f)}.expect("Could not reset command pool");
            },
            None => {
                unsafe{self.device.device().reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty())}.expect("Could not reset command pool");
            },
        }
    }
}

impl <D: DeviceStore> CommandPoolStore for Arc<CommandPool<D>>{
    fn cmdpool(&self) -> &vk::CommandPool {
        &self.command_pool
    }
}

impl<D:DeviceStore> DeviceSupplier<D> for Arc<CommandPool<D>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}

impl<D: DeviceStore> Drop for CommandPool<D>{
    fn drop(&mut self) {
        debug!("Destroyed command pool {:?}", self.command_pool);
        unsafe{
            self.device.device().destroy_command_pool(self.command_pool, None);
        }
    }
}

impl SettingsStore{
    pub fn new(queue_family_index: u32) -> SettingsStore {
        SettingsStore{ queue_family_index, create_flags: None, reset_flags: None }
    }
    pub fn set_create_flags(&mut self, flags: CommandPoolCreateFlags){
        self.create_flags = Some(flags);
    }
    pub fn set_reset_flags(&mut self, flags: vk::CommandPoolResetFlags){
        self.reset_flags = Some(flags);
    }
}

impl<D: DeviceStore> DeviceSupplier<D> for CommandPool<D>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}
