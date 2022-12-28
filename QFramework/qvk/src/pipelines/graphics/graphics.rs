use std::sync::Arc;

use ash::vk;
use log::{info, debug};

use crate::{init::DeviceSource, pipelines::PipelineLayoutSource, shader::ShaderSource};

use super::{Graphics, RenderPassSource, VertexStateFactory, TesselationStateFactory, InputStateFactory, ViewportStateFactory, RasterizationStateFactory, MultisampleStateFactory, DepthStencilStateFactory, ColorBlendStateFactory, ColorBlendAttachmentFactory, DynamicStateFactory, GraphicsPipelineFactory, GraphicsPipelineState, GraphicsPipelineSource};

pub struct State<Shd:ShaderSource>{
    _flags:  Option<vk::PipelineCreateFlags>,
    shader_stages:  Vec<vk::PipelineShaderStageCreateInfo>,
    shaders: Vec<Shd>,
    vertex_bindings: Vec<vk::VertexInputBindingDescription>,
    vertex_attributes: Vec<vk::VertexInputAttributeDescription>,
    vertex_state:  vk::PipelineVertexInputStateCreateInfo,
    input_assembly_state:  vk::PipelineInputAssemblyStateCreateInfo,
    tesselation_state:  vk::PipelineTessellationStateCreateInfo,
    viewports: Vec<vk::Viewport>,
    scissors: Vec<vk::Rect2D>,
    viewport_state:  vk::PipelineViewportStateCreateInfo,
    rasterization_state:  vk::PipelineRasterizationStateCreateInfo,
    sample_mask: Vec<u32>,
    multisample_state:  vk::PipelineMultisampleStateCreateInfo,
    depth_stencil_state:  vk::PipelineDepthStencilStateCreateInfo,
    blend_attachments: Vec<vk::PipelineColorBlendAttachmentState>,
    color_blend_state:  vk::PipelineColorBlendStateCreateInfo,
    dynamics: Vec<vk::DynamicState>,
    dynamic_state:  vk::PipelineDynamicStateCreateInfo,
    
}

impl<D:DeviceSource + Clone, R:RenderPassSource + Clone, L:PipelineLayoutSource + Clone, S:GraphicsPipelineState> GraphicsPipelineFactory<Arc<Graphics<D,R,L>>, S, L, R> for D{
    fn create_graphics_pipeline(&self, state: &S, layout: &L, renderpass: &R, tgt_subpass: u32) -> Result<Arc<Graphics<D,R,L>>, vk::Result> {
        let mut info = vk::GraphicsPipelineCreateInfo::builder();
        if let Some(flags) = state.flags(){
            info = info.flags(flags);
        }

        let stages = state.shader_stages();
        info = info.stages(stages);

        let vertex_input = state.vertex_state();
        info = info.vertex_input_state(&vertex_input);

        let assembly = state.input_assembly_state();
        info = info.input_assembly_state(&assembly);

        let tesselatiom = state.tesselation_state();
        info = info.tessellation_state(&tesselatiom);

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
        info = info.dynamic_state(&dynamic);

        info = info.layout(layout.layout());

        info = info.subpass(tgt_subpass);
        info = info.render_pass(renderpass.renderpass());
        
        let info = [info.build()];
        let graphics;
        unsafe{
            graphics = self.device().create_graphics_pipelines(vk::PipelineCache::null(), &info, None);
        }

        if let Err(e) = graphics{
            return Err(e.1);
        }

        let graphics = graphics.unwrap()[0];

        info!("Created graphics pipeline {:?}", graphics);

        Ok(Arc::new(
            Graphics{
                device: self.clone(),
                pipeline: graphics,
                _render_pass: renderpass.clone(),
                _layout: layout.clone(),
            }
        ))
    }
}

impl<D:DeviceSource,R:RenderPassSource,L:PipelineLayoutSource> Drop for Graphics<D,R,L>{
    fn drop(&mut self) {
        debug!("Destroyed graphics pipeline {:?}", self.pipeline);
        unsafe{
            self.device.device().destroy_pipeline(self.pipeline, None);
        }
    }
}

impl<Shd:ShaderSource + Clone> State<Shd>{
    pub fn new(flags: Option<vk::PipelineCreateFlags>) -> State<Shd> {
        Self{
            shader_stages:  vec![],
            shaders: vec![],
            vertex_bindings: vec![],
            vertex_attributes: vec![],
            vertex_state:  vk::PipelineVertexInputStateCreateInfo::default(),
            input_assembly_state:  vk::PipelineInputAssemblyStateCreateInfo::default(),
            tesselation_state:  vk::PipelineTessellationStateCreateInfo::default(),
            viewport_state:  vk::PipelineViewportStateCreateInfo::default(),
            rasterization_state:  vk::PipelineRasterizationStateCreateInfo::default(),
            multisample_state:  vk::PipelineMultisampleStateCreateInfo::default(),
            depth_stencil_state:  vk::PipelineDepthStencilStateCreateInfo::default(),
            color_blend_state:  vk::PipelineColorBlendStateCreateInfo::default(),
            dynamic_state:  vk::PipelineDynamicStateCreateInfo::default(),
            viewports: vec![],
            scissors: vec![],
            sample_mask: vec![],
            blend_attachments: vec![],
            dynamics: vec![],
            _flags: flags,
        }
    }

    pub fn add_shader(&mut self, shader: &Shd){
        let stage = shader.stage();
        self.shader_stages.push(stage);
        self.shaders.push(shader.clone());
    }
    pub fn set_vertex_state<V:VertexStateFactory>(&mut self, factory: &V){
        self.vertex_bindings = factory.bindings().to_vec();
        self.vertex_attributes = factory.attributes().to_vec();
        let mut info = vk::PipelineVertexInputStateCreateInfo::builder();
        if let Some(flags) = factory.flags(){
            info = info.flags(flags);
        }

        self.vertex_state = info
            .vertex_binding_descriptions(&self.vertex_bindings)
            .vertex_attribute_descriptions(&self.vertex_attributes)
            .build();
        
    }
    pub fn set_input_state<I:InputStateFactory>(&mut self, factory: &I){
        let mut info = vk::PipelineInputAssemblyStateCreateInfo::builder();
        if let Some(flags) = factory.flags(){
            info = info.flags(flags);
        }
        info = info.topology(factory.topology());
        info = info.primitive_restart_enable(factory.restart_enabled());
        self.input_assembly_state = info.build();
    }
    pub fn set_tessalation_state<T:TesselationStateFactory>(&mut self, factory: &T){
        let mut info = vk::PipelineTessellationStateCreateInfo::builder();
        if let Some(flags) = factory.flags(){
            info = info.flags(flags);
        }

        info = info.patch_control_points(factory.patch_control_points());
        self.tesselation_state = info.build();
    }
    pub fn set_viewport_state<V:ViewportStateFactory>(&mut self, factory: &V){
        let mut info = vk::PipelineViewportStateCreateInfo::builder();
        if let Some(flags) = factory.flags(){
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
    pub fn set_rasterization_state<R:RasterizationStateFactory>(&mut self, factory: &R){
        let mut info = vk::PipelineRasterizationStateCreateInfo::builder();
        if let Some(flags) = factory.flags(){
            info = info.flags(flags);
        }
        info = info
        .depth_clamp_enable(factory.depth_clamp())
        .rasterizer_discard_enable(factory.raster_discard())
        .polygon_mode(factory.polygon_mod())
        .cull_mode(factory.cull_mode())
        .front_face(factory.front_face())
        .depth_bias_enable(factory.depth_bias())
        .depth_bias_constant_factor(factory.depth_bias_factor())
        .depth_bias_clamp(factory.depth_bias_clamp())
        .depth_bias_slope_factor(factory.depth_bias_slope_factor())
        .line_width(factory.line_width());
        self.rasterization_state = info.build();
    }
    pub fn set_multisample_state<M:MultisampleStateFactory>(&mut self, factory: &M){
        let mut info = vk::PipelineMultisampleStateCreateInfo::builder();
        if let Some(flags) = factory.flags(){
            info = info.flags(flags);
        }
        self.sample_mask = factory.sample_mask().to_vec();
        info = info
        .rasterization_samples(factory.rasterization_samples())
        .sample_shading_enable(factory.sample_shading())
        .min_sample_shading(factory.min_sample_shading())
        .sample_mask(&self.sample_mask)
        .alpha_to_coverage_enable(factory.alpha_to_converge())
        .alpha_to_one_enable(factory.alpha_to_one());
        self.multisample_state = info.build();
    }
    pub fn set_depth_stencil_state<D:DepthStencilStateFactory>(&mut self, factory: &D){
        let mut info = vk::PipelineDepthStencilStateCreateInfo::builder();
        if let Some(flags) = factory.flags(){
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
    pub fn set_color_blend_state<CB:ColorBlendStateFactory, CA:ColorBlendAttachmentFactory>(&mut self, factory: &CB, attachments: &[&CA]){
        let mut _attachments = Vec::with_capacity(attachments.len());
        for a in attachments.iter(){
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
        if let Some(flags) = factory.flags(){
            info = info.flags(flags);
        }
        info = info
        .logic_op_enable(factory.enable_logic_op())
        .logic_op(factory.op())
        .attachments(&self.blend_attachments)
        .blend_constants(factory.blend_constants());
        self.color_blend_state = info.build();
    }
    pub fn set_dynamic_state<D:DynamicStateFactory>(&mut self, factory: &D){
        let mut info = vk::PipelineDynamicStateCreateInfo::builder();
        if let Some(flags) = factory.flags(){
            info = info.flags(flags);
        }
        self.dynamics = factory.dynamics().to_vec();
        info = info.dynamic_states(&self.dynamics);
        self.dynamic_state = info.build();
    }
}

impl<D:DeviceSource, L:PipelineLayoutSource, R:RenderPassSource> GraphicsPipelineSource for Arc<Graphics<D,R,L>>{
    
}
