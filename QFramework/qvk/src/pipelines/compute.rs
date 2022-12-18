use std::sync::Arc;

use ash::vk;
use log::{info, debug};

use crate::{device::DeviceProvider, shader::shader::ShaderProvider};

use super::{Compute, layout::PipelineLayoutProvider};


impl<D:DeviceProvider, L:PipelineLayoutProvider> Compute<D,L>{
    pub fn new<Shd:ShaderProvider>(device_provider: &Arc<D>, shader: &Arc<Shd>, layout_provider: &Arc<L>, flags: Option<vk::PipelineCreateFlags>) -> Arc<Compute<D, L>> {
        let mut info = vk::ComputePipelineCreateInfo::builder();
        if let Some(flags) = flags{
            info = info.flags(flags);
        }
        info = info.stage(shader.stage());
        info = info.layout(layout_provider.layout());
        let info = [info.build()];
        
        let pipeline;
        unsafe{
            let device = device_provider.device();
            pipeline = device.create_compute_pipelines(vk::PipelineCache::null(), &info, None).unwrap()[0];
        }

        info!("Created compute pipeline {:?}", pipeline);

        Arc::new(
            Self{
                device: device_provider.clone(),
                layout: layout_provider.clone(),
                pipeline,
            }
        )
        
    }
}

impl<D:DeviceProvider, L:PipelineLayoutProvider> Drop for Compute<D,L>{
    fn drop(&mut self) {
        debug!("Destroyed compute pipeline {:?}", self.pipeline);
        unsafe{
            self.device.device().destroy_pipeline(self.pipeline, None);
        }
    }
}