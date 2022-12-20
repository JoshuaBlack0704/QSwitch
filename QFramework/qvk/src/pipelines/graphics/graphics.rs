use std::sync::Arc;

use ash::vk;
use log::{info, debug};

use crate::{init::DeviceStore, pipelines::PipelineLayoutStore, shader::ShaderStore};

use super::{Graphics, RenderPassStore, VertexStateFactory, TesselationStateFactory};

pub trait GraphicsPipelineState{
    fn flags(&self) -> Option<vk::PipelineCreateFlags>;
    fn shader_stages(&self) -> &[vk::PipelineShaderStageCreateInfo];
    fn vertex_state(&self) -> vk::PipelineVertexInputStateCreateInfo;
    fn input_assembly_state(&self) -> vk::PipelineInputAssemblyStateCreateInfo;
    fn tesselation_state(&self) -> vk::PipelineTessellationStateCreateInfo;
    fn viewport_state(&self) -> vk::PipelineViewportStateCreateInfo;
    fn rasterization_state(&self) -> vk::PipelineRasterizationStateCreateInfo;
    fn multisample_state(&self) -> vk::PipelineMultisampleStateCreateInfo;
    fn depth_stencil_state(&self) -> vk::PipelineDepthStencilStateCreateInfo;
    fn color_blend_state(&self) -> vk::PipelineColorBlendStateCreateInfo;
    fn dynamic_state(&self) -> vk::PipelineDynamicStateCreateInfo;
      
}

pub struct State<Shd:ShaderStore>{
    flags:  Option<vk::PipelineCreateFlags>,
    shader_stages:  Vec<vk::PipelineShaderStageCreateInfo>,
    shaders: Vec<Shd>,
    vertex_bindings: Vec<vk::VertexInputBindingDescription>,
    vertex_attributes: Vec<vk::VertexInputAttributeDescription>,
    vertex_state:  vk::PipelineVertexInputStateCreateInfo,
    input_assembly_state:  vk::PipelineInputAssemblyStateCreateInfo,
    tesselation_state:  vk::PipelineTessellationStateCreateInfo,
    viewport_state:  vk::PipelineViewportStateCreateInfo,
    rasterization_state:  vk::PipelineRasterizationStateCreateInfo,
    multisample_state:  vk::PipelineMultisampleStateCreateInfo,
    depth_stencil_state:  vk::PipelineDepthStencilStateCreateInfo,
    color_blend_state:  vk::PipelineColorBlendStateCreateInfo,
    dynamic_state:  vk::PipelineDynamicStateCreateInfo,
    
}

impl<D:DeviceStore + Clone, R:RenderPassStore + Clone, L:PipelineLayoutStore + Clone> Graphics<D,R,L>{
    pub fn new<S:GraphicsPipelineState>(device_provider: &D, state: &S, layout: &L, renderpass: &R, tgt_subpass: u32) -> Result<Arc<Self>, vk::Result>{
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
            graphics = device_provider.device().create_graphics_pipelines(vk::PipelineCache::null(), &info, None);
        }

        if let Err(e) = graphics{
            return Err(e.1);
        }

        let graphics = graphics.unwrap()[0];

        info!("Created graphics pipeline {:?}", graphics);

        Ok(Arc::new(
            Self{
                device: device_provider.clone(),
                pipeline: graphics,
                _render_pass: renderpass.clone(),
                _layout: layout.clone(),
            }
        ))
    }
}

impl<D:DeviceStore,R:RenderPassStore,L:PipelineLayoutStore> Drop for Graphics<D,R,L>{
    fn drop(&mut self) {
        debug!("Destroyed graphics pipeline {:?}", self.pipeline);
        unsafe{
            self.device.device().destroy_pipeline(self.pipeline, None);
        }
    }
}

impl<Shd:ShaderStore + Clone> State<Shd>{
    pub fn new(flags: Option<vk::PipelineCreateFlags>) -> State<Shd> {
        Self{
            flags,
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
    pub fn set_input_assembly<T:TesselationStateFactory>(&mut self, factory: &T){
        
    }
}

