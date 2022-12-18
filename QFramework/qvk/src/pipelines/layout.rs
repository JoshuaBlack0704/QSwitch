use std::sync::Arc;

use ash::vk;
use log::{info, debug};

use crate::{device::DeviceProvider, descriptor::descriptorlayout::DescriptorLayoutProvider, SettingsProvider};

use super::Layout;

pub trait PipelineLayoutProvider{
    fn layout(&self) -> vk::PipelineLayout;
}

pub struct Settings{
    flags: Option<vk::PipelineLayoutCreateFlags>,
    layouts: Vec<vk::DescriptorSetLayout>,
    pushes: Vec<vk::PushConstantRange>,
}

impl<'a, D:DeviceProvider> Layout<D>{
    pub fn new<S:SettingsProvider<'a, vk::PipelineLayoutCreateInfoBuilder<'a>>>(device_provider: &Arc<D>, settings: &'a S) -> Arc<Layout<D>> {
        let mut info = vk::PipelineLayoutCreateInfo::builder();
        info = settings.add_to_builder(info);

        let layout;
        unsafe{
            let device = device_provider.device();
            layout = device.create_pipeline_layout(&info, None).unwrap();
        }

        info!("Created pipeline layout {:?}", layout);

        Arc::new(
            Self{
                device: device_provider.clone(),
                layout,
            }
        )
    }
}

impl<D:DeviceProvider> PipelineLayoutProvider for Layout<D>{
    fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }
}

impl<D:DeviceProvider> Drop for Layout<D>{
    fn drop(&mut self) {
        debug!("Destroyed pipeline layout {:?}", self.layout);
        unsafe{
            self.device.device().destroy_pipeline_layout(self.layout, None);
        }
    }
}

impl Settings{
    pub fn new(flags: Option<vk::PipelineLayoutCreateFlags>) -> Settings {
        Self{
            flags,
            layouts: vec![],
            pushes: vec![],
        }
    }
    pub fn add_layout<L:DescriptorLayoutProvider>(&mut self, layout: &Arc<L>){
        self.layouts.push(layout.layout());
    }
    pub fn add_push(&mut self, push: vk::PushConstantRange){
        self.pushes.push(push);
    }
}

impl<'a> SettingsProvider<'a, vk::PipelineLayoutCreateInfoBuilder<'a>> for Settings{
    fn add_to_builder(&'a self, mut builder: vk::PipelineLayoutCreateInfoBuilder<'a>) -> vk::PipelineLayoutCreateInfoBuilder<'a> {

        if let Some(flags) = self.flags{
            builder = builder.flags(flags);
        }

        builder = builder.set_layouts(&self.layouts);
        builder = builder.push_constant_ranges(&self.pushes);
        builder
        
    }
}