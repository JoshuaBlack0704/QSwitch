use std::{sync::Arc, ffi::CString};

use ash::vk;
use log::{info, debug};

use crate::device::DeviceStore;

use super::Shader;

pub trait SpirvStore{
    fn code(&self) -> &[u32];
    fn entry_name(&self) -> &str;
}

pub trait ShaderStore{
    fn stage(&self) -> vk::PipelineShaderStageCreateInfo;
}

impl<D:DeviceStore> Shader<D>{
    pub fn new<Spv: SpirvStore>(device_provider: &Arc<D>, spriv_data: &Spv, stage: vk::ShaderStageFlags, flags: Option<vk::ShaderModuleCreateFlags>) -> Arc<Shader<D>> {
        let mut info = vk::ShaderModuleCreateInfo::builder();
        if let Some(flags) = flags{
            info = info.flags(flags);
        }
        info = info.code(spriv_data.code());

        let module;
        unsafe{
            module = device_provider.device().create_shader_module(&info, None).unwrap();
        }
        info!("Created shader module {:?}", module);

        Arc::new(
            Self{
                device: device_provider.clone(),
                module,
                stage,
                name: CString::new(spriv_data.entry_name()).unwrap(),
            }
        )
    }
}

impl<D:DeviceStore> Drop for Shader<D>{
    fn drop(&mut self) {
        debug!("Destroyed shader module {:?}", self.module);
        unsafe{
            self.device.device().destroy_shader_module(self.module, None);
        }
    }
}

impl<D:DeviceStore> ShaderStore for Shader<D>{
    fn stage(&self) -> vk::PipelineShaderStageCreateInfo {
        vk::PipelineShaderStageCreateInfo::builder()
        .stage(self.stage)
        .module(self.module)
        .name(&self.name)
        .build()
    }
}