use std::sync::Arc;

use ash::vk::{self, CommandPoolCreateFlags, CommandPoolCreateInfo};
use log::{debug, info};
use crate::command::CommandPoolStore;

use crate::init::{DeviceSource, DeviceSupplier};
use super::{CommandPool, CommandPoolOps};


pub trait CommandPoolSettingsStore{
    fn queue_family_index(&self) -> u32;
    fn reset_flags(&self) -> Option<vk::CommandPoolResetFlags>;
    fn create_flags(&self) -> Option<CommandPoolCreateFlags>;
}

#[derive(Clone)]
pub struct SettingsStore{
    pub queue_family_index: u32,
    pub create_flags: Option<CommandPoolCreateFlags>,
    pub reset_flags: Option<vk::CommandPoolResetFlags>,
}

impl<D: DeviceSource + Clone, S: CommandPoolSettingsStore + Clone> CommandPool<D,S>{
    pub fn new(settings: &S, device_provider: &D) -> Result<Arc<CommandPool<D,S>>, vk::Result>{
        
        let mut cmdpool_cinfo = CommandPoolCreateInfo::builder();
        cmdpool_cinfo = cmdpool_cinfo.queue_family_index(settings.queue_family_index());
        if let Some(flags) = settings.create_flags(){
            cmdpool_cinfo = cmdpool_cinfo.flags(flags);
        }
        
        let command_pool = unsafe{device_provider.device().create_command_pool(&cmdpool_cinfo, None)};
        
        match command_pool{
            Ok(pool) => {
                info!("Created command pool {:?}", pool);
                return Ok(Arc::new(CommandPool{ device: device_provider.clone(), settings: settings.clone(), command_pool: pool}));
            },
            Err(res) => {
                return Err(res);
            },
        }
        
    }
}

impl<D:DeviceSource, S:CommandPoolSettingsStore> CommandPoolOps for Arc<CommandPool<D,S>>{
    fn reset_cmdpool(&self) {
        match self.settings.reset_flags(){
            Some(f) => {
                unsafe{self.device.device().reset_command_pool(self.command_pool, f)}.expect("Could not reset command pool");
            },
            None => {
                unsafe{self.device.device().reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty())}.expect("Could not reset command pool");
            },
        }
    }
}

impl <D: DeviceSource, S: CommandPoolSettingsStore> CommandPoolStore for Arc<CommandPool<D,S>>{
    fn cmdpool(&self) -> &vk::CommandPool {
        &self.command_pool
    }
}

impl<D: DeviceSource, S: CommandPoolSettingsStore> Drop for CommandPool<D,S>{
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

impl CommandPoolSettingsStore for SettingsStore{
    fn queue_family_index(&self) -> u32 {
        self.queue_family_index
    }

    fn create_flags(&self) -> Option<CommandPoolCreateFlags> {
        self.create_flags
    }

    fn reset_flags(&self) -> Option<vk::CommandPoolResetFlags> {
        self.reset_flags
    }

}

impl<D: DeviceSource, S: CommandPoolSettingsStore> DeviceSupplier<D> for CommandPool<D,S>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}
