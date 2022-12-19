use std::sync::{Arc, Mutex};

use ash::vk;

use crate::init::device::DeviceStore;

use self::{commandpool::{CommandPoolSettingsStore, CommandPoolStore}, commandbuffer::CommandBufferSettingsStore};

pub mod commandpool;
pub struct CommandPool<D: DeviceStore, S: CommandPoolSettingsStore>{
    device: Arc<D>,
    settings: S,
    command_pool: vk::CommandPool,
}
pub mod commandbuffer;
pub struct CommandBufferSet<D: DeviceStore, P: CommandPoolStore, S: CommandBufferSettingsStore>{
    device: Arc<D>,
    cmdpool: Arc<P>,
    settings: S,
    cmds: Mutex<Vec<Arc<vk::CommandBuffer>>>,
}

// pub mod commandbuffer;