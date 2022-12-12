use std::{sync::Arc, collections::{HashSet, VecDeque}};

use ash::vk;
use log::info;

use crate::{device, commandpool, CommandBufferSet};

pub trait CommandBufferSettingsProvider{
    fn cmd_level(&self) -> vk::CommandBufferLevel;
    fn cmd_batch_size(&self) -> u32;
    fn cmd_reset_flags(&self) -> Option<vk::CommandBufferResetFlags>;
}
pub trait CommandBufferProvider{
    fn next_cmd(&mut self) -> vk::CommandBuffer;
    fn return_cmd(&mut self, cmd: vk::CommandBuffer);
    fn reset_cmd(&self, cmd: &vk::CommandBuffer);
}

#[derive(Clone)]
pub struct SettingsProvider{
    cmd_level: vk::CommandBufferLevel,
    batch_size: u32,
    reset_flags: Option<vk::CommandBufferResetFlags>,
}

impl<D: device::DeviceProvider, P: commandpool::CommandPoolProvider, S: CommandBufferSettingsProvider + Clone> CommandBufferSet<D, P, S>{
    pub fn new(settings: &S, device_provider: &Arc<D>, cmdpool_provider: &Arc<P>) -> CommandBufferSet<D, P, S> {
        CommandBufferSet{ 
            device: device_provider.clone(),
            cmdpool: cmdpool_provider.clone(),
            settings: settings.clone(),
            cmds: HashSet::new(),
            free_cmds: VecDeque::new(), }
    }
}

impl<D:device::DeviceProvider, P:commandpool::CommandPoolProvider, S:CommandBufferSettingsProvider> CommandBufferProvider for CommandBufferSet<D,P,S>{
    fn next_cmd(&mut self) -> vk::CommandBuffer {
        // First we need to see if there are any cmds available
        
        if let Some(cmd) = self.free_cmds.pop_front(){
            return cmd;
        }
        
        // If not we need to make a new batch
        let mut alloc_builder = vk::CommandBufferAllocateInfo::builder();
        alloc_builder = alloc_builder.command_pool(*self.cmdpool.cmdpool());
        alloc_builder = alloc_builder.command_buffer_count(self.settings.cmd_batch_size());
        alloc_builder = alloc_builder.level(self.settings.cmd_level());
        let cmds = unsafe{self.device.device().allocate_command_buffers(&alloc_builder).expect("Could not allocate command buffers")};
        // Now the book keeping and queueing
        for cmd in cmds{
            info!("Created command buffer {:?}", cmd);
            self.cmds.insert(cmd);
            self.free_cmds.push_back(cmd);
        }
        // Now we get a newly queued element
        self.free_cmds.pop_front().unwrap()
        
    }

    fn return_cmd(&mut self, cmd: vk::CommandBuffer) {
        // First we checl if this command belongs here
        if let None = self.cmds.get(&cmd){
            panic!("Command buffer {:?} returned to wrong set", cmd);
        }
        
        // If it does we add it back to the queue
        self.free_cmds.push_back(cmd);
    }

    /// This requires flags to be set on the parent command pool
    fn reset_cmd(&self, cmd: &vk::CommandBuffer) {
        unsafe{self.device.device().reset_command_buffer(*cmd, self.settings.cmd_reset_flags().expect("No command buffer reset flags provided"))}.expect("Failed to reset command buffer");
    }
}

impl SettingsProvider{
    pub fn new(level: vk::CommandBufferLevel, batch_size: u32) -> SettingsProvider {
        SettingsProvider{ cmd_level: level, batch_size, reset_flags: None }
    }
    pub fn use_reset_flags(&mut self, flags: vk::CommandBufferResetFlags){
        self.reset_flags = Some(flags);
    }
}

impl Default for SettingsProvider{
    fn default() -> Self {
        Self::new(vk::CommandBufferLevel::PRIMARY, 3)
    }
}

impl CommandBufferSettingsProvider for SettingsProvider{
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
