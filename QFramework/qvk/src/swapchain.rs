use std::sync::{Arc, Mutex};

use ash::vk;
use log::{info, debug};

use crate::{sync::{semaphore::SemaphoreProvider, fence::FenceProvider, Semaphore}, image::{Image, ImageView, image::{ImageProvider, UsesImageProvider}, imageview::ImageViewProvider, imageresource::ImageSubresourceProvider, ImageResource}, memory::{Memory, PartitionSystem}, instance::{InstanceProvider, UsesInstanceProvider}, device::{DeviceProvider, UsesDeviceProvider}, Swapchain, queue::{queue::QueueProvider, Queue}};


pub trait SwapchainSettingsProvider{
    fn extensions(&self) -> Option<Vec<SwapchainCreateExtension>>;
    fn create_flags(&self) -> Option<vk::SwapchainCreateFlagsKHR>;
    fn custom_min_image_count(&self) -> Option<u32>;
    fn custom_ranked_image_format(&self) -> Option<&[vk::SurfaceFormatKHR]>;
    fn custom_image_extent(&self) -> Option<vk::Extent2D>;
    fn image_array_layers(&self) -> u32;
    fn image_usage(&self) -> vk::ImageUsageFlags;
    fn share(&self) -> Option<&[u32]>;
    fn custom_pre_transform(&self) -> Option<vk::SurfaceTransformFlagsKHR>;
    fn composite_alpha(&self) -> vk::CompositeAlphaFlagsKHR;
    fn custom_ranked_present_modes(&self) -> Option<&[vk::PresentModeKHR]>;
    fn clipped(&self) -> bool;
}

pub trait SwapchainProvider{
    fn present<S:SemaphoreProvider> (&self, next_image: u32, waits: Option<&[&Arc<S>]>);
    fn wait_present<S:SemaphoreProvider>(&self, next_image: u32, waits: Option<&[&Arc<S>]>);
    fn aquire_next_image<F:FenceProvider, S:SemaphoreProvider>(&self, timeout: u64,fence: Option<&Arc<F>>, semaphore: Option<&Arc<S>>) -> u32;
    fn resize(&self);
}

#[derive(Clone)]
pub enum SwapchainCreateExtension{
    
}

#[derive(Debug)]
pub enum SwapchainCreateError{
    NoSurface,
    VulkanError(vk::Result),
}

type ImageType<D> = Image<D, Memory<D, PartitionSystem>>;
type ImageViewType<D> = ImageView<D, ImageType<D>>;

#[derive(Clone)]
pub struct SettingsProvider{
    pub extensions: Option<Vec<SwapchainCreateExtension>>,
    pub create_flags: Option<vk::SwapchainCreateFlagsKHR>,
    pub custom_min_image_count: Option<u32>,
    pub custom_ranked_image_format: Option<Vec<vk::SurfaceFormatKHR>>,
    pub custom_image_extent: Option<vk::Extent2D>,
    pub image_array_layers: u32,
    pub image_usage: vk::ImageUsageFlags,
    pub share: Option<Vec<u32>>,
    pub custom_pre_transform: Option<vk::SurfaceTransformFlagsKHR>,
    pub composite_alpha: vk::CompositeAlphaFlagsKHR,
    pub custom_ranked_present_modes: Option<Vec<vk::PresentModeKHR>>,
    pub clipped: bool,
}

impl<I:InstanceProvider, D: DeviceProvider + UsesInstanceProvider<I>, S:SwapchainSettingsProvider + Clone> Swapchain<I,D,S, ImageType<D>, ImageViewType<D>,Queue<D>>{
    pub fn new(device_provider: &Arc<D>, settings: &S, old_swapchain: Option<&Arc<Self>>)  -> Result<Arc<Self>, SwapchainCreateError>{
        let instance_provider = device_provider.instance_provider();
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
        
        if let Some(flags) = settings.create_flags(){
            info = info.flags(flags);
        }
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
        if let Some(os) = old_swapchain{
            info = info.old_swapchain(*os.swapchain.lock().unwrap());
        }

        let swapchain_loader = ash::extensions::khr::Swapchain::new(instance_provider.instance(), device_provider.device());

        let swapchain = unsafe{swapchain_loader.create_swapchain(&info, None)};
        if let Err(e) = swapchain{
            return Err(SwapchainCreateError::VulkanError(e));
        }
        let swapchain = swapchain.unwrap();

        let queue = Queue::new(device_provider, vk::QueueFlags::GRAPHICS).unwrap();

        info!("Created swapchain {:?}", swapchain);
        let swapchain = Swapchain{
                    _instance: instance_provider.clone(),
                    device: device_provider.clone(),
                    _settings: settings.clone(),
                    create_info: info.build(),
                    surface_loader,
                    swapchain_loader,
                    swapchain: Mutex::new(swapchain),
                    images: Mutex::new(vec![]),
                    views: Mutex::new(vec![]),
                    present_queue: queue,
                };
        
        swapchain.get_images(*swapchain.swapchain.lock().unwrap());

        Ok(
            Arc::new(
                swapchain
            )
        )
    }

    fn get_images(&self, swapchain: vk::SwapchainKHR){
        let mut img_lock = self.images.lock().unwrap();
        let _img_view_lock = self.views.lock().unwrap();
        let imgs = unsafe{self.swapchain_loader.get_swapchain_images(swapchain).unwrap()};
        let mut images = Vec::with_capacity(imgs.len());
        for image in imgs.iter(){
            let image = Image::<D,Memory<D,PartitionSystem>>::from_swapchain_image(&self.device, *image);
            image.internal_transistion(vk::ImageLayout::PRESENT_SRC_KHR, None);
            images.push(image);
        }
        *img_lock = images;
    }

    pub fn present_image<Img:ImageProvider, IR: ImageSubresourceProvider + UsesImageProvider<Img>, F:FenceProvider>(&self, src: &Arc<IR>){
        // let semaphore:S
        let images = self.images.lock().unwrap();
        
        let aquire = Semaphore::new(self.device_provider());
        let dst_index = self.aquire_next_image(u64::MAX, None::<&Arc<F>>, Some(&aquire));
        let dst = &images[dst_index as usize];

        src.image_provider().internal_transistion(vk::ImageLayout::TRANSFER_SRC_OPTIMAL, None);
        dst.internal_transistion(vk::ImageLayout::TRANSFER_DST_OPTIMAL, None);

        let dst_res = ImageResource::new(&dst, vk::ImageAspectFlags::COLOR, 0, 0, 1, vk::Offset3D::default(), dst.extent()).unwrap();
        src.copy_to_image_internal(&dst_res).unwrap();

        dst.internal_transistion(vk::ImageLayout::PRESENT_SRC_KHR, None);

        let waits = [&aquire];
        self.present(dst_index, Some(&waits));
        self.present_queue.wait_idle();
    }
}

impl<I:InstanceProvider, D: DeviceProvider + UsesInstanceProvider<I>, S:SwapchainSettingsProvider + Clone> SwapchainProvider for Swapchain<I,D,S,ImageType<D>,ImageViewType<D>,Queue<D>>{
    fn present<Sem:SemaphoreProvider> (&self, next_image: u32, waits: Option<&[&Arc<Sem>]>) {

        
        let mut info = vk::PresentInfoKHR::builder();
        let wait_semaphores:Vec<vk::Semaphore>;
        if let Some(waits) = waits{
            wait_semaphores = waits.iter().map(|s| *s.semaphore()).collect();
            info = info.wait_semaphores(&wait_semaphores);
        }
        let swapchains = [*self.swapchain.lock().unwrap()];
        let images_indices = [next_image];
        info = info.swapchains(&swapchains);
        info = info.image_indices(&images_indices);

        let _ = unsafe{self.swapchain_loader.queue_present(*self.present_queue.queue(), &info)};
    }

    fn resize(&self) {
        let mut swapchain_lock = self.swapchain.lock().unwrap();
        let capabilites = unsafe{self.surface_loader.get_physical_device_surface_capabilities(self.device.physical_device().physical_device, self.device.surface().unwrap()).unwrap()};
        debug!("Resizing swapchain {:?} to {:?}", *swapchain_lock, capabilites.current_extent);
        let mut info = self.create_info.clone();
        info.image_extent = capabilites.current_extent;
        info.old_swapchain = *swapchain_lock;
        let new_swapchain = unsafe{self.swapchain_loader.create_swapchain(&info, None).unwrap()};
        unsafe{self.swapchain_loader.destroy_swapchain(*swapchain_lock, None)};
        debug!("Swapchain {:?} destroyed for swapchain {:?}", *swapchain_lock, new_swapchain);
        self.get_images(new_swapchain);
        *swapchain_lock = new_swapchain;
    }

    fn aquire_next_image<F:FenceProvider, Sem:SemaphoreProvider>(&self, timeout: u64, fence: Option<&Arc<F>>, semaphore: Option<&Arc<Sem>>) -> u32{
        let mut present_fence = vk::Fence::null();
        let mut present_semaphore = vk::Semaphore::null();

        if let Some(f) = fence{
            present_fence = *f.fence();
        }
        if let Some(s) = semaphore{
            present_semaphore = *s.semaphore();
        }
        let mut swapchain = *self.swapchain.lock().unwrap();

        let mut next_image = unsafe{self.swapchain_loader.acquire_next_image(swapchain, timeout, present_semaphore, present_fence)};
        if let Err(e) = next_image{
            if !(e == vk::Result::ERROR_OUT_OF_DATE_KHR){
                todo!();
            }
            self.resize();
            swapchain = *self.swapchain.lock().unwrap();
            next_image = unsafe{self.swapchain_loader.acquire_next_image(swapchain, timeout, present_semaphore, present_fence)};
        }
        let (next_image, suboptimal) = next_image.unwrap();
        if !suboptimal{
            return next_image;
        }

        self.resize();
        swapchain = *self.swapchain.lock().unwrap();
        let (next_image, _) = unsafe{self.swapchain_loader.acquire_next_image(swapchain, timeout, present_semaphore, present_fence).unwrap()};

        next_image
        
        

        
    }

    fn wait_present<Sem:SemaphoreProvider>(&self, next_image: u32, waits: Option<&[&Arc<Sem>]>) {
        self.present(next_image, waits);
        self.present_queue.wait_idle();
        
    }
}

impl<I:InstanceProvider, D: DeviceProvider, S:SwapchainSettingsProvider, Img:ImageProvider, ImgV: ImageViewProvider, Q:QueueProvider> Drop for Swapchain<I,D,S,Img,ImgV,Q>{
    fn drop(&mut self) {
        let lock = self.swapchain.lock().unwrap();
        let swapchain = *lock;
        debug!("Destroying swapchain {:?}", swapchain);
        unsafe{
            self.swapchain_loader.destroy_swapchain(swapchain, None);
        }
    }
}

impl SettingsProvider{
    pub fn new(
    extensions: Option<Vec<SwapchainCreateExtension>>,
    create_flags: Option<vk::SwapchainCreateFlagsKHR>,
    custom_min_image_count: Option<u32>,
    custom_ranked_image_format: Option<Vec<vk::SurfaceFormatKHR>>,
    custom_image_extent: Option<vk::Extent2D>,
    image_array_layers: u32,
    image_usage: vk::ImageUsageFlags,
    share: Option<Vec<u32>>,
    custom_pre_transform: Option<vk::SurfaceTransformFlagsKHR>,
    composite_alpha: vk::CompositeAlphaFlagsKHR,
    custom_ranked_present_modes: Option<Vec<vk::PresentModeKHR>>,
    clipped: bool,
    )
-> SettingsProvider     {
        SettingsProvider{
            extensions,
            create_flags,
            custom_min_image_count,
            custom_ranked_image_format,
            custom_image_extent,
            image_array_layers,
            image_usage,
            share,
            custom_pre_transform,
            composite_alpha,
            custom_ranked_present_modes,
            clipped,
        }
    }
}

impl Default for SettingsProvider{
    fn default() -> Self {
        Self::new(None, None, None, None, None, 1, vk::ImageUsageFlags::TRANSFER_DST, None, None, vk::CompositeAlphaFlagsKHR::OPAQUE, Some(vec![vk::PresentModeKHR::MAILBOX]), true)
    }
}

impl SwapchainSettingsProvider for SettingsProvider{
    fn extensions(&self) -> Option<Vec<SwapchainCreateExtension>> {
        self.extensions.clone()
    }

    fn create_flags(&self) -> Option<vk::SwapchainCreateFlagsKHR> {
        self.create_flags
    }

    fn custom_min_image_count(&self) -> Option<u32> {
        self.custom_min_image_count
    }

    fn custom_ranked_image_format(&self) -> Option<&[vk::SurfaceFormatKHR]> {
        if let Some(formats) = &self.custom_ranked_image_format{
            return Some(&formats)
        }
        None
    }

    fn custom_image_extent(&self) -> Option<vk::Extent2D> {
        self.custom_image_extent
    }

    fn image_array_layers(&self) -> u32 {
        self.image_array_layers
    }

    fn image_usage(&self) -> vk::ImageUsageFlags {
        self.image_usage
    }

    fn share(&self) -> Option<&[u32]> {
        if let Some(indecies) = &self.share{
            return Some(&indecies)
        }
        None
    }

    fn custom_pre_transform(&self) -> Option<vk::SurfaceTransformFlagsKHR> {
        self.custom_pre_transform
    }

    fn composite_alpha(&self) -> vk::CompositeAlphaFlagsKHR {
        self.composite_alpha
    }

    fn custom_ranked_present_modes(&self) -> Option<&[vk::PresentModeKHR]> {
        if let Some(modes) = &self.custom_ranked_present_modes{
            return Some(&modes);
        }
        None
    }

    fn clipped(&self) -> bool {
        self.clipped
    }
}

impl<I:InstanceProvider, D: DeviceProvider, S:SwapchainSettingsProvider, Img:ImageProvider, ImgV: ImageViewProvider,Q: QueueProvider> UsesDeviceProvider<D> for Swapchain<I,D,S,Img,ImgV,Q>{
    fn device_provider(&self) -> &Arc<D> {
        &self.device
    }
}

impl<I:InstanceProvider, D: DeviceProvider, S:SwapchainSettingsProvider, Img:ImageProvider, ImgV: ImageViewProvider, Q: QueueProvider> UsesInstanceProvider<I> for Swapchain<I,D,S,Img,ImgV,Q>{
    fn instance_provider(&self) -> &Arc<I> {
        &self._instance
    }
}

