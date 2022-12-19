use std::sync::Arc;

use ash::vk;

use crate::init::device::DeviceStore;

use self::layout::PipelineLayoutStore;

pub trait BindPipelineFactory{
    ///Should bind the pipeline to the command buffer
    fn bind(&self, cmd: &Arc<vk::CommandBuffer>);
}

pub mod layout;
pub struct Layout<D:DeviceStore>{
    device: Arc<D>,
    layout: vk::PipelineLayout,
}

pub mod compute;
pub struct Compute<D:DeviceStore, L:PipelineLayoutStore>{
    device: Arc<D>,
    layout: Arc<L>,
    pipeline: vk::Pipeline,
}

pub mod graphics;

pub mod raytracing;