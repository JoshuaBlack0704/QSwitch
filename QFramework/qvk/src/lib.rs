use std::sync::Arc;

use ash::{self, vk};
use winit;

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
}


