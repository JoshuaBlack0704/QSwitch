use std::{ffi::CString, sync::Arc};

use ash::vk;
use log::{debug, info};

use crate::init::{DeviceSource, DeviceSupplier};
use crate::shader::{ShaderSource, SpirvStore};

use super::{Shader, ShaderFactory};

impl<D:DeviceSource + Clone, DS:DeviceSupplier<D>> ShaderFactory<Arc<Shader<D>>> for DS{
    fn create_shader(&self, spriv_data: &impl SpirvStore, stage: vk::ShaderStageFlags, flags: Option<vk::ShaderModuleCreateFlags>) -> Arc<Shader<D>> {
        let device_provider = self.device_provider();
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
            Shader{
                device: device_provider.clone(),
                module,
                stage,
                name: CString::new(spriv_data.entry_name()).unwrap(),
            }
        )
    }
}


impl<D:DeviceSource> ShaderSource for Arc<Shader<D>>{
    fn stage(&self) -> vk::PipelineShaderStageCreateInfo {
        vk::PipelineShaderStageCreateInfo::builder()
        .stage(self.stage)
        .module(self.module)
        .name(&self.name)
        .build()
    }
}

impl<D:DeviceSource> Drop for Shader<D>{
    fn drop(&mut self) {
        debug!("Destroyed shader module {:?}", self.module);
        unsafe{
            self.device.device().destroy_shader_module(self.module, None);
        }
    }
}
