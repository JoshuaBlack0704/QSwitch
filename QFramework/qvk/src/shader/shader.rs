use std::sync::Arc;

use ash::vk;
use log::{info, debug};

use crate::device::DeviceProvider;

use super::Shader;

pub trait SpirvProvider{
    fn code(&self) -> &[u32];
    
}

impl<D:DeviceProvider> Shader<D>{
    pub fn new<Spv: SpirvProvider>(device_provider: &Arc<D>, spriv_data: &Spv, stage: vk::ShaderStageFlags, flags: Option<vk::ShaderModuleCreateFlags>) -> Arc<Shader<D>> {
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
            }
        )
    }
}

impl<D:DeviceProvider> Drop for Shader<D>{
    fn drop(&mut self) {
        debug!("Destroyed shader module {:?}", self.module);
        unsafe{
            self.device.device().destroy_shader_module(self.module, None);
        }
    }
}