use std::sync::Arc;

use ash::vk;

use crate::device::DeviceProvider;

pub mod shader;
pub struct Shader<D:DeviceProvider>{
    device: Arc<D>,
    module: vk::ShaderModule,
    stage: vk::ShaderStageFlags,
}

pub mod spirvdata;
pub struct HLSL{
    code: Vec<u32>,
}