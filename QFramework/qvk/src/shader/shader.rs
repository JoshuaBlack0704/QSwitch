use std::{ffi::CString, sync::Arc};

use ash::vk;
use log::{debug, info};

use crate::init::{DeviceSource, InstanceSource};
use crate::shader::{ShaderSource, SpirvStore};

use super::{Shader, ShaderFactory};

impl<D:DeviceSource + Clone> ShaderFactory<Arc<Shader<D>>> for D{
    fn create_shader(&self, spriv_data: &impl SpirvStore, stage: vk::ShaderStageFlags, flags: Option<vk::ShaderModuleCreateFlags>) -> Arc<Shader<D>> {
        let device_provider = self;
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

impl<D:DeviceSource + InstanceSource> InstanceSource for Arc<Shader<D>>{
    
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}

impl<D:DeviceSource> DeviceSource for Arc<Shader<D>>{
    fn device(&self) -> &ash::Device {
        self.device.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.device.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.device.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.device.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.device.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.device.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.device.host_memory_index()
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
