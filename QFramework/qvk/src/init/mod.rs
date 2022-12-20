use std::sync::Mutex;

use ash::vk;

use crate::queue::QueueStore;
use crate::image::{ImageStore, ImageViewStore};

use self::swapchain::SwapchainSettingsStore;

pub mod instance;
pub trait InstanceStore{
    fn instance(&self) -> &ash::Instance;
    fn entry(&self) -> &ash::Entry;
}

pub trait InternalInstanceStore<I:InstanceStore>{
    fn instance_provider(&self) -> &I;
}
pub struct Instance{
    entry: ash::Entry,
    instance: ash::Instance,
}

pub mod device;
pub struct Device<I: InstanceStore>{
    instance: I,
    surface: Option<vk::SurfaceKHR>,
    surface_loader: ash::extensions::khr::Surface,
    physical_device: PhysicalDeviceData,
    device: ash::Device,
    created_queue_families: Vec<usize>,
}
pub mod swapchain;
pub struct Swapchain<I:InstanceStore, D: DeviceStore, S: SwapchainSettingsStore, Img:ImageStore, ImgV: ImageViewStore, Q:QueueStore>{
    _instance: I,
    device: D,
    _settings: S,
    create_info: vk::SwapchainCreateInfoKHR,
    surface_loader: ash::extensions::khr::Surface,
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: Mutex<vk::SwapchainKHR>,
    images: Mutex<Vec<Img>>,
    views: Mutex<Vec<ImgV>>,
    present_queue: Q,
}

#[derive(Clone)]
pub struct PhysicalDeviceData{
    pub physical_device: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties,
    pub queue_properties: Vec<vk::QueueFamilyProperties>,
    pub raytracing_properties: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
    pub acc_structure_properties: vk::PhysicalDeviceAccelerationStructurePropertiesKHR,
    pub mem_props: vk::PhysicalDeviceMemoryProperties,
    pub mem_budgets: vk::PhysicalDeviceMemoryBudgetPropertiesEXT
}

pub trait DeviceStore{
    fn device(&self) -> &ash::Device;
    fn surface(&self) -> &Option<vk::SurfaceKHR>;
    fn physical_device(&self) -> &PhysicalDeviceData;
    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)>;
    fn grahics_queue(&self) -> Option<(vk::Queue, u32)>;
    fn compute_queue(&self) -> Option<(vk::Queue, u32)>;
    fn transfer_queue(&self) -> Option<(vk::Queue, u32)>;
    fn present_queue(&self) -> Option<(vk::Queue, u32)>;
    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32;
    fn device_memory_index(&self) -> u32;
    fn host_memory_index(&self) -> u32;
}

pub trait InternalDeviceStore<D:DeviceStore>{
    fn device_provider(&self) -> &D;
}


