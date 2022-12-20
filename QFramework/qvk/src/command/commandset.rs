use std::sync::{Arc, Mutex};

use ash::vk;
use log::info;
use crate::command::{CommandBufferFactory, CommandPoolStore};

use crate::init::device::DeviceStore;

use super::{CommandBuffer, CommandBufferStore, CommandSet};


pub trait CommandSetSettingsStore{
    fn cmd_level(&self) -> vk::CommandBufferLevel;
    fn cmd_batch_size(&self) -> u32;
    fn cmd_reset_flags(&self) -> Option<vk::CommandBufferResetFlags>;
}

pub enum CopyOpError{
    NoSpace,
}

#[derive(Clone)]
pub struct SettingsStore{
    pub cmd_level: vk::CommandBufferLevel,
    pub batch_size: u32,
    pub reset_flags: Option<vk::CommandBufferResetFlags>,
}

impl<D: DeviceStore, P: CommandPoolStore, S: CommandSetSettingsStore + Clone> CommandSet<D,P,S>{
    pub fn new(settings: &S, device_provider: &Arc<D>, cmdpool_provider: &Arc<P>) -> Arc<CommandSet<D,P,S>> {
        Arc::new(
            CommandSet{ 
                device: device_provider.clone(),
                cmdpool: cmdpool_provider.clone(),
                settings: settings.clone(),
                cmds: Mutex::new(vec![]),
            }
        )
    }
}

impl<D:DeviceStore, P:CommandPoolStore, S:CommandSetSettingsStore> CommandBufferFactory<D> for CommandSet<D,P,S>{
    fn next_cmd(&self) -> Arc<CommandBuffer<D>> {
        // First we need to see if there are any cmds available
        let mut cmds = self.cmds.lock().unwrap();

        //All we do is loop through cmds and see if we have a free cmd
        for cmd in cmds.iter(){
            if Arc::strong_count(cmd) == 1{
                //If we do we return it
                return cmd.clone();
            }
        }
      
        // If not we need to make a new batch
        let mut alloc_builder = vk::CommandBufferAllocateInfo::builder();
        alloc_builder = alloc_builder.command_pool(*self.cmdpool.cmdpool());
        alloc_builder = alloc_builder.command_buffer_count(self.settings.cmd_batch_size());
        alloc_builder = alloc_builder.level(self.settings.cmd_level());
        let new_cmds = unsafe{self.device.device().allocate_command_buffers(&alloc_builder).expect("Could not allocate command buffers")};
        // Now the book keeping and queueing
        for cmd in new_cmds{
            info!("Created command buffer {:?}", cmd);
            cmds.push(CommandBuffer::new(&self.device, cmd));
        }
        // Now we get a newly queued element
        cmds.last().unwrap().clone()
                
    }

    /// This requires flags to be set on the parent command pool
    fn reset_cmd(&self, cmd: &Arc<CommandBuffer<D>>) {
        unsafe{self.device.device().reset_command_buffer(cmd.cmd(), self.settings.cmd_reset_flags().expect("No command buffer reset flags provided"))}.expect("Failed to reset command buffer");
    }
}

impl SettingsStore{
    pub fn new(level: vk::CommandBufferLevel, batch_size: u32) -> SettingsStore {
        SettingsStore{ cmd_level: level, batch_size, reset_flags: None }
    }
    pub fn use_reset_flags(&mut self, flags: vk::CommandBufferResetFlags){
        self.reset_flags = Some(flags);
    }
}

impl Default for SettingsStore{
    fn default() -> Self {
        Self::new(vk::CommandBufferLevel::PRIMARY, 3)
    }
}

impl CommandSetSettingsStore for SettingsStore{
    fn cmd_level(&self) -> vk::CommandBufferLevel {
        self.cmd_level
    }

    fn cmd_batch_size(&self) -> u32 {
        self.batch_size
    }

    fn cmd_reset_flags(&self) -> Option<vk::CommandBufferResetFlags> {
        self.reset_flags
    }
}
