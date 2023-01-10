use std::{mem::size_of, sync::Arc};

use ash::vk;
use log::{debug, info};

use crate::{
    command::BindPipelineFactory, init::DeviceSource, pipelines::PipelineLayoutSource,
    shader::ShaderSource,
};

use super::{
    ColorBlendAttachmentFactory, ColorBlendStateFactory, DepthStencilStateFactory,
    DynamicStateFactory, Graphics, GraphicsPipelineFactory, GraphicsPipelineSource,
    GraphicsPipelineState, InputStateFactory, MultisampleStateFactory, RasterizationStateFactory,
    RenderPassSource, RenderpassAttachmentSource, TesselationStateFactory, VertexStateFactory,
    ViewportStateFactory,
};

pub struct State<Shd: ShaderSource> {
    _flags: Option<vk::PipelineCreateFlags>,
    shader_stages: Vec<vk::PipelineShaderStageCreateInfo>,
    shaders: Vec<Shd>,
    vertex_bindings: Vec<vk::VertexInputBindingDescription>,
    vertex_attributes: Vec<vk::VertexInputAttributeDescription>,
    vertex_state: vk::PipelineVertexInputStateCreateInfo,
    input_assembly_state: vk::PipelineInputAssemblyStateCreateInfo,
    tesselation_state: Option<vk::PipelineTessellationStateCreateInfo>,
    viewports: Vec<vk::Viewport>,
    scissors: Vec<vk::Rect2D>,
    viewport_state: vk::PipelineViewportStateCreateInfo,
    rasterization_state: vk::PipelineRasterizationStateCreateInfo,
    sample_mask: Vec<u32>,
    multisample_state: vk::PipelineMultisampleStateCreateInfo,
    depth_stencil_state: vk::PipelineDepthStencilStateCreateInfo,
    blend_attachments: Vec<vk::PipelineColorBlendAttachmentState>,
    color_blend_state: vk::PipelineColorBlendStateCreateInfo,
    dynamics: Vec<vk::DynamicState>,
    dynamic_state: Option<vk::PipelineDynamicStateCreateInfo>,
}

impl<
        A: RenderpassAttachmentSource,
        D: DeviceSource + Clone,
        R: RenderPassSource<A> + Clone,
        L: PipelineLayoutSource + Clone,
        S: GraphicsPipelineState,
    > GraphicsPipelineFactory<A, Arc<Graphics<D, A, R, L>>, S, L, R> for D
{
    fn create_graphics_pipeline(
        &self,
        state: &S,
        layout: &L,
        renderpass: &R,
        tgt_subpass: u32,
    ) -> Result<Arc<Graphics<D, A, R, L>>, vk::Result> {
        let mut info = vk::GraphicsPipelineCreateInfo::builder();
        if let Some(flags) = state.flags() {
            info = info.flags(flags);
        }

        let stages = state.shader_stages();
        info = info.stages(stages);

        let vertex_input = state.vertex_state();
        info = info.vertex_input_state(&vertex_input);

        let assembly = state.input_assembly_state();
        info = info.input_assembly_state(&assembly);

        let tesselatiom = state.tesselation_state();
        if let Some(t) = &tesselatiom {
            info = info.tessellation_state(t);
        }

        let viewport = state.viewport_state();
        info = info.viewport_state(&viewport);

        let rasterization = state.rasterization_state();
        info = info.rasterization_state(&rasterization);

        let multisample = state.multisample_state();
        info = info.multisample_state(&multisample);

        let depth_stencil = state.depth_stencil_state();
        info = info.depth_stencil_state(&depth_stencil);

        let color_blend = state.color_blend_state();
        info = info.color_blend_state(&color_blend);

        let dynamic = state.dynamic_state();
        if let Some(d) = &dynamic {
            info = info.dynamic_state(d);
        }

        info = info.layout(layout.layout());

        info = info.subpass(tgt_subpass);
        info = info.render_pass(renderpass.renderpass());

        let info = [info.build()];
        let graphics;
        unsafe {
            graphics =
                self.device()
                    .create_graphics_pipelines(vk::PipelineCache::null(), &info, None);
        }

        if let Err(e) = graphics {
            return Err(e.1);
        }

        let graphics = graphics.unwrap()[0];

        info!("Created graphics pipeline {:?}", graphics);

        Ok(Arc::new(Graphics {
            device: self.clone(),
            pipeline: graphics,
            _render_pass: renderpass.clone(),
            _layout: layout.clone(),
            _attch: std::marker::PhantomData,
        }))
    }
}

impl<
        A: RenderpassAttachmentSource,
        D: DeviceSource,
        R: RenderPassSource<A>,
        L: PipelineLayoutSource,
    > Drop for Graphics<D, A, R, L>
{
    fn drop(&mut self) {
        debug!("Destroyed graphics pipeline {:?}", self.pipeline);
        unsafe {
            self.device.device().destroy_pipeline(self.pipeline, None);
        }
    }
}

impl<Shd: ShaderSource + Clone> State<Shd> {
    pub fn new(flags: Option<vk::PipelineCreateFlags>) -> State<Shd> {
        Self {
            shader_stages: vec![],
            shaders: vec![],
            vertex_bindings: vec![],
            vertex_attributes: vec![],
            vertex_state: vk::PipelineVertexInputStateCreateInfo::default(),
            input_assembly_state: vk::PipelineInputAssemblyStateCreateInfo::default(),
            tesselation_state: None,
            viewport_state: vk::PipelineViewportStateCreateInfo::default(),
            rasterization_state: vk::PipelineRasterizationStateCreateInfo::default(),
            multisample_state: vk::PipelineMultisampleStateCreateInfo::default(),
            depth_stencil_state: vk::PipelineDepthStencilStateCreateInfo::default(),
            color_blend_state: vk::PipelineColorBlendStateCreateInfo::default(),
            dynamic_state: None,
            viewports: vec![],
            scissors: vec![],
            sample_mask: vec![],
            blend_attachments: vec![],
            dynamics: vec![],
            _flags: flags,
        }
    }

    pub fn add_shader(&mut self, shader: &Shd) {
        let stage = shader.stage();
        self.shader_stages.push(stage);
        self.shaders.push(shader.clone());
    }
    pub fn set_vertex_state<V: VertexStateFactory>(&mut self, factory: &V) {
        self.vertex_bindings = factory.bindings().to_vec();
        self.vertex_attributes = factory.attributes().to_vec();
        let mut info = vk::PipelineVertexInputStateCreateInfo::builder();
        if let Some(flags) = factory.flags() {
            info = info.flags(flags);
        }

        self.vertex_state = info
            .vertex_binding_descriptions(&self.vertex_bindings)
            .vertex_attribute_descriptions(&self.vertex_attributes)
            .build();
    }
    pub fn set_input_state<I: InputStateFactory>(&mut self, factory: &I) {
        let mut info = vk::PipelineInputAssemblyStateCreateInfo::builder();
        if let Some(flags) = factory.flags() {
            info = info.flags(flags);
        }
        info = info.topology(factory.topology());
        info = info.primitive_restart_enable(factory.restart_enabled());
        self.input_assembly_state = info.build();
    }
    pub fn set_tessalation_state<T: TesselationStateFactory>(&mut self, factory: &T) {
        let mut info = vk::PipelineTessellationStateCreateInfo::builder();
        if let Some(flags) = factory.flags() {
            info = info.flags(flags);
        }
        if let Some(c) = factory.patch_control_points() {
            info = info.patch_control_points(c);
            self.tesselation_state = Some(info.build());
        }
        self.tesselation_state = None;
    }
    pub fn set_viewport_state<V: ViewportStateFactory>(&mut self, factory: &V) {
        let mut info = vk::PipelineViewportStateCreateInfo::builder();
        if let Some(flags) = factory.flags() {
            info = info.flags(flags);
        }
        self.viewports = factory.viewports().to_vec();
        self.scissors = factory.scissors().to_vec();
        info = info.viewport_count(factory.viewports().len() as u32);
        info = info.viewports(&self.viewports);
        info = info.scissor_count(factory.scissors().len() as u32);
        info = info.scissors(&self.scissors);
        self.viewport_state = info.build();
    }
    pub fn set_rasterization_state<R: RasterizationStateFactory>(&mut self, factory: &R) {
        let mut info = vk::PipelineRasterizationStateCreateInfo::builder();
        if let Some(flags) = factory.flags() {
            info = info.flags(flags);
        }
        info = info
            .depth_clamp_enable(factory.depth_clamp())
            .rasterizer_discard_enable(factory.raster_discard())
            .polygon_mode(factory.polygon_mode())
            .cull_mode(factory.cull_mode())
            .front_face(factory.front_face())
            .depth_bias_enable(factory.depth_bias())
            .depth_bias_constant_factor(factory.depth_bias_factor())
            .depth_bias_clamp(factory.depth_bias_clamp())
            .depth_bias_slope_factor(factory.depth_bias_slope_factor())
            .line_width(factory.line_width());
        self.rasterization_state = info.build();
    }
    pub fn set_multisample_state<M: MultisampleStateFactory>(&mut self, factory: &M) {
        let mut info = vk::PipelineMultisampleStateCreateInfo::builder();
        if let Some(flags) = factory.flags() {
            info = info.flags(flags);
        }

        if let Some(s) = factory.sample_mask() {
            self.sample_mask = s;
        }
        info = info
            .rasterization_samples(factory.rasterization_samples())
            .sample_shading_enable(factory.sample_shading())
            .min_sample_shading(factory.min_sample_shading())
            .sample_mask(&self.sample_mask)
            .alpha_to_coverage_enable(factory.alpha_to_converge())
            .alpha_to_one_enable(factory.alpha_to_one());
        self.multisample_state = info.build();
    }
    pub fn set_depth_stencil_state<D: DepthStencilStateFactory>(&mut self, factory: &D) {
        let mut info = vk::PipelineDepthStencilStateCreateInfo::builder();
        if let Some(flags) = factory.flags() {
            info = info.flags(flags);
        }
        info = info
            .depth_test_enable(factory.depth_test())
            .depth_write_enable(factory.depth_write())
            .depth_compare_op(factory.compare_op())
            .depth_bounds_test_enable(factory.depth_bounds_test())
            .stencil_test_enable(factory.stencil_test())
            .front(factory.front())
            .back(factory.back())
            .min_depth_bounds(factory.min_depth())
            .max_depth_bounds(factory.max_depth());
        self.depth_stencil_state = info.build();
    }
    pub fn set_color_blend_state<CB: ColorBlendStateFactory, CA: ColorBlendAttachmentFactory>(
        &mut self,
        factory: &CB,
        attachments: &[&CA],
    ) {
        let mut _attachments = Vec::with_capacity(attachments.len());
        for a in attachments.iter() {
            let info = vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(a.blend())
                .src_color_blend_factor(a.src_color())
                .dst_color_blend_factor(a.dst_color())
                .color_blend_op(a.color_op())
                .src_alpha_blend_factor(a.src_alpha())
                .dst_alpha_blend_factor(a.dst_alpha())
                .alpha_blend_op(a.alpha_op())
                .color_write_mask(a.color_write_mask())
                .build();
            _attachments.push(info);
        }

        self.blend_attachments = _attachments;

        let mut info = vk::PipelineColorBlendStateCreateInfo::builder();
        if let Some(flags) = factory.flags() {
            info = info.flags(flags);
        }
        info = info
            .logic_op_enable(factory.enable_logic_op())
            .logic_op(factory.op())
            .attachments(&self.blend_attachments)
            .blend_constants(factory.blend_constants());
        self.color_blend_state = info.build();
    }
    pub fn set_dynamic_state<D: DynamicStateFactory>(&mut self, factory: &D) {
        let mut info = vk::PipelineDynamicStateCreateInfo::builder();
        if let Some(flags) = factory.flags() {
            info = info.flags(flags);
        }
        if let Some(s) = factory.dynamics() {
            self.dynamics = s.clone();
            info = info.dynamic_states(&self.dynamics);
            self.dynamic_state = Some(info.build());
        }
        self.dynamic_state = None;
    }
}

impl<
        A: RenderpassAttachmentSource,
        D: DeviceSource,
        L: PipelineLayoutSource,
        R: RenderPassSource<A>,
    > GraphicsPipelineSource for Arc<Graphics<D, A, R, L>>
{
}

impl<
        A: RenderpassAttachmentSource,
        D: DeviceSource,
        L: PipelineLayoutSource,
        R: RenderPassSource<A>,
    > BindPipelineFactory for Arc<Graphics<D, A, R, L>>
{
    fn layout(&self) -> vk::PipelineLayout {
        self._layout.layout()
    }

    fn bind_point(&self) -> vk::PipelineBindPoint {
        vk::PipelineBindPoint::GRAPHICS
    }

    fn pipeline(&self) -> vk::Pipeline {
        self.pipeline
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct DefaultVertex {
    ///3 f32 for pos, 3 f32 for color
    pub data: [f32; 6],
}
#[derive(Clone)]
pub struct GraphicsDefaultState<V: VertexStateFactory> {
    viewports: Vec<vk::Viewport>,
    scissors: Vec<vk::Rect2D>,
    vertex_type: V,
}
impl VertexStateFactory for DefaultVertex {
    fn flags(&self) -> Option<vk::PipelineVertexInputStateCreateFlags> {
        None
    }

    fn bindings(&self) -> Vec<vk::VertexInputBindingDescription> {
        let b1 = vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build();
        vec![b1]
    }

    fn attributes(&self) -> Vec<vk::VertexInputAttributeDescription> {
        let att1 = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();
        let att2 = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<f32>() as u32 * 3)
            .build();

        vec![att1, att2]
    }
}
impl<V: VertexStateFactory + Default> GraphicsDefaultState<V> {
    pub fn new(viewport_extent: vk::Extent3D) -> GraphicsDefaultState<V> {
        let viewport = vk::Viewport::builder()
            .x(0.0)
            .y(viewport_extent.height as f32)
            .width(viewport_extent.width as f32)
            .height(-(viewport_extent.height as f32))
            .min_depth(0.0)
            .max_depth(viewport_extent.depth as f32)
            .build();

        let scissor = vk::Rect2D::builder()
            .offset(vk::Offset2D { x: 0, y: 0 })
            .extent(
                vk::Extent2D::builder()
                    .width(viewport_extent.width)
                    .height(viewport_extent.height)
                    .build(),
            )
            .build();
        GraphicsDefaultState {
            viewports: vec![viewport],
            scissors: vec![scissor],
            vertex_type: V::default(),
        }
    }
    pub fn create_state<Shd: ShaderSource + Clone>(&self, shaders: &[&Shd]) -> State<Shd> {
        let mut state = State::<Shd>::new(None);
        for shd in shaders.iter() {
            state.add_shader(shd.clone());
        }
        state.set_vertex_state(self);
        state.set_input_state(self);
        state.set_tessalation_state(self);
        state.set_viewport_state(self);
        state.set_rasterization_state(self);
        state.set_multisample_state(self);
        state.set_depth_stencil_state(self);
        let attachments = [self];
        state.set_color_blend_state(self, &attachments);
        state.set_dynamic_state(self);
        state
    }
}

impl<V: VertexStateFactory> VertexStateFactory for GraphicsDefaultState<V> {
    fn flags(&self) -> Option<vk::PipelineVertexInputStateCreateFlags> {
        self.vertex_type.flags()
    }

    fn bindings(&self) -> Vec<vk::VertexInputBindingDescription> {
        self.vertex_type.bindings()
    }

    fn attributes(&self) -> Vec<vk::VertexInputAttributeDescription> {
        self.vertex_type.attributes()
    }
}

impl<V: VertexStateFactory> InputStateFactory for GraphicsDefaultState<V> {
    fn flags(&self) -> Option<vk::PipelineInputAssemblyStateCreateFlags> {
        None
    }

    fn topology(&self) -> vk::PrimitiveTopology {
        vk::PrimitiveTopology::TRIANGLE_LIST
    }

    fn restart_enabled(&self) -> bool {
        false
    }
}
impl<V: VertexStateFactory> TesselationStateFactory for GraphicsDefaultState<V> {
    fn flags(&self) -> Option<vk::PipelineTessellationStateCreateFlags> {
        None
    }

    fn patch_control_points(&self) -> Option<u32> {
        None
    }
}

impl<V: VertexStateFactory> ViewportStateFactory for GraphicsDefaultState<V> {
    fn flags(&self) -> Option<vk::PipelineViewportStateCreateFlags> {
        None
    }

    fn viewports(&self) -> Vec<vk::Viewport> {
        self.viewports.clone()
    }

    fn scissors(&self) -> Vec<vk::Rect2D> {
        self.scissors.clone()
    }
}

impl<V: VertexStateFactory> RasterizationStateFactory for GraphicsDefaultState<V> {
    fn flags(&self) -> Option<vk::PipelineRasterizationStateCreateFlags> {
        None
    }

    fn depth_clamp(&self) -> bool {
        false
    }

    fn raster_discard(&self) -> bool {
        false
    }

    fn polygon_mode(&self) -> vk::PolygonMode {
        vk::PolygonMode::FILL
    }

    fn cull_mode(&self) -> vk::CullModeFlags {
        vk::CullModeFlags::NONE
    }

    fn front_face(&self) -> vk::FrontFace {
        vk::FrontFace::COUNTER_CLOCKWISE
    }

    fn depth_bias(&self) -> bool {
        false
    }

    fn depth_bias_factor(&self) -> f32 {
        0.0
    }

    fn depth_bias_clamp(&self) -> f32 {
        0.0
    }

    fn depth_bias_slope_factor(&self) -> f32 {
        0.0
    }

    fn line_width(&self) -> f32 {
        1.0
    }
}
impl<V: VertexStateFactory> MultisampleStateFactory for GraphicsDefaultState<V> {
    fn flags(&self) -> Option<vk::PipelineMultisampleStateCreateFlags> {
        None
    }

    fn rasterization_samples(&self) -> vk::SampleCountFlags {
        vk::SampleCountFlags::TYPE_1
    }

    fn sample_shading(&self) -> bool {
        false
    }

    fn min_sample_shading(&self) -> f32 {
        1.0
    }

    fn sample_mask(&self) -> Option<Vec<u32>> {
        None
    }

    fn alpha_to_converge(&self) -> bool {
        false
    }

    fn alpha_to_one(&self) -> bool {
        false
    }
}

impl<V: VertexStateFactory> DepthStencilStateFactory for GraphicsDefaultState<V> {
    fn flags(&self) -> Option<vk::PipelineDepthStencilStateCreateFlags> {
        None
    }

    fn depth_test(&self) -> bool {
        true
    }

    fn depth_write(&self) -> bool {
        true
    }

    fn compare_op(&self) -> vk::CompareOp {
        vk::CompareOp::LESS
    }

    fn depth_bounds_test(&self) -> bool {
        false
    }

    fn stencil_test(&self) -> bool {
        false
    }

    fn front(&self) -> vk::StencilOpState {
        vk::StencilOpState::default()
    }

    fn back(&self) -> vk::StencilOpState {
        vk::StencilOpState::default()
    }

    fn min_depth(&self) -> f32 {
        0.0
    }

    fn max_depth(&self) -> f32 {
        1.0
    }
}

impl<V: VertexStateFactory> ColorBlendStateFactory for GraphicsDefaultState<V> {
    fn flags(&self) -> Option<vk::PipelineColorBlendStateCreateFlags> {
        None
    }

    fn enable_logic_op(&self) -> bool {
        false
    }

    fn op(&self) -> vk::LogicOp {
        vk::LogicOp::default()
    }

    fn blend_constants(&self) -> [f32; 4] {
        [0.0; 4]
    }
}
impl<V: VertexStateFactory> ColorBlendAttachmentFactory for GraphicsDefaultState<V> {
    fn blend(&self) -> bool {
        false
    }

    fn src_color(&self) -> vk::BlendFactor {
        vk::BlendFactor::ONE
    }

    fn dst_color(&self) -> vk::BlendFactor {
        vk::BlendFactor::ZERO
    }

    fn color_op(&self) -> vk::BlendOp {
        vk::BlendOp::ADD
    }

    fn src_alpha(&self) -> vk::BlendFactor {
        vk::BlendFactor::ONE
    }

    fn dst_alpha(&self) -> vk::BlendFactor {
        vk::BlendFactor::ZERO
    }

    fn alpha_op(&self) -> vk::BlendOp {
        vk::BlendOp::ADD
    }

    fn color_write_mask(&self) -> vk::ColorComponentFlags {
        vk::ColorComponentFlags::R
            | vk::ColorComponentFlags::G
            | vk::ColorComponentFlags::B
            | vk::ColorComponentFlags::A
    }
}
impl<V: VertexStateFactory> DynamicStateFactory for GraphicsDefaultState<V> {
    fn flags(&self) -> Option<vk::PipelineDynamicStateCreateFlags> {
        None
    }

    fn dynamics(&self) -> Option<Vec<vk::DynamicState>> {
        None
    }
}
impl<Shd: ShaderSource + Clone> GraphicsPipelineState for State<Shd> {
    fn flags(&self) -> Option<vk::PipelineCreateFlags> {
        self._flags
    }

    fn shader_stages(&self) -> &[vk::PipelineShaderStageCreateInfo] {
        &self.shader_stages
    }

    fn vertex_state(&self) -> vk::PipelineVertexInputStateCreateInfo {
        self.vertex_state
    }

    fn input_assembly_state(&self) -> vk::PipelineInputAssemblyStateCreateInfo {
        self.input_assembly_state
    }

    fn tesselation_state(&self) -> Option<vk::PipelineTessellationStateCreateInfo> {
        self.tesselation_state
    }

    fn viewport_state(&self) -> vk::PipelineViewportStateCreateInfo {
        self.viewport_state
    }

    fn rasterization_state(&self) -> vk::PipelineRasterizationStateCreateInfo {
        self.rasterization_state
    }

    fn multisample_state(&self) -> vk::PipelineMultisampleStateCreateInfo {
        self.multisample_state
    }

    fn depth_stencil_state(&self) -> vk::PipelineDepthStencilStateCreateInfo {
        self.depth_stencil_state
    }

    fn color_blend_state(&self) -> vk::PipelineColorBlendStateCreateInfo {
        self.color_blend_state
    }

    fn dynamic_state(&self) -> Option<vk::PipelineDynamicStateCreateInfo> {
        self.dynamic_state
    }
}

impl Default for DefaultVertex {
    fn default() -> Self {
        Self { data: [0.0; 6] }
    }
}
