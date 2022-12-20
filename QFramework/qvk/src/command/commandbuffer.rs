use std::sync::Arc;

use ash::vk;

use crate::{pipelines::Compute};
use crate::init::DeviceStore;

use super::{CommandBuffer, CommandBufferStore, BindPipelineFactory, BindSetFactory};

impl<D:DeviceStore> CommandBuffer<D>{
    pub fn new(device_store: &Arc<D>, cmd: vk::CommandBuffer) -> Arc<CommandBuffer<D>> {
        Arc::new(
            Self{
                device: device_store.clone(),
                cmd,
            }
        )
    }
}

impl<D:DeviceStore> CommandBufferStore for CommandBuffer<D>{
    fn cmd(&self) -> vk::CommandBuffer {
        self.cmd
    }

    fn begin(&self, info: Option<vk::CommandBufferBeginInfo>) -> Result<(), vk::Result> {
        unsafe{
            let mut begin = vk::CommandBufferBeginInfo::default();
            if let Some(i) = info{
                begin = i;
            }
            self.device.device().begin_command_buffer(self.cmd, &begin)
        }
    }

    fn end(&self) -> Result<(), vk::Result> {
        unsafe{
            self.device.device().end_command_buffer(self.cmd)
        }
    }

    fn barrier(&self, info: vk::DependencyInfo) {
        unsafe{
            self.device.device().cmd_pipeline_barrier2(self.cmd, &info);
        }
    }

    fn bind_pipeline<BP: BindPipelineFactory>(&self, pipeline: &Arc<BP>) {
        unsafe{
            self.device.device().cmd_bind_pipeline(self.cmd, pipeline.bind_point(), pipeline.pipeline());
        }
    }

    fn bind_set<BP:BindPipelineFactory, BS: BindSetFactory>(&self, set: &Arc<BS>, set_index: u32, pipeline: &Arc<BP>) {
        unsafe{
            if let Some(o) = set.dynamic_offsets(){
                let sets = [set.set()];
                self.device.device().cmd_bind_descriptor_sets(self.cmd, set.bind_point(), pipeline.layout(), set_index, &sets, &o);
            }
        }
    }
}
