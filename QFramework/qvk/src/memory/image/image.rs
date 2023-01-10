use std::sync::{Arc, Mutex};

use ash::vk;
use log::debug;

use crate::command::{CommandBufferFactory, CommandBufferSource, Executor, ImageTransitionFactory};
use crate::init::{DeviceSource, InstanceSource};
use crate::memory::allocators::ImageAllocatorSource;

use super::{Image, ImageFactory, ImageSource};

impl<M: DeviceSource + ImageAllocatorSource + Clone> ImageFactory for M {
    type Image = Arc<Image<M>>;

    fn create_image(&self, extent: vk::Extent3D) -> Self::Image {
        let (info, image, mem_part) = self.get_image(extent);
        Arc::new(Image {
            device: self.clone(),
            _mem_part: mem_part,
            image,
            create_info: info,
            current_layout: Arc::new(Mutex::new(vk::ImageLayout::UNDEFINED)),
        })
    }
}

impl<D: DeviceSource> ImageSource for Arc<Image<D>> {
    fn internal_transistion(&self, new_layout: vk::ImageLayout) {
        {
            let old_layout = *ImageSource::layout(self).lock().unwrap();
            if old_layout == new_layout {
                return;
            }
        }
        let executor = Executor::new(self, vk::QueueFlags::GRAPHICS);
        let cmd = executor.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        cmd.transition_img(
            self,
            new_layout,
            vk::PipelineStageFlags2::TRANSFER,
            vk::AccessFlags2::MEMORY_WRITE,
            vk::PipelineStageFlags2::TRANSFER,
            vk::AccessFlags2::MEMORY_READ,
        );
        cmd.end().unwrap();
        executor.wait_submit_internal();
    }

    fn image(&self) -> &vk::Image {
        &self.image
    }

    fn layout(&self) -> Arc<Mutex<vk::ImageLayout>> {
        self.current_layout.clone()
    }

    fn mip_levels(&self) -> u32 {
        self.create_info.mip_levels
    }

    fn array_layers(&self) -> u32 {
        self.create_info.array_layers
    }

    fn extent(&self) -> vk::Extent3D {
        self.create_info.extent
    }
}

impl<D: DeviceSource> ImageTransitionFactory for Arc<Image<D>> {
    fn image(&self) -> vk::Image {
        self.image
    }

    fn range(&self) -> vk::ImageSubresourceRange {
        vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(self.mip_levels())
            .base_array_layer(0)
            .layer_count(self.array_layers())
            .build()
    }

    fn old_layout(&self) -> Arc<Mutex<vk::ImageLayout>> {
        self.current_layout.clone()
    }
}

impl<D: DeviceSource> Drop for Image<D> {
    fn drop(&mut self) {
        debug!("Destroyed image {:?}", self.image);
        unsafe {
            self.device.device().destroy_image(self.image, None);
        }
    }
}

impl<D: DeviceSource + InstanceSource> InstanceSource for Arc<Image<D>> {
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}

impl<D: DeviceSource> DeviceSource for Arc<Image<D>> {
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
