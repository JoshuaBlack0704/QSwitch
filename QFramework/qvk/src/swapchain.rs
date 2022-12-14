use std::sync::Arc;

use ash::vk;
use log::{info, debug};

use crate::{Swapchain, device, instance};

pub trait SwapchainSettingsProvider{
    fn extensions(&self) -> Option<Vec<SwapchainCreateExtension>>;
    fn create_flags(&self) -> vk::SwapchainCreateFlagsKHR;
    fn custom_min_image_count(&self) -> Option<u32>;
    fn custom_ranked_image_format(&self) -> Option<&[vk::SurfaceFormatKHR]>;
    fn image_color_space(&self) -> vk::ColorSpaceKHR;
    fn custom_image_extent(&self) -> Option<vk::Extent2D>;
    fn image_array_layers(&self) -> u32;
    fn image_usage(&self) -> vk::ImageUsageFlags;
    fn share(&self) -> Option<&[u32]>;
    fn custom_pre_transform(&self) -> Option<vk::SurfaceTransformFlagsKHR>;
    fn composite_alpha(&self) -> vk::CompositeAlphaFlagsKHR;
    fn custom_ranked_present_modes(&self) -> Option<&[vk::PresentModeKHR]>;
    fn clipped(&self) -> bool;
}

pub enum SwapchainCreateExtension{
    
}

#[derive(Debug)]
pub enum SwapchainCreateError{
    NoSurface,
    VulkanError(vk::Result),
}

impl<D: device::DeviceProvider, S:SwapchainSettingsProvider> Swapchain<D,S>{
    pub fn new<I:instance::InstanceProvider>(instance_provider: &Arc<I>, device_provider: &Arc<D>, settings: S)  -> Result<Arc<Swapchain<D,S>>, SwapchainCreateError>{
        let surface = device_provider.surface();
        if let None = surface{
            return Err(SwapchainCreateError::NoSurface);
        }
        let surface = surface.unwrap();
        let surface_loader = ash::extensions::khr::Surface::new(instance_provider.entry(), instance_provider.instance());

        let capabilites = unsafe{surface_loader.get_physical_device_surface_capabilities(device_provider.physical_device().physical_device, surface)};
        let present_modes = unsafe{surface_loader.get_physical_device_surface_present_modes(device_provider.physical_device().physical_device, surface)};
        let formats = unsafe{surface_loader.get_physical_device_surface_formats(device_provider.physical_device().physical_device, surface)};
        if let Err(e) = capabilites{
            return Err(SwapchainCreateError::VulkanError(e));
        }
        if let Err(e) = present_modes{
            return Err(SwapchainCreateError::VulkanError(e));
        }
        if let Err(e) = formats{
            return Err(SwapchainCreateError::VulkanError(e));
        }
        let capabilities = capabilites.unwrap();
        let present_modes = present_modes.unwrap();
        let formats = formats.unwrap();

        let mut chosen_format = vk::SurfaceFormatKHR{
            format: vk::Format::B8G8R8A8_SRGB,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        };

        if !formats.contains(&chosen_format){
            chosen_format = formats[0];
        }
        if let Some(custom_formats) = settings.custom_ranked_image_format(){
            for f in custom_formats.iter(){
                if formats.contains(f){
                    chosen_format = *f;
                    break;
                }
            }
        }
        
        let mut info = vk::SwapchainCreateInfoKHR::builder();
        let extensions = settings.extensions();
        if let Some(mut ext) = extensions{
            for ext in ext.iter_mut(){
                match ext{
                    _ => todo!()
                }
            }
        }

        let mut chosen_present_mode = vk::PresentModeKHR::FIFO;
        if let Some(modes) = settings.custom_ranked_present_modes(){
            for mode in modes{
                if present_modes.contains(mode){
                    chosen_present_mode = *mode;
                    break;
                }
            }
        }
        
        info = info.flags(settings.create_flags());
        info = info.surface(surface);
        info = info.min_image_count(capabilities.min_image_count);
        if let Some(c) = settings.custom_min_image_count(){
            info = info.min_image_count(c);
        }
        info = info.image_format(chosen_format.format);
        info = info.image_color_space(chosen_format.color_space);
        info = info.image_extent(capabilities.current_extent);
        if let Some(e) = settings.custom_image_extent(){
            info = info.image_extent(e);
        }
        info = info.image_array_layers(settings.image_array_layers());
        info = info.image_usage(settings.image_usage());
        info = info.image_sharing_mode(vk::SharingMode::EXCLUSIVE);
        if let Some(i) = settings.share(){
            info = info.image_sharing_mode(vk::SharingMode::CONCURRENT);
            info = info.queue_family_indices(i);
        }
        info = info.pre_transform(capabilities.current_transform);
        if let Some(t) = settings.custom_pre_transform(){
            info = info.pre_transform(t);
        }
        info = info.composite_alpha(settings.composite_alpha());
        info = info.present_mode(chosen_present_mode);
        info = info.clipped(settings.clipped());

        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance_provider.instance(), device_provider.device());

        let swapchain = unsafe{swapchain_loader.create_swapchain(&info, None)};
        if let Err(e) = swapchain{
            return Err(SwapchainCreateError::VulkanError(e));
        }
        let swapchain = swapchain.unwrap();

        info!("Created swapchain {:?}", swapchain);


        Ok(
            Arc::new(
                Swapchain{
                    device: device_provider.clone(),
                    settings,
                    swapchain,
                    surface_loader,
                    swapchain_loader,
                }
            )
        )
    }
}

impl<D:device::DeviceProvider, S:SwapchainSettingsProvider> Drop for Swapchain<D,S>{
    fn drop(&mut self) {
        debug!("Destroying swapchain {:?}", self.swapchain);
        unsafe{
            self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        }
    }
}