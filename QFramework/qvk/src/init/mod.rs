use std::sync::{Arc, Mutex};

use ash::vk;

use crate::{image::{image::ImageStore, imageview::ImageViewStore}, queue::queue::QueueStore};

use self::{instance::InstanceStore, device::DeviceStore, swapchain::SwapchainSettingsStore};

pub mod instance;
pub struct Instance{
    entry: ash::Entry,
    instance: ash::Instance,
}

pub mod device;
pub struct Device<I: InstanceStore>{
    instance: Arc<I>,
    surface: Option<vk::SurfaceKHR>,
    surface_loader: ash::extensions::khr::Surface,
    physical_device: device::PhysicalDeviceData,
    device: ash::Device,
    created_queue_families: Vec<usize>,
}
pub mod swapchain;
pub struct Swapchain<I:InstanceStore, D: DeviceStore, S: SwapchainSettingsStore, Img:ImageStore, ImgV: ImageViewStore, Q:QueueStore>{
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
