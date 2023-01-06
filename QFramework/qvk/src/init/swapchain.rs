use std::sync::{Arc, Mutex};

use ash::vk;
use log::{debug, info};

use crate::sync::SemaphoreFactory;
use crate::{
    image::{Image, ImageResourceFactory, ImageResourceSource, ImageSource},
    memory::{Memory, PartitionSystem},
    queue::{Queue, QueueFactory, QueueOps, QueueSource},
    sync::{self, FenceSource, SemaphoreSource},
};

use super::{DeviceSource, InstanceSource, Swapchain};

pub trait SwapchainSettingsStore {
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

pub trait SwapchainSource<D: DeviceSource> {
    fn present<S: SemaphoreSource>(&self, next_image: u32, waits: Option<&[&S]>);
    fn wait_present<S: SemaphoreSource>(&self, next_image: u32, waits: Option<&[&S]>);
    fn aquire_next_image<F: FenceSource, S: SemaphoreSource>(
        &self,
        timeout: u64,
        fence: Option<&F>,
       semaphore: Option<&S>,
    ) -> u32;
    fn gpu_aquire_next_image<S:SemaphoreSource>(&self, timeout: u64, semaphore: &S) -> u32;
    fn cpu_aquire_next_image<F:FenceSource>(&self, timeout: u64, fence: &F) -> u32;
    fn resize(&self, size: Option<(u32,u32)>);
    fn extent(&self) -> vk::Extent3D;
    fn images(&self) -> Vec<Arc<Image<D, Arc<Memory<D, PartitionSystem>>>>>;
    fn present_image<IR: ImageResourceSource + ImageSource, Q: QueueOps>(
        &self,
        src: &IR,
        queue: &Q,
    );
}

#[derive(Clone)]
pub enum SwapchainCreateExtension {}

#[derive(Debug)]
pub enum SwapchainCreateError {
    NoSurface,
    VulkanError(vk::Result),
}

type SwapchainType<D, S> = Swapchain<D, S, Arc<Queue<D>>>;

#[derive(Clone)]
pub struct SettingsStore {
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

impl<D: DeviceSource + InstanceSource + Clone, S: SwapchainSettingsStore + Clone>
    SwapchainType<D, S>
{
    pub fn new(
        device_supplier: &D,
        settings: &S,
        old_swapchain: Option<&Arc<Self>>,
    ) -> Result<Arc<Self>, SwapchainCreateError> {
        let device_provider = device_supplier;
        let instance_provider = device_provider.clone();
        let surface = device_provider.surface();
        if let None = surface {
            return Err(SwapchainCreateError::NoSurface);
        }
        let surface = surface.unwrap();
        let surface_loader = ash::extensions::khr::Surface::new(
            instance_provider.entry(),
            instance_provider.instance(),
        );

        let capabilites = unsafe {
            surface_loader.get_physical_device_surface_capabilities(
                device_provider.physical_device().physical_device,
                surface,
            )
        };
        let present_modes = unsafe {
            surface_loader.get_physical_device_surface_present_modes(
                device_provider.physical_device().physical_device,
                surface,
            )
        };
        let formats = unsafe {
            surface_loader.get_physical_device_surface_formats(
                device_provider.physical_device().physical_device,
                surface,
            )
        };
        if let Err(e) = capabilites {
            return Err(SwapchainCreateError::VulkanError(e));
        }
        if let Err(e) = present_modes {
            return Err(SwapchainCreateError::VulkanError(e));
        }
        if let Err(e) = formats {
            return Err(SwapchainCreateError::VulkanError(e));
        }
        let capabilities = capabilites.unwrap();
        let present_modes = present_modes.unwrap();
        let formats = formats.unwrap();

        let mut chosen_format = vk::SurfaceFormatKHR {
            format: vk::Format::B8G8R8A8_SRGB,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        };

        if !formats.contains(&chosen_format) {
            chosen_format = formats[0];
        }
        if let Some(custom_formats) = settings.custom_ranked_image_format() {
            for f in custom_formats.iter() {
                if formats.contains(f) {
                    chosen_format = *f;
                    break;
                }
            }
        }

        let mut info = vk::SwapchainCreateInfoKHR::builder();
        let extensions = settings.extensions();
        if let Some(mut ext) = extensions {
            for ext in ext.iter_mut() {
                match ext {
                    _ => todo!(),
                }
            }
        }

        let mut chosen_present_mode = vk::PresentModeKHR::FIFO;
        if let Some(modes) = settings.custom_ranked_present_modes() {
            for mode in modes {
                if present_modes.contains(mode) {
                    chosen_present_mode = *mode;
                    break;
                }
            }
        }

        if let Some(flags) = settings.create_flags() {
            info = info.flags(flags);
        }
        info = info.surface(surface);
        info = info.min_image_count(capabilities.min_image_count);
        if let Some(c) = settings.custom_min_image_count() {
            info = info.min_image_count(c);
        }
        info = info.image_format(chosen_format.format);
        info = info.image_color_space(chosen_format.color_space);

        let mut adjusted_extent = capabilities.current_extent;
        if adjusted_extent.width > capabilities.max_image_extent.width || adjusted_extent.width < capabilities.min_image_extent.width{
            adjusted_extent.width = capabilities.min_image_extent.width;
        }
        if adjusted_extent.height > capabilities.max_image_extent.height || adjusted_extent.height < capabilities.min_image_extent.height{
            adjusted_extent.height = capabilities.min_image_extent.height;
        }

        info = info.image_extent(adjusted_extent);
        if let Some(e) = settings.custom_image_extent() {
            info = info.image_extent(e);
        }
        info = info.image_array_layers(settings.image_array_layers());
        info = info.image_usage(settings.image_usage());
        info = info.image_sharing_mode(vk::SharingMode::EXCLUSIVE);
        if let Some(i) = settings.share() {
            info = info.image_sharing_mode(vk::SharingMode::CONCURRENT);
            info = info.queue_family_indices(i);
        }
        info = info.pre_transform(capabilities.current_transform);
        if let Some(t) = settings.custom_pre_transform() {
            info = info.pre_transform(t);
        }
        info = info.composite_alpha(settings.composite_alpha());
        info = info.present_mode(chosen_present_mode);
        info = info.clipped(settings.clipped());
        if let Some(os) = old_swapchain {
            info = info.old_swapchain(*os.swapchain.lock().unwrap());
        }

        let swapchain_loader = ash::extensions::khr::Swapchain::new(
            instance_provider.instance(),
            device_provider.device(),
        );

        let swapchain = unsafe { swapchain_loader.create_swapchain(&info, None) };
        if let Err(e) = swapchain {
            return Err(SwapchainCreateError::VulkanError(e));
        }
        let swapchain = swapchain.unwrap();

        let queue = device_supplier
            .create_queue(vk::QueueFlags::GRAPHICS)
            .unwrap();

        info!("Created swapchain {:?}", swapchain);
        let swapchain = Swapchain {
            device: device_provider.clone(),
            _settings: settings.clone(),
            create_info: Mutex::new(info.build()),
            surface_loader,
            swapchain_loader,
            swapchain: Mutex::new(swapchain),
            images: Mutex::new(vec![]),
            present_queue: queue,
        };

        swapchain.get_images(*swapchain.swapchain.lock().unwrap());

        Ok(Arc::new(swapchain))
    }

    fn get_images(&self, swapchain: vk::SwapchainKHR) {
        let mut img_lock = self.images.lock().unwrap();
        let imgs = unsafe {
            self.swapchain_loader
                .get_swapchain_images(swapchain)
                .unwrap()
        };
        *img_lock = imgs;
    }
}

impl<D: DeviceSource + InstanceSource + Clone, S: SwapchainSettingsStore + Clone> SwapchainSource<D>
    for Arc<SwapchainType<D, S>>
{
    fn present<Sem: SemaphoreSource>(&self, next_image: u32, waits: Option<&[&Sem]>) {
        let mut info = vk::PresentInfoKHR::builder();
        let wait_semaphores: Vec<vk::Semaphore>;
        if let Some(waits) = waits {
            wait_semaphores = waits.iter().map(|s| *s.semaphore()).collect();
            info = info.wait_semaphores(&wait_semaphores);
        }
        let swapchains = [*self.swapchain.lock().unwrap()];
        let images_indices = [next_image];
        info = info.swapchains(&swapchains);
        info = info.image_indices(&images_indices);

        let _ = unsafe {
            self.swapchain_loader
                .queue_present(*self.present_queue.queue(), &info)
        };
    }

    fn resize(&self, extent: Option<(u32, u32)>) {
        let mut swapchain_lock = self.swapchain.lock().unwrap();
        let capabilities = unsafe {
            self.surface_loader
                .get_physical_device_surface_capabilities(
                    self.device.physical_device().physical_device,
                    self.device.surface().unwrap(),
                )
                .unwrap()
        };
        debug!(
            "Resizing swapchain {:?} to {:?}",
            *swapchain_lock, capabilities.current_extent
        );
        let mut info = self.create_info.lock().unwrap();
        info.image_extent = capabilities.current_extent;
        info.old_swapchain = *swapchain_lock;
        let mut adjusted_extent = capabilities.current_extent;
        if let Some((width, height)) = extent{
            adjusted_extent.width = width;
            adjusted_extent.height = height;
        }
        else{
            if adjusted_extent.width > capabilities.max_image_extent.width || adjusted_extent.width < capabilities.min_image_extent.width{
                adjusted_extent.width = capabilities.min_image_extent.width;
            }
            if adjusted_extent.height > capabilities.max_image_extent.height || adjusted_extent.height < capabilities.min_image_extent.height{
                adjusted_extent.height = capabilities.min_image_extent.height;
            }
        }
        info.image_extent = adjusted_extent;
        let new_swapchain = unsafe { self.swapchain_loader.create_swapchain(&info, None).unwrap() };
        unsafe {
            self.swapchain_loader
                .destroy_swapchain(*swapchain_lock, None)
        };
        debug!(
            "Swapchain {:?} destroyed for swapchain {:?}",
            *swapchain_lock, new_swapchain
        );
        self.get_images(new_swapchain);
        *swapchain_lock = new_swapchain;
    }

    fn aquire_next_image<F: FenceSource, Sem: SemaphoreSource>(
        &self,
        timeout: u64,
        fence: Option<&F>,
        semaphore: Option<&Sem>,
    ) -> u32 {
        let mut present_fence = vk::Fence::null();
        let mut present_semaphore = vk::Semaphore::null();

        if let Some(f) = fence {
            present_fence = *f.fence();
        }
        if let Some(s) = semaphore {
            present_semaphore = *s.semaphore();
        }
        let mut swapchain = *self.swapchain.lock().unwrap();

        let mut next_image = unsafe {
            self.swapchain_loader.acquire_next_image(
                swapchain,
                timeout,
                present_semaphore,
                present_fence,
            )
        };
        if let Err(e) = next_image {
            if !(e == vk::Result::ERROR_OUT_OF_DATE_KHR) {
                todo!();
            }
            self.resize(None);
            swapchain = *self.swapchain.lock().unwrap();
            next_image = unsafe {
                self.swapchain_loader.acquire_next_image(
                    swapchain,
                    timeout,
                    present_semaphore,
                    present_fence,
                )
            };
        }
        let (next_image, suboptimal) = next_image.unwrap();
        if !suboptimal {
            return next_image;
        }

        self.resize(None);
        swapchain = *self.swapchain.lock().unwrap();
        let (next_image, _) = unsafe {
            self.swapchain_loader
                .acquire_next_image(swapchain, timeout, present_semaphore, present_fence)
                .unwrap()
        };

        next_image
    }

    fn wait_present<Sem: SemaphoreSource>(&self, next_image: u32, waits: Option<&[&Sem]>) {
        self.present(next_image, waits);
        self.present_queue.wait_idle();
    }

    fn extent(&self) -> vk::Extent3D {
        let lock = self.create_info.lock().unwrap();
        vk::Extent3D::builder()
            .width(lock.image_extent.width)
            .height(lock.image_extent.height)
            .depth(1)
            .build()
    }

    fn images(&self) -> Vec<Arc<Image<D, Arc<Memory<D, PartitionSystem>>>>> {
        let mut images = vec![];
        let info = self.create_info.lock().unwrap();
        let lock = self.images.lock().unwrap();
        for img in lock.iter() {
            let img = Image::<D, Arc<Memory<D, PartitionSystem>>>::from_swapchain_image(
                &self.device,
                *img,
                info.image_extent,
            );
            images.push(img);
        }
        images
    }

    fn present_image<IR: ImageResourceSource + ImageSource, Q: QueueOps>(
        &self,
        src: &IR,
        queue: &Q,
    ) {
        // let semaphore:S
        let images = self.images();

        let aquire = self.create_semaphore();
        let dst_index =
            self.aquire_next_image(u64::MAX, None::<&Arc<sync::Fence<D>>>, Some(&aquire));
        let dst = &images[dst_index as usize];

        src.internal_transistion(vk::ImageLayout::TRANSFER_SRC_OPTIMAL, None);
        dst.internal_transistion(vk::ImageLayout::TRANSFER_DST_OPTIMAL, None);

        let dst_res = dst
            .create_resource(
                vk::Offset3D::default(),
                dst.extent(),
                0,
                vk::ImageAspectFlags::COLOR,
            )
            .unwrap();
        src.blit_to_image_internal(&dst_res, vk::Filter::LINEAR)
            .unwrap();

        dst.internal_transistion(vk::ImageLayout::PRESENT_SRC_KHR, None);

        let waits = [&aquire];
        self.present(dst_index, Some(&waits));
        queue.wait_idle();
    }

    fn gpu_aquire_next_image<Sem:SemaphoreSource>(&self, timeout: u64, semaphore: &Sem) -> u32 {
        let mut swapchain = *self.swapchain.lock().unwrap();

        let mut next_image = unsafe {
            self.swapchain_loader.acquire_next_image(
                swapchain,
                timeout,
                *semaphore.semaphore(),
                vk::Fence::null(),
            )
        };
        if let Err(e) = next_image {
            if !(e == vk::Result::ERROR_OUT_OF_DATE_KHR) {
                todo!();
            }
            self.resize(None);
            swapchain = *self.swapchain.lock().unwrap();
            next_image = unsafe {
                self.swapchain_loader.acquire_next_image(
                    swapchain,
                    timeout,
                    *semaphore.semaphore(),
                    vk::Fence::null(),
                )
            };
        }
        let (next_image, suboptimal) = next_image.unwrap();
        if !suboptimal {
            return next_image;
        }

        self.resize(None);
        swapchain = *self.swapchain.lock().unwrap();
        let (next_image, _) = unsafe {
            self.swapchain_loader
                .acquire_next_image(swapchain, timeout, *semaphore.semaphore(), vk::Fence::null())
                .unwrap()
        };

        next_image
    }

    fn cpu_aquire_next_image<F:FenceSource>(&self, timeout: u64, fence: &F) -> u32 {
        let mut swapchain = *self.swapchain.lock().unwrap();

        let mut next_image = unsafe {
            self.swapchain_loader.acquire_next_image(
                swapchain,
                timeout,
                vk::Semaphore::null(),
                *fence.fence(),
            )
        };
        if let Err(e) = next_image {
            if !(e == vk::Result::ERROR_OUT_OF_DATE_KHR) {
                todo!();
            }
            self.resize(None);
            swapchain = *self.swapchain.lock().unwrap();
            next_image = unsafe {
                self.swapchain_loader.acquire_next_image(
                    swapchain,
                    timeout,
                    vk::Semaphore::null(),
                    *fence.fence(),
                )
            };
        }
        let (next_image, suboptimal) = next_image.unwrap();
        if !suboptimal {
            return next_image;
        }

        self.resize(None);
        swapchain = *self.swapchain.lock().unwrap();
        let (next_image, _) = unsafe {
            self.swapchain_loader.acquire_next_image(
                swapchain,
                timeout,
                vk::Semaphore::null(),
                *fence.fence(),
            ).unwrap()
        };

        next_image
    }
}

impl<D: DeviceSource + InstanceSource, S: SwapchainSettingsStore, Q: QueueSource> Drop
    for Swapchain<D, S, Q>
{
    fn drop(&mut self) {
        let lock = self.swapchain.lock().unwrap();
        let swapchain = *lock;
        debug!("Destroying swapchain {:?}", swapchain);
        unsafe {
            self.swapchain_loader.destroy_swapchain(swapchain, None);
        }
    }
}

impl SettingsStore {
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
    ) -> SettingsStore {
        SettingsStore {
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

impl Default for SettingsStore {
    fn default() -> Self {
        Self::new(
            None,
            None,
            None,
            None,
            None,
            1,
            vk::ImageUsageFlags::TRANSFER_DST,
            None,
            None,
            vk::CompositeAlphaFlagsKHR::OPAQUE,
            Some(vec![vk::PresentModeKHR::MAILBOX]),
            true,
        )
    }
}

impl SwapchainSettingsStore for SettingsStore {
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
        if let Some(formats) = &self.custom_ranked_image_format {
            return Some(&formats);
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
        if let Some(indecies) = &self.share {
            return Some(&indecies);
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
        if let Some(modes) = &self.custom_ranked_present_modes {
            return Some(&modes);
        }
        None
    }

    fn clipped(&self) -> bool {
        self.clipped
    }
}

impl<D: DeviceSource + InstanceSource, S: SwapchainSettingsStore, Q: QueueSource> DeviceSource
    for Arc<Swapchain<D, S, Q>>
{
    fn device(&self) -> &ash::Device {
        self.device.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.device.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.device.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.device.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.device.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.device.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.device.host_memory_index()
    }
}

impl<D: DeviceSource + InstanceSource, S: SwapchainSettingsStore, Q: QueueSource> InstanceSource
    for Arc<Swapchain<D, S, Q>>
{
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}
