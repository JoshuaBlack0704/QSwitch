use std::sync::Arc;

use ash::vk;
use log::{debug, info};

use crate::command::BindPipelineFactory;
use crate::init::DeviceSource;
use crate::pipelines::PipelineLayoutStore;
use crate::shader::ShaderStore;

use super::Compute;

impl<D:DeviceSource + Clone, L:PipelineLayoutStore + Clone> Compute<D,L>{
    pub fn new<Shd:ShaderStore>(device_provider: &D, shader: &Shd, layout_provider: &L, flags: Option<vk::PipelineCreateFlags>) -> Arc<Compute<D, L>> {
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

impl<D:DeviceSource, L:PipelineLayoutStore> BindPipelineFactory for Arc<Compute<D,L>>{
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

impl<D:DeviceSource, L:PipelineLayoutStore> Drop for Compute<D,L>{
    fn drop(&mut self) {
        debug!("Destroyed compute pipeline {:?}", self.pipeline);
        unsafe{
            self.device.device().destroy_pipeline(self.pipeline, None);
        }
    }
}
