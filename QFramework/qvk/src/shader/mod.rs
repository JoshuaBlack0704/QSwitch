use std::{ffi::CString, sync::Arc};

use ash::vk;
use log::info;

use crate::init::DeviceStore;

pub mod shader;
pub trait ShaderFactory<Shd: ShaderStore>{
    fn create_shader<Spv:SpirvStore>(&self, spirv_data: &Spv, stage: vk::ShaderStageFlags, flags: Option<vk::ShaderModuleCreateFlags>) -> Shd; 
}
impl<D:DeviceStore + Clone> ShaderFactory<Arc<Shader<D>>> for D{
    fn create_shader<Spv:SpirvStore>(&self, spirv_data: &Spv, stage: vk::ShaderStageFlags, flags: Option<vk::ShaderModuleCreateFlags>) -> Arc<Shader<D>> {
        let mut info = vk::ShaderModuleCreateInfo::builder();
        if let Some(flags) = flags{
            info = info.flags(flags);
        }
        info = info.code(spirv_data.code());

        let module;
        unsafe{
            module = self.device().create_shader_module(&info, None).unwrap();
        }
        info!("Created shader module {:?}", module);

        Arc::new(
            Shader{
                device: self.clone(),
                module,
                stage,
                name: CString::new(spirv_data.entry_name()).unwrap(),
            }
        )
    }
}
pub trait SpirvStore{
    fn code(&self) -> &[u32];
    fn entry_name(&self) -> &str;
}
pub trait ShaderStore{
    fn stage(&self) -> vk::PipelineShaderStageCreateInfo;
}
pub struct Shader<D:DeviceStore>{
    device: D,
    module: vk::ShaderModule,
    stage: vk::ShaderStageFlags,
    name: CString,
    }

pub mod spirvdata;
pub struct HLSL{
    code: Vec<u32>,
    entry_name: String,
}




