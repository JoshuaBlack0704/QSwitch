use std::sync::{Mutex, MutexGuard};

use ash::vk;

use crate::{image::ImageViewSource, init::DeviceSource};

use super::PipelineLayoutSource;

pub mod renderpass;
pub trait RenderpassAttachmentSource {
    fn flags(&self) -> Option<vk::AttachmentDescriptionFlags>;
    fn inital_layout(&self) -> vk::ImageLayout;
    fn final_layout(&self) -> vk::ImageLayout;
    fn subpass_layout(&self) -> vk::ImageLayout;
    fn index(&self) -> MutexGuard<u32>;
    fn format(&self) -> vk::Format;
    fn samples(&self) -> vk::SampleCountFlags;
    fn load_op(&self) -> vk::AttachmentLoadOp;
    fn store_op(&self) -> vk::AttachmentStoreOp;
    fn stencil_load(&self) -> Option<vk::AttachmentLoadOp>;
    fn stencil_store(&self) -> Option<vk::AttachmentStoreOp>;
    fn view(&self) -> vk::ImageView;
}
pub trait SubpassDescriptionSource {
    fn index(&self) -> MutexGuard<u32>;
    fn flags(&self) -> Option<vk::SubpassDescriptionFlags>;
    fn bind_point(&self) -> vk::PipelineBindPoint;
    fn input_attachments(&self) -> Option<MutexGuard<Vec<vk::AttachmentReference>>>;
    fn color_attachments(&self) -> Option<MutexGuard<Vec<vk::AttachmentReference>>>;
    fn resolve_attachments(&self) -> Option<MutexGuard<Vec<vk::AttachmentReference>>>;
    fn depth_stencil_attachment(&self) -> Option<MutexGuard<vk::AttachmentReference>>;
    fn preserve_attachments(&self) -> Option<&[u32]>;
    fn dependencies(&self) -> Option<&[vk::SubpassDependency]>;
}
pub trait RenderpassFactory<R: RenderPassSource, A: RenderpassAttachmentSource> {
    fn create_renderpass<S: SubpassDescriptionSource>(
        &self,
        attachments: &[&A],
        subpasses: &[&S],
        flags: Option<vk::RenderPassCreateFlags>,
    ) -> R;
}
pub trait RenderPassSource {
    fn renderpass(&self) -> vk::RenderPass;
}
pub trait FramebufferSource {}
pub struct Renderpass<D: DeviceSource, A: RenderpassAttachmentSource> {
    _device: D,
    _renderpass: vk::RenderPass,
    _attachments: Vec<vk::AttachmentDescription>,
    _subpass_refs: Vec<vk::SubpassDescription>,
    _image_views: Vec<A>,
}
pub struct RenderPassAttachment<IV: ImageViewSource> {
    index: Mutex<u32>,
    view: Mutex<IV>,
    initial_layout: vk::ImageLayout,
    subpass_layout: vk::ImageLayout,
    final_layout: vk::ImageLayout,
    load_op: vk::AttachmentLoadOp,
    store_op: vk::AttachmentStoreOp,
}
pub struct SubpassDescription<A: RenderpassAttachmentSource> {
    index: Mutex<u32>,
    bind_point: vk::PipelineBindPoint,
    flags: Option<vk::SubpassDescriptionFlags>,
    input_attachments: Vec<A>,
    input_refs: Mutex<Vec<vk::AttachmentReference>>,
    color_attachments: Vec<A>,
    color_refs: Mutex<Vec<vk::AttachmentReference>>,
    resolve_attachments: Vec<A>,
    resolve_refs: Mutex<Vec<vk::AttachmentReference>>,
    depth_attachment: Option<A>,
    depth_ref: Mutex<vk::AttachmentReference>,
    preserve_attachments: Vec<u32>,
    dependencies: Vec<vk::SubpassDependency>,
}

pub mod graphics;
pub trait GraphicsPipelineState {
    fn flags(&self) -> Option<vk::PipelineCreateFlags>;
    fn shader_stages(&self) -> &[vk::PipelineShaderStageCreateInfo];
    fn vertex_state(&self) -> vk::PipelineVertexInputStateCreateInfo;
    fn input_assembly_state(&self) -> vk::PipelineInputAssemblyStateCreateInfo;
    fn tesselation_state(&self) -> Option<vk::PipelineTessellationStateCreateInfo>;
    fn viewport_state(&self) -> vk::PipelineViewportStateCreateInfo;
    fn rasterization_state(&self) -> vk::PipelineRasterizationStateCreateInfo;
    fn multisample_state(&self) -> vk::PipelineMultisampleStateCreateInfo;
    fn depth_stencil_state(&self) -> vk::PipelineDepthStencilStateCreateInfo;
    fn color_blend_state(&self) -> vk::PipelineColorBlendStateCreateInfo;
    fn dynamic_state(&self) -> Option<vk::PipelineDynamicStateCreateInfo>;
}
pub trait GraphicsPipelineSource {}
pub trait GraphicsPipelineFactory<
    G: GraphicsPipelineSource,
    S: GraphicsPipelineState,
    L: PipelineLayoutSource,
    R: RenderPassSource,
>
{
    fn create_graphics_pipeline(
        &self,
        state: &S,
        layout: &L,
        renderpass: &R,
        tgt_subpass: u32,
    ) -> Result<G, vk::Result>;
}
pub trait VertexStateFactory {
    fn flags(&self) -> Option<vk::PipelineVertexInputStateCreateFlags>;
    fn bindings(&self) -> Vec<vk::VertexInputBindingDescription>;
    fn attributes(&self) -> Vec<vk::VertexInputAttributeDescription>;
}
pub trait InputStateFactory {
    fn flags(&self) -> Option<vk::PipelineInputAssemblyStateCreateFlags>;
    fn topology(&self) -> vk::PrimitiveTopology;
    fn restart_enabled(&self) -> bool;
}
pub trait TesselationStateFactory {
    fn flags(&self) -> Option<vk::PipelineTessellationStateCreateFlags>;
    fn patch_control_points(&self) -> Option<u32>;
}
pub trait ViewportStateFactory {
    fn flags(&self) -> Option<vk::PipelineViewportStateCreateFlags>;
    fn viewports(&self) -> Vec<vk::Viewport>;
    fn scissors(&self) -> Vec<vk::Rect2D>;
}
pub trait RasterizationStateFactory {
    fn flags(&self) -> Option<vk::PipelineRasterizationStateCreateFlags>;
    fn depth_clamp(&self) -> bool;
    fn raster_discard(&self) -> bool;
    fn polygon_mode(&self) -> vk::PolygonMode;
    fn cull_mode(&self) -> vk::CullModeFlags;
    fn front_face(&self) -> vk::FrontFace;
    fn depth_bias(&self) -> bool;
    fn depth_bias_factor(&self) -> f32;
    fn depth_bias_clamp(&self) -> f32;
    fn depth_bias_slope_factor(&self) -> f32;
    fn line_width(&self) -> f32;
}
pub trait MultisampleStateFactory {
    fn flags(&self) -> Option<vk::PipelineMultisampleStateCreateFlags>;
    fn rasterization_samples(&self) -> vk::SampleCountFlags;
    fn sample_shading(&self) -> bool;
    fn min_sample_shading(&self) -> f32;
    fn sample_mask(&self) -> Option<Vec<vk::SampleMask>>;
    fn alpha_to_converge(&self) -> bool;
    fn alpha_to_one(&self) -> bool;
}
pub trait DepthStencilStateFactory {
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
pub trait ColorBlendStateFactory {
    fn flags(&self) -> Option<vk::PipelineColorBlendStateCreateFlags>;
    fn enable_logic_op(&self) -> bool;
    fn op(&self) -> vk::LogicOp;
    fn blend_constants(&self) -> [f32; 4];
}
pub trait ColorBlendAttachmentFactory {
    fn blend(&self) -> bool;
    fn src_color(&self) -> vk::BlendFactor;
    fn dst_color(&self) -> vk::BlendFactor;
    fn color_op(&self) -> vk::BlendOp;
    fn src_alpha(&self) -> vk::BlendFactor;
    fn dst_alpha(&self) -> vk::BlendFactor;
    fn alpha_op(&self) -> vk::BlendOp;
    fn color_write_mask(&self) -> vk::ColorComponentFlags;
}
pub trait DynamicStateFactory {
    fn flags(&self) -> Option<vk::PipelineDynamicStateCreateFlags>;
    fn dynamics(&self) -> Option<Vec<vk::DynamicState>>;
}
pub struct Graphics<D: DeviceSource, R: RenderPassSource, L: PipelineLayoutSource> {
    device: D,
    _render_pass: R,
    _layout: L,
    pipeline: vk::Pipeline,
}
