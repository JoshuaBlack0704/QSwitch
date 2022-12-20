use std::sync::Arc;

use ash::vk;

use crate::init::DeviceStore;

pub mod layout;
pub trait PipelineLayoutStore{
    fn layout(&self) -> vk::PipelineLayout;
}
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


