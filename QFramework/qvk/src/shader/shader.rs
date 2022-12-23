use std::sync::Arc;

use ash::vk;
use log::debug;

use crate::init::DeviceStore;
use crate::shader::ShaderStore;

use super::Shader;

impl<D:DeviceStore> Drop for Shader<D>{
    fn drop(&mut self) {
        debug!("Destroyed shader module {:?}", self.module);
        unsafe{
            self.device.device().destroy_shader_module(self.module, None);
        }
    }
}

impl<D:DeviceStore> ShaderStore for Arc<Shader<D>>{
    fn stage(&self) -> vk::PipelineShaderStageCreateInfo {
        vk::PipelineShaderStageCreateInfo::builder()
        .stage(self.stage)
        .module(self.module)
        .name(&self.name)
        .build()
    }
}