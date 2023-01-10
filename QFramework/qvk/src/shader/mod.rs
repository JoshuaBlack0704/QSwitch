use std::ffi::CString;

use ash::vk;

use crate::init::DeviceSource;

pub mod shader;
pub trait ShaderFactory<Shd: ShaderSource> {
    fn create_shader(
        &self,
        spriv_data: &impl SpirvStore,
        stage: vk::ShaderStageFlags,
        flags: Option<vk::ShaderModuleCreateFlags>,
    ) -> Shd;
}
pub trait SpirvStore {
    fn code(&self) -> &[u32];
    fn entry_name(&self) -> &str;
}
pub trait ShaderSource {
    fn stage(&self) -> vk::PipelineShaderStageCreateInfo;
}
pub struct Shader<D: DeviceSource> {
    device: D,
    module: vk::ShaderModule,
    stage: vk::ShaderStageFlags,
    name: CString,
}

pub mod spirvdata;
pub struct HLSL {
    code: Vec<u32>,
    entry_name: String,
}
pub struct GLSL {
    code: Vec<u32>,
    entry_name: String,
}
pub struct SPV{
    code: Vec<u32>,
    entry_name: String,
}
