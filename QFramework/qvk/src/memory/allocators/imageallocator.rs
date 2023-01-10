use std::sync::Arc;

use ash::vk;
use log::info;

use crate::init::{DeviceSource, InstanceSource};

use super::{
    ImageAllocator, ImageAllocatorFactory, ImageAllocatorSource, ImageExtensions, MemorySource,
};

impl<M: DeviceSource + MemorySource + Clone> ImageAllocatorFactory for M {
    type ImgAlloc = Arc<ImageAllocator<M>>;

    fn create_image_allocator(
        &self,
        format: ash::vk::Format,
        levels: u32,
        layers: u32,
        usage: ash::vk::ImageUsageFlags,
        img_type: ash::vk::ImageType,
        samples: ash::vk::SampleCountFlags,
        tiling: ash::vk::ImageTiling,
        share: Option<Vec<u32>>,
        flags: Option<ash::vk::ImageCreateFlags>,
        extensions: &[super::ImageExtensions],
    ) -> Self::ImgAlloc {
        Arc::new(ImageAllocator {
            mem_alloc: self.clone(),
            format,
            levels,
            layers,
            usage,
            img_type,
            samples,
            tiling,
            share,
            flags,
            extensions: extensions.to_vec(),
        })
    }

    fn create_image_allocator_simple(
        &self,
        format: ash::vk::Format,
        usage: ash::vk::ImageUsageFlags,
    ) -> Self::ImgAlloc {
        let levels = 1;
        let layers = 1;
        let img_type = vk::ImageType::TYPE_2D;
        let samples = vk::SampleCountFlags::TYPE_1;
        let tiling = vk::ImageTiling::OPTIMAL;
        let share = None;
        let flags = None;
        let extensions = [];
        self.create_image_allocator(
            format,
            levels,
            layers,
            usage,
            img_type,
            samples,
            tiling,
            share,
            flags,
            &extensions,
        )
    }
}

impl<M: DeviceSource + MemorySource + Clone> ImageAllocatorSource for Arc<ImageAllocator<M>> {
    fn get_image(&self, extent: vk::Extent3D) -> (vk::ImageCreateInfo, vk::Image, super::MemPart) {
        let mut info = vk::ImageCreateInfo::builder();
        let mut extensions = self.extensions.clone();
        for ext in extensions.iter_mut() {
            info = ext.push(info);
        }
        if let Some(flags) = self.flags {
            info = info.flags(flags);
        }
        info = info.image_type(self.img_type);
        info = info.format(self.format);
        info = info.extent(extent);
        info = info.mip_levels(self.levels);
        info = info.array_layers(self.layers);
        info = info.samples(self.samples);
        info = info.tiling(self.tiling);
        info = info.usage(self.usage);
        info = info.sharing_mode(vk::SharingMode::EXCLUSIVE);
        if let Some(indices) = &self.share {
            info = info.sharing_mode(vk::SharingMode::CONCURRENT);
            info = info.queue_family_indices(&indices);
        }
        info = info.initial_layout(vk::ImageLayout::UNDEFINED);

        let device = self.mem_alloc.device();
        let image = unsafe { device.create_image(&info, None).unwrap() };
        info!("Created image {:?}", image);

        let reqs = unsafe { device.get_image_memory_requirements(image) };
        let mem_part = self.mem_alloc.get_space(reqs.size, Some(reqs.alignment));

        unsafe {
            device
                .bind_image_memory(image, mem_part.0, mem_part.1.offset)
                .unwrap()
        };

        (info.build(), image, mem_part)
    }
}

impl ImageExtensions {
    pub fn push<'a>(
        &mut self,
        _builder: vk::ImageCreateInfoBuilder<'a>,
    ) -> vk::ImageCreateInfoBuilder<'a> {
        todo!();
    }
}
impl<M: MemorySource + DeviceSource> DeviceSource for Arc<ImageAllocator<M>> {
    fn device(&self) -> &ash::Device {
        self.mem_alloc.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.mem_alloc.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.mem_alloc.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.mem_alloc.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.mem_alloc.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.mem_alloc.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.mem_alloc.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.mem_alloc.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.mem_alloc.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.mem_alloc.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.mem_alloc.host_memory_index()
    }
}

impl<M: MemorySource + DeviceSource + InstanceSource> InstanceSource for Arc<ImageAllocator<M>> {
    fn instance(&self) -> &ash::Instance {
        self.mem_alloc.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.mem_alloc.entry()
    }
}
