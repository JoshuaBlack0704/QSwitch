use std::sync::Arc;

use ash::vk;
use log::{debug, info};

use crate::init::DeviceSupplier;
use crate::init::DeviceSource;
use crate::descriptor::{DescriptorLayoutSource, WriteSource};
use crate::pipelines::PipelineLayoutSource;

use super::{Layout, PipelineLayoutFactory};

impl<D:DeviceSource + Clone, DS:DeviceSupplier<D>> PipelineLayoutFactory<Arc<Layout<D>>> for DS{
    fn create_pipeline_layout<W:WriteSource>(&self, layouts: &[&impl DescriptorLayoutSource<W>], pushes: &[vk::PushConstantRange], flags: Option<vk::PipelineLayoutCreateFlags>) -> Arc<Layout<D>> {
        let mut info = vk::PipelineLayoutCreateInfo::builder();

        if let Some(flags) = flags{
            info = info.flags(flags);
        }

        let layouts:Vec<vk::DescriptorSetLayout> = layouts.iter().map(|l| l.layout()).collect();

        info = info.set_layouts(&layouts);
        info = info.push_constant_ranges(&pushes);

        let layout;
        unsafe{
            let device = self.device_provider().device();
            layout = device.create_pipeline_layout(&info, None).unwrap();
        }

        info!("Created pipeline layout {:?}", layout);

        Arc::new(
            Layout{
                device: self.device_provider().clone(),
                layout,
            }
        )
    }
}

impl<D:DeviceSource> PipelineLayoutSource for Arc<Layout<D>>{
    fn layout(&self) -> vk::PipelineLayout {
        self.layout
    }
}

impl<D:DeviceSource> DeviceSupplier<D> for Arc<Layout<D>>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}

impl<D:DeviceSource> Drop for Layout<D>{
    fn drop(&mut self) {
        debug!("Destroyed pipeline layout {:?}", self.layout);
        unsafe{
            self.device.device().destroy_pipeline_layout(self.layout, None);
        }
    }
}
