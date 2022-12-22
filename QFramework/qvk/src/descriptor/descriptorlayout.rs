use std::sync::{Arc, Mutex, MutexGuard};

use ash::vk;
use log::{debug, info};
use crate::descriptor::{DescriptorLayoutBindingFactory, DescriptorLayoutStore};


use crate::init::{DeviceStore, DeviceSupplier};
use super::{DescriptorLayout, WriteHolder, WriteStore};

impl<D:DeviceStore + Clone> DescriptorLayout<D,Arc<WriteHolder>>{
    pub fn new(device_provider: &D, flags: Option<vk::DescriptorSetLayoutCreateFlags>) -> Arc<Self> {
        Arc::new(
            Self{
                device: device_provider.clone(),
                bindings: Mutex::new(vec![]),
                layout: Mutex::new(None),
                flags,
                writes: Mutex::new(vec![]),
            }
        )
    }

    pub fn form_binding<BP: DescriptorLayoutBindingFactory>(self: &Arc<Self>, binding_provider: &BP, stage: vk::ShaderStageFlags) -> Arc<super::WriteHolder>{
        if let Some(_) = *self.layout.lock().unwrap(){
            //The layout will be created the first time it is used
            panic!("Cannot add descriptor layout binding after the first time you use the layout");
        }

        let mut bindings = self.bindings.lock().unwrap();
        let mut writes = self.writes.lock().unwrap();

        
        let mut binding = binding_provider.binding();
        binding.stage_flags = stage;
        binding.binding = bindings.len() as u32;

        let write = vk::WriteDescriptorSet::builder()
            .dst_binding(bindings.len() as u32)
            .descriptor_type(binding.descriptor_type)
            .build();
        let write = WriteHolder::new(binding.descriptor_type, bindings.len() as u32, write);
        bindings.push(binding);
        writes.push(write.clone());
        write
    }
}

impl<D:DeviceStore,W:WriteStore> DescriptorLayoutStore<W> for Arc<DescriptorLayout<D,W>>{
    fn layout(&self) -> vk::DescriptorSetLayout {
        let mut layout = self.layout.lock().unwrap();
        if let Some(l) = *layout{
            return l;
        }

        let mut info = vk::DescriptorSetLayoutCreateInfo::builder();
        if let Some(f) = self.flags{
            info = info.flags(f);
        }
        let bindings = self.bindings.lock().unwrap();
        info = info.bindings(&bindings);
        unsafe{
            let device = self.device.device();
            let res = device.create_descriptor_set_layout(&info, None).unwrap();
            info!("Created descriptor set layout {:?} with {} bindings", res, bindings.len());
            *layout = Some(res);
            res
        }
    }

    fn writes(&self) -> MutexGuard<Vec<W>> {
        self.writes.lock().unwrap()
    }

    fn bindings(&self) -> MutexGuard<Vec<vk::DescriptorSetLayoutBinding>> {
        self.bindings.lock().unwrap()
    }
}

impl<D:DeviceStore,W:WriteStore> Drop for DescriptorLayout<D,W>{
    fn drop(&mut self) {
        if let Some(l) = *self.layout.lock().unwrap(){
            debug!("Destroyed descriptor set layout {:?}", l);
            unsafe{
                self.device.device().destroy_descriptor_set_layout(l, None);
            }
        }
    }
}

impl<D:DeviceStore,W:WriteStore> DeviceSupplier<D> for Arc<DescriptorLayout<D,W>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}