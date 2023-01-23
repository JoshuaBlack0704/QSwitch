use std::ffi::CString;

use ash::vk;

pub mod instance;
pub enum InstanceExtension{
    
}
pub trait InstanceSource{
    fn get_instance(&self) -> &ash::Instance;
}
pub struct InstanceBuilder{
    pub app_name: CString,
    pub engine_name: CString,
    pub app_version: u32,
    pub engine_version: u32,
    pub api_version: u32,
    pub use_validation: bool,
    pub validation_enables: Option<Vec<vk::ValidationFeatureEnableEXT>>,
    pub validation_disables: Option<Vec<vk::ValidationFeatureDisableEXT>>,
    pub use_debug: bool,
    pub window_extensions: Option<Vec<*const i8>>,
    pub instance_extensions: Vec<InstanceExtension>,
}
pub struct Instance{
    entry: ash::Entry,
    instance: ash::Instance,
}

pub mod device;
pub enum DeviceExtension{
    Name(*const i8),
}
pub enum DeviceFeatures{}
pub trait DeviceSource{}
pub struct DeviceBuilder<'a>{
    pub user_device_select: bool,
    pub surface_support: Option<&'a winit::window::Window>,
    pub features: vk::PhysicalDeviceFeatures,
    pub extended_features: Vec<DeviceFeatures>,
    pub extensions: Vec<DeviceExtension>,
}
pub struct Device{
    device: ash::Device,
}
#[derive(Clone)]
pub struct PhysicalDeviceData {
    pub physical_device: vk::PhysicalDevice,
    pub properties: vk::PhysicalDeviceProperties,
    pub queue_properties: Vec<vk::QueueFamilyProperties>,
    pub raytracing_properties: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
    pub acc_structure_properties: vk::PhysicalDeviceAccelerationStructurePropertiesKHR,
    pub mem_props: vk::PhysicalDeviceMemoryProperties,
    pub mem_budgets: vk::PhysicalDeviceMemoryBudgetPropertiesEXT,
}
