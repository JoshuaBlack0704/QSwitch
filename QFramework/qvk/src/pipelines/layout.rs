use std::sync::Arc;

use ash::vk;
use log::{debug, info};

use crate::init::{DeviceSource, InstanceSource};
use crate::descriptor::{DescriptorLayoutSource, WriteSource};
use crate::pipelines::PipelineLayoutSource;

use super::{Layout, PipelineLayoutFactory};

impl<D:DeviceSource + Clone> PipelineLayoutFactory<Arc<Layout<D>>> for D{
    fn create_pipeline_layout<W:WriteSource>(&self, layouts: &[&impl DescriptorLayoutSource<W>], pushes: &[vk::PushConstantRange], flags: Option<vk::PipelineLayoutCreateFlags>) -> Arc<Layout<D>> {
        let mut info = vk::PipelineLayoutCreateInfo::builder();

        if let Some(flags) = flags{
            info = info.flags(flags);
        }

        let layouts:Vec<vk::DescriptorSetLayout> = layouts.iter().map(|l| l.layout()).collect();

        info = info.set_layouts(&layouts);
        info = info.push_constant_ranges(&pushes);

        let layout;
        unsafe{
            let device = self.device();
            layout = device.create_pipeline_layout(&info, None).unwrap();
        }

        info!("Created pipeline layout {:?}", layout);

        Arc::new(
            Layout{
                device: self.clone(),
                layout,
            }
        )
    }
}

impl<D:DeviceSource> PipelineLayoutSource for Arc<Layout<D>>{
    fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }
}

impl<D:DeviceSource + InstanceSource> InstanceSource for Arc<Layout<D>>{
    
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}

impl<D:DeviceSource> DeviceSource for Arc<Layout<D>>{
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

impl<D:DeviceSource> Drop for Layout<D>{
    fn drop(&mut self) {
        debug!("Destroyed pipeline layout {:?}", self.layout);
        unsafe{
            self.device.device().destroy_pipeline_layout(self.layout, None);
        }
    }
}
