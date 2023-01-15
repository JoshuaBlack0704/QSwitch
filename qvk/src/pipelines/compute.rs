use std::sync::Arc;

use ash::vk;
use log::{debug, info};

use crate::init::DeviceSource;
use crate::pipelines::PipelineLayoutSource;
use crate::shader::ShaderSource;
use crate::{command::BindPipelineFactory, init::InstanceSource};

use super::{Compute, ComputePipelineFactory, ComputePipelineSource};

impl<L: PipelineLayoutSource + DeviceSource + Clone> ComputePipelineFactory<Arc<Compute<L>>> for L {
    fn create_compute_pipeline(
        &self,
        shader: &impl ShaderSource,
        flags: Option<vk::PipelineCreateFlags>,
    ) -> Arc<Compute<L>> {
        let mut info = vk::ComputePipelineCreateInfo::builder();
        if let Some(flags) = flags {
            info = info.flags(flags);
        }
        info = info.stage(shader.stage());
        info = info.layout(self.layout());
        let info = [info.build()];

        let pipeline;
        unsafe {
            let device = self.device();
            pipeline = device
                .create_compute_pipelines(vk::PipelineCache::null(), &info, None)
                .unwrap()[0];
        }

        info!("Created compute pipeline {:?}", pipeline);

        Arc::new(Compute {
            layout: self.clone(),
            pipeline,
        })
    }
}

impl<L: PipelineLayoutSource + DeviceSource> ComputePipelineSource for Arc<Compute<L>> {
    fn pipeline(&self) -> &vk::Pipeline {
        &self.pipeline
    }
}

impl<L: PipelineLayoutSource + DeviceSource> BindPipelineFactory for Arc<Compute<L>> {
    fn layout(&self) -> vk::PipelineLayout {
        self.layout.layout()
    }

    fn bind_point(&self) -> vk::PipelineBindPoint {
        vk::PipelineBindPoint::COMPUTE
    }

    fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }
}

impl<L: PipelineLayoutSource + DeviceSource> Drop for Compute<L> {
    fn drop(&mut self) {
        debug!("Destroyed compute pipeline {:?}", self.pipeline);
        unsafe {
            self.layout.device().destroy_pipeline(self.pipeline, None);
        }
    }
}

impl<L: DeviceSource + PipelineLayoutSource + InstanceSource> InstanceSource for Arc<Compute<L>> {
    fn instance(&self) -> &ash::Instance {
        self.layout.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.layout.entry()
    }
}

impl<L: DeviceSource + PipelineLayoutSource> DeviceSource for Arc<Compute<L>> {
    fn device(&self) -> &ash::Device {
        self.layout.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.layout.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.layout.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.layout.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.layout.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.layout.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.layout.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.layout.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.layout.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.layout.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.layout.host_memory_index()
    }
}
