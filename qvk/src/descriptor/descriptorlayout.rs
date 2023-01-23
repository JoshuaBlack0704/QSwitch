use std::sync::{Arc, Mutex, MutexGuard};

use crate::descriptor::{DescriptorLayoutBindingFactory, DescriptorLayoutSource};
use ash::vk;
use log::{debug, info};

use super::{DescriptorLayout, DescriptorLayoutFactory, WriteHolder, WriteSource};
use crate::init::{DeviceSource, InstanceSource};

impl<D: DeviceSource + Clone>
    DescriptorLayoutFactory<Arc<WriteHolder>, Arc<DescriptorLayout<D, Arc<WriteHolder>>>> for D
{
    fn create_descriptor_layout(
        &self,
        flags: Option<vk::DescriptorSetLayoutCreateFlags>,
    ) -> Arc<DescriptorLayout<D, Arc<WriteHolder>>> {
        Arc::new(DescriptorLayout {
            device: self.clone(),
            bindings: Mutex::new(vec![]),
            layout: Mutex::new(None),
            flags,
            writes: Mutex::new(vec![]),
        })
    }
}

impl<D: DeviceSource + Clone> DescriptorLayout<D, Arc<WriteHolder>> {
    pub fn form_binding<BP: DescriptorLayoutBindingFactory>(
        self: &Arc<Self>,
        binding_provider: &BP,
        stage: vk::ShaderStageFlags,
    ) -> Arc<super::WriteHolder> {
        if let Some(_) = *self.layout.lock().unwrap() {
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

impl<D: DeviceSource, W: WriteSource> DescriptorLayoutSource<W> for Arc<DescriptorLayout<D, W>> {
    fn layout(&self) -> vk::DescriptorSetLayout {
        let mut layout = self.layout.lock().unwrap();
        if let Some(l) = *layout {
            return l;
        }

        let mut info = vk::DescriptorSetLayoutCreateInfo::builder();
        if let Some(f) = self.flags {
            info = info.flags(f);
        }
        let bindings = self.bindings.lock().unwrap();
        info = info.bindings(&bindings);
        unsafe {
            let device = self.device.device();
            let res = device.create_descriptor_set_layout(&info, None).unwrap();
            info!(
                "Created descriptor set layout {:?} with {} bindings",
                res,
                bindings.len()
            );
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

impl<D: DeviceSource, W: WriteSource> Drop for DescriptorLayout<D, W> {
    fn drop(&mut self) {
        if let Some(l) = *self.layout.lock().unwrap() {
            debug!("Destroyed descriptor set layout {:?}", l);
            unsafe {
                self.device.device().destroy_descriptor_set_layout(l, None);
            }
        }
    }
}

impl<D: DeviceSource + InstanceSource, W: WriteSource> InstanceSource
    for Arc<DescriptorLayout<D, W>>
{
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}

impl<D: DeviceSource, W: WriteSource> DeviceSource for Arc<DescriptorLayout<D, W>> {
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