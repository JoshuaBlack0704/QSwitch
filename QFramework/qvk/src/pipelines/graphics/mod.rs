use ash::vk;

use crate::init::DeviceStore;

use super::PipelineLayoutStore;

pub mod renderpass;
pub trait RenderPassStore{
    fn renderpass(&self) -> vk::RenderPass;
}

pub mod graphics;
pub trait VertexStateFactory{
    fn flags(&self) -> Option<vk::PipelineVertexInputStateCreateFlags>;
    fn bindings(&self) -> &[vk::VertexInputBindingDescription];
    fn attributes(&self) -> &[vk::VertexInputAttributeDescription];
}
pub trait TesselationStateFactory{
    fn flags(&self) -> Option<vk::PipelineTessellationStateCreateFlags>;
    fn patch_control_points(&self) -> u32;
    
}
pub struct Graphics<D:DeviceStore, R:RenderPassStore, L:PipelineLayoutStore>{
    device: D,
    _render_pass: R,
    _layout: L,
    pipeline: vk::Pipeline,
}