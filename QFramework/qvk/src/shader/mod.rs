use std::ffi::CString;

use ash::vk;

use crate::init::DeviceSource;

pub mod shader;
pub trait SpirvStore{
    fn code(&self) -> &[u32];
    fn entry_name(&self) -> &str;
}
pub trait ShaderStore{
    fn stage(&self) -> vk::PipelineShaderStageCreateInfo;
}
pub struct Shader<D:DeviceSource>{
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




