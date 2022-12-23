use ash::vk;

use crate::init::DeviceSource;

pub mod layout;
pub trait PipelineLayoutStore{
    fn layout(&self) -> vk::PipelineLayout;
}
pub struct Layout<D:DeviceSource>{
    device: D,
    layout: vk::PipelineLayout,
}

pub mod compute;
#[allow(unused)]
pub struct Compute<D:DeviceSource, L:PipelineLayoutStore>{
    device: D,
    layout: L,
    pipeline: vk::Pipeline,
}

pub mod graphics;

pub mod raytracing;


