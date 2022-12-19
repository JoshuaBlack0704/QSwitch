use std::{sync::Arc, ffi::CString};

use ash::vk;

use crate::init::device::DeviceStore;

pub mod shader;
pub struct Shader<D:DeviceStore>{
    device: Arc<D>,
    module: vk::ShaderModule,
    stage: vk::ShaderStageFlags,
    name: CString,
    }

pub mod spirvdata;
pub struct HLSL{
    code: Vec<u32>,
    entry_name: String,
}