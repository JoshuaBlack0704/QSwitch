use ash::vk;

use crate::{init::DeviceSource, descriptor::{DescriptorLayoutSource, WriteSource}, shader::ShaderSource};

pub mod layout;
pub trait PipelineLayoutFactory<P: PipelineLayoutSource>{
    fn create_pipeline_layout<W:WriteSource>(&self, layouts: &[&impl DescriptorLayoutSource<W>], pushes: &[vk::PushConstantRange], flags: Option<vk::PipelineLayoutCreateFlags>) -> P;
}
pub trait PipelineLayoutSource{
    fn layout(&self) -> vk::PipelineLayout;
}
pub struct Layout<D:DeviceSource>{
    device: D,
    layout: vk::PipelineLayout,
}

pub mod compute;
pub trait ComputePipelineFactory<C:ComputePipelineSource>{
    fn create_compute_pipeline(&self, shader: &impl ShaderSource, flags: Option<vk::PipelineCreateFlags>) -> C;
}
pub trait ComputePipelineSource{
    fn pipeline(&self) -> &vk::Pipeline;
}
#[allow(unused)]
pub struct Compute<D:DeviceSource, L:PipelineLayoutSource>{
    device: D,
    layout: L,
    pipeline: vk::Pipeline,
}

pub mod graphics;

pub mod raytracing;


