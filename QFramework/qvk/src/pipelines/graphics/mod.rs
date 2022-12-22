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
pub trait InputStateFactory{
    fn flags(&self) -> Option<vk::PipelineInputAssemblyStateCreateFlags>;
    fn topology(&self) -> vk::PrimitiveTopology;
    fn restart_enabled(&self) -> bool;
}
pub trait TesselationStateFactory{
    fn flags(&self) -> Option<vk::PipelineTessellationStateCreateFlags>;
    fn patch_control_points(&self) -> u32;
}
pub trait ViewportStateFactory{
    fn flags(&self) -> Option<vk::PipelineViewportStateCreateFlags>;
    fn viewports(&self) -> &[vk::Viewport];
    fn scissors(&self) -> &[vk::Rect2D];
}
pub trait RasterizationStateFactory{
    fn flags(&self) -> Option<vk::PipelineRasterizationStateCreateFlags>;
    fn depth_clamp(&self) -> bool;
    fn raster_discard(&self) -> bool;
    fn polygon_mod(&self) -> vk::PolygonMode;
    fn cull_mode(&self) -> vk::CullModeFlags;
    fn front_face(&self) -> vk::FrontFace;
    fn depth_bias(&self) -> bool;
    fn depth_bias_factor(&self) -> f32;
    fn depth_bias_clamp(&self) -> f32;
    fn depth_bias_slope_factor(&self) -> f32;
    fn line_width(&self) -> f32;
}
pub trait MultisampleStateFactory{
    fn flags(&self) -> Option<vk::PipelineMultisampleStateCreateFlags>;
    fn rasterization_samples(&self) -> vk::SampleCountFlags;
    fn sample_shading(&self) -> bool;
    fn min_sample_shading(&self) -> f32;
    fn sample_mask(&self) -> &[vk::SampleMask];
    fn alpha_to_converge(&self) -> bool;
    fn alpha_to_one(&self) -> bool;
}
pub trait DepthStencilStateFactory{
    fn flags(&self) -> Option<vk::PipelineDepthStencilStateCreateFlags>;
    fn depth_test(&self) -> bool;
    fn depth_write(&self) -> bool;
    fn compare_op(&self) -> vk::CompareOp;
    fn depth_bounds_test(&self) -> bool;
    fn stencil_test(&self) -> bool;
    fn front(&self) -> vk::StencilOpState;
    fn back(&self) -> vk::StencilOpState;
    fn min_depth(&self) -> f32;
    fn max_depth(&self) -> f32;
}
pub trait ColorBlendStateFactory{
    fn flags(&self) -> Option<vk::PipelineColorBlendStateCreateFlags>;
    fn enable_logic_op(&self) -> bool;
    fn op(&self) -> vk::LogicOp;
    fn blend_constants(&self) -> [f32; 4];
}
pub trait ColorBlendAttachmentFactory{
    fn blend(&self) -> bool;
    fn src_color(&self) -> vk::BlendFactor;
    fn dst_color(&self) -> vk::BlendFactor;
    fn color_op(&self) -> vk::BlendOp;
    fn src_alpha(&self) -> vk::BlendFactor;
    fn dst_alpha(&self) -> vk::BlendFactor;
    fn alpha_op(&self) -> vk::BlendOp;
    fn color_write_mask(&self) -> vk::ColorComponentFlags;
} 
pub trait DynamicStateFactory{
    fn flags(&self) -> Option<vk::PipelineDynamicStateCreateFlags>;
    fn dynamics(&self) -> &[vk::DynamicState];
}
pub struct Graphics<D:DeviceStore, R:RenderPassStore, L:PipelineLayoutStore>{
    device: D,
    _render_pass: R,
    _layout: L,
    pipeline: vk::Pipeline,
}