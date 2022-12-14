use std::{sync::{Arc, Mutex}, collections::{HashSet, VecDeque}};

use ash::{self, vk};
use memory::Partition;

/// The Provider pattern
/// The framework will provide complete abstraction and zero dependence by using a provider pattern
/// Essentially each object that has a dependency will take type provider traits instead of concrete objects
/// We an object needs a particular dependency it will simply call the provider to give it the data
/// How the data is aquired is completley opaque to the requester

pub mod qvk_settings;
pub struct QvkSettings{
    _instance_settings: instance::SettingsProvider,
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
    cmds: Mutex<HashSet<vk::CommandBuffer>>,
    free_cmds: Mutex<VecDeque<vk::CommandBuffer>>,
}

pub mod memory;

pub mod swapchain;
pub struct Swapchain<I:instance::InstanceProvider, D: device::DeviceProvider, S: swapchain::SwapchainSettingsProvider, Img:image::ImageProvider, ImgV: imageview::ImageViewProvider>{
    _instance: Arc<I>,
    device: Arc<D>,
    _settings: S,
    create_info: vk::SwapchainCreateInfoKHR,
    surface_loader: ash::extensions::khr::Surface,
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: Mutex<vk::SwapchainKHR>,
    images: Mutex<Vec<Arc<Img>>>,
    views: Mutex<Vec<Arc<ImgV>>>,
}

pub mod sync;

pub mod image;
pub struct Image<D:device::DeviceProvider, M:memory::memory::MemoryProvider>{
    device: Arc<D>,
    memory: Option<Arc<M>>,
    _partition: Option<Partition>,
    image: vk::Image,
    create_info: vk::ImageCreateInfo,
    current_layout: Mutex<vk::ImageLayout>,
}

pub mod imageview;
pub struct ImageView<D:device::DeviceProvider, I:image::ImageProvider>{
    _device: Arc<D>,
    _image: Arc<I>,
    _view: vk::ImageView,
}

