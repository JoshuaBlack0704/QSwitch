use std::sync::Mutex;

use ash::vk;

use crate::queue::QueueSource;

use self::swapchain::SwapchainSettingsStore;

pub mod instance;
pub trait InstanceFactory<I:InstanceSource>{
    fn create_instance(&self) -> I;
}
pub trait InstanceSource{
    fn instance(&self) -> &ash::Instance;
    fn entry(&self) -> &ash::Entry;
}
pub struct Instance{
    entry: ash::Entry,
    instance: ash::Instance,
}

pub mod device;
pub trait DeviceFactory<D:DeviceSource>{
    fn create_device(&self) -> Result<D, vk::Result>;
}
pub trait DeviceSource{
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

pub struct Device<I: InstanceSource>{
    instance: I,
    surface: Option<vk::SurfaceKHR>,
    surface_loader: ash::extensions::khr::Surface,
    physical_device: PhysicalDeviceData,
    device: ash::Device,
    created_queue_families: Vec<usize>,
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

pub mod swapchain;
pub struct Swapchain<D: DeviceSource + InstanceSource, S: SwapchainSettingsStore, Q:QueueSource>{
    device: D,
    _settings: S,
    create_info: Mutex<vk::SwapchainCreateInfoKHR>,
    surface_loader: ash::extensions::khr::Surface,
    swapchain_loader: ash::extensions::khr::Swapchain,
    swapchain: Mutex<vk::SwapchainKHR>,
    images: Mutex<Vec<vk::Image>>,
    present_queue: Q,
}




