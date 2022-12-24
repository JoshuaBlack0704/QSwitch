use std::sync::Arc;

use ash::vk;
use log::{debug, info};

use crate::command::BindPipelineFactory;
use crate::init::{DeviceSource, DeviceSupplier};
use crate::pipelines::PipelineLayoutSource;
use crate::shader::ShaderSource;

use super::{Compute, ComputePipelineFactory, ComputePipelineSource};

impl<D:DeviceSource + Clone, L:PipelineLayoutSource + DeviceSupplier<D> + Clone> ComputePipelineFactory<Arc<Compute<D,L>>> for L{
    fn create_compute_pipeline(&self, shader: &impl ShaderSource, flags: Option<vk::PipelineCreateFlags>) -> Arc<Compute<D,L>> {
        let mut info = vk::ComputePipelineCreateInfo::builder();
        if let Some(flags) = flags{
            info = info.flags(flags);
        }
        info = info.stage(shader.stage());
        info = info.layout(self.layout());
        let info = [info.build()];
        
        let pipeline;
        unsafe{
            let device = self.device_provider().device();
            pipeline = device.create_compute_pipelines(vk::PipelineCache::null(), &info, None).unwrap()[0];
        }

        info!("Created compute pipeline {:?}", pipeline);

        Arc::new(
            Compute{
                device: self.device_provider().clone(),
                layout: self.clone(),
                pipeline,
            }
        )
    }
}

impl<D:DeviceSource, L:PipelineLayoutSource> ComputePipelineSource for Arc<Compute<D,L>>{
    fn pipeline(&self) -> &vk::Pipeline {
        &self.pipeline
    }
}

impl<D:DeviceSource, L:PipelineLayoutSource> BindPipelineFactory for Arc<Compute<D,L>>{
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

impl<D:DeviceSource, L:PipelineLayoutSource> Drop for Compute<D,L>{
    fn drop(&mut self) {
        debug!("Destroyed compute pipeline {:?}", self.pipeline);
        unsafe{
            self.device.device().destroy_pipeline(self.pipeline, None);
        }
    }
}
