use std::{sync::Arc, collections::{HashSet, VecDeque}};

use ash::{self, vk};

/// The Provider pattern
/// The framework will provide complete abstraction and zero dependence by using a provider pattern
/// Essentially each object that has a dependency will take type provider traits instead of concrete objects
/// We an object needs a particular dependency it will simply call the provider to give it the data
/// How the data is aquired is completley opaque to the requester

pub mod qvk_settings;
pub struct QvkSettings{
    instance_settings: instance::SettingsProvider,
    
}

pub mod instance;
pub struct Instance{
    entry: ash::Entry,
    instance: ash::Instance,
}

pub mod device;
pub struct Device<I: instance::InstanceProvider>{
    instance: Arc<I>,
    surface: Option<vk::SurfaceKHR>,
    surface_loader: ash::extensions::khr::Surface,
    physical_device: device::PhysicalDeviceData,
    device: ash::Device,
    created_queue_families: Vec<usize>,
}

pub mod commandpool;
pub struct CommandPool<D: device::DeviceProvider, S: commandpool::CommandPoolSettingsProvider>{
    device: Arc<D>,
    settings: S,
    command_pool: vk::CommandPool,
}
pub mod commandbuffer;
pub struct CommandBufferSet<D: device::DeviceProvider, P: commandpool::CommandPoolProvider, S: commandbuffer::CommandBufferSettingsProvider>{
    device: Arc<D>,
    cmdpool: Arc<P>,
    settings: S,
    cmds: HashSet<vk::CommandBuffer>,
    free_cmds: VecDeque<vk::CommandBuffer>,
}

pub mod memory;

pub mod swapchain;
pub struct Swapchain<D: device::DeviceProvider, S: swapchain::SwapchainSettingsProvider>{
    device: Arc<D>,
    settings: S,
    surface_loader: ash::extensions::khr::Surface,
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: vk::SwapchainKHR,
}

