use std::sync::{Arc, Mutex};

use ash::{self, vk};
use commandbuffer::CommandBufferSettingsProvider;
use commandpool::{CommandPoolSettingsProvider, CommandPoolProvider};
use device::DeviceProvider;
use self::image::{image::ImageProvider, imageview::ImageViewProvider};
use instance::InstanceProvider;
use queue::queue::QueueProvider;
use swapchain::SwapchainSettingsProvider;

/// The Provider pattern
/// The framework will provide complete abstraction and zero dependence by using a provider pattern
/// Essentially each object that has a dependency will take type provider traits instead of concrete objects
/// We an object needs a particular dependency it will simply call the provider to give it the data
/// How the data is aquired is completley opaque to the requester

pub trait SettingsProvider<B>{
    fn add_to_builder(&self, builder: B) -> B;
}

pub mod instance;
pub struct Instance{
    entry: ash::Entry,
    instance: ash::Instance,
}

pub mod device;
pub struct Device<I: InstanceProvider>{
    instance: Arc<I>,
    surface: Option<vk::SurfaceKHR>,
    surface_loader: ash::extensions::khr::Surface,
    physical_device: device::PhysicalDeviceData,
    device: ash::Device,
    created_queue_families: Vec<usize>,
}

pub mod commandpool;
pub struct CommandPool<D: DeviceProvider, S: CommandPoolSettingsProvider>{
    device: Arc<D>,
    settings: S,
    command_pool: vk::CommandPool,
}
pub mod commandbuffer;
pub struct CommandBufferSet<D: DeviceProvider, P: CommandPoolProvider, S: CommandBufferSettingsProvider>{
    device: Arc<D>,
    cmdpool: Arc<P>,
    settings: S,
    cmds: Mutex<Vec<Arc<vk::CommandBuffer>>>,
}

pub mod memory;

pub mod swapchain;
pub struct Swapchain<I:InstanceProvider, D: DeviceProvider, S: SwapchainSettingsProvider, Img:ImageProvider, ImgV: ImageViewProvider, Q:QueueProvider>{
    _instance: Arc<I>,
    device: Arc<D>,
    _settings: S,
    create_info: vk::SwapchainCreateInfoKHR,
    surface_loader: ash::extensions::khr::Surface,
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: Mutex<vk::SwapchainKHR>,
    images: Mutex<Vec<Arc<Img>>>,
    views: Mutex<Vec<Arc<ImgV>>>,
    present_queue: Arc<Q>,
}

pub mod sync;

pub mod image;

pub mod queue;

pub mod descriptor;

pub mod shader;

pub mod pipelines;
