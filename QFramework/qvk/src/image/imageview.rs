use std::sync::Arc;

use ash::vk;
use log::{debug, info};

use crate::image::{ImageSource, ImageViewSource};
use crate::init::{DeviceSource, InstanceSource};
use crate::memory::MemorySource;

use super::{ImageResourceSource, ImageView, ImageViewFactory};

impl<Factory: DeviceSource + ImageSource + ImageResourceSource + Clone>
    ImageViewFactory<Arc<ImageView<Factory>>> for Factory
{
    fn create_image_view(
        &self,
        format: vk::Format,
        view_type: vk::ImageViewType,
        swizzle: Option<vk::ComponentMapping>,
        flags: Option<vk::ImageViewCreateFlags>,
    ) -> Arc<ImageView<Factory>> {
        let components;
        if let Some(c) = swizzle {
            components = c;
        } else {
            components = vk::ComponentMapping::builder()
                .r(vk::ComponentSwizzle::R)
                .g(vk::ComponentSwizzle::G)
                .b(vk::ComponentSwizzle::B)
                .a(vk::ComponentSwizzle::A)
                .build();
        }

        let mut info = vk::ImageViewCreateInfo::builder();
        if let Some(flags) = flags {
            info = info.flags(flags);
        }

        let range = vk::ImageSubresourceRange::builder()
            .aspect_mask(self.aspect())
            .base_mip_level(self.level())
            .base_array_layer(0)
            .level_count(1)
            .layer_count(1);

        info = info
            .image(*self.image())
            .view_type(view_type)
            .format(format)
            .components(components)
            .subresource_range(range.build());

        let view;
        unsafe {
            view = self.device().create_image_view(&info, None).unwrap();
        }
        info!("Created image view {:?}", view);

        Arc::new(ImageView {
            _image_resource: self.clone(),
            view,
            format,
        })
    }
}

impl<IR: ImageResourceSource + DeviceSource + ImageSource> ImageViewSource for Arc<ImageView<IR>> {
    fn format(&self) -> vk::Format {
        self.format
    }

    fn view(&self) -> vk::ImageView {
        self.view
    }
}

impl<IR: ImageResourceSource + DeviceSource + ImageSource + InstanceSource> InstanceSource
    for Arc<ImageView<IR>>
{
    fn instance(&self) -> &ash::Instance {
        self._image_resource.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self._image_resource.entry()
    }
}

impl<IR: ImageResourceSource + DeviceSource + ImageSource + MemorySource> MemorySource
    for Arc<ImageView<IR>>
{
    fn partition(
        &self,
        size: u64,
        alignment: Option<u64>,
    ) -> Result<crate::memory::Partition, crate::memory::partitionsystem::PartitionError> {
        self._image_resource.partition(size, alignment)
    }

    fn memory(&self) -> &vk::DeviceMemory {
        self._image_resource.memory()
    }
}

impl<IR: ImageResourceSource + DeviceSource + ImageSource> DeviceSource for Arc<ImageView<IR>> {
    fn device(&self) -> &ash::Device {
        self._image_resource.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self._image_resource.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self._image_resource.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self._image_resource.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self._image_resource.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self._image_resource.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self._image_resource.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self._image_resource.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self._image_resource.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self._image_resource.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self._image_resource.host_memory_index()
    }
}

impl<IR: ImageResourceSource + DeviceSource + ImageSource> Drop for ImageView<IR> {
    fn drop(&mut self) {
        debug!("Destroyed image view {:?}", self.view);
        unsafe {
            self._image_resource
                .device()
                .destroy_image_view(self.view, None);
        }
    }
}
