use std::ffi::{c_void, CString};
use ash::{self, vk};
use qforce::core::{self};
use cgmath;
use shaderc;


#[cfg(debug_assertions)]
    fn get_vulkan_validate() -> bool{
        println!("Validation Layers Active");
        true
    }
    #[cfg(not(debug_assertions))]
    fn get_vulkan_validate() -> bool {
        println!("Validation Layers Inactive");
        false
    }


#[repr(C)]
    #[derive(Clone)]
    pub struct Vertex{
        pos: [f32; 3],
    }

    
fn main(){
    let _err = pretty_env_logger::try_init();
    let (event_loop, _window, mut engine) = qforce::core::Engine::init(get_vulkan_validate());
    let v_data = [
        Vertex{pos: [ 1.0,-1.0,0.0]},
        Vertex{pos: [ 0.0, 1.0,0.0]},
        Vertex{pos: [-1.0,-1.0,0.0]},
    ];
    let i_data = [1,2,3];
    let objects = [core::ray_tracing::ObjectOutline{ 
        vertex_data: v_data.to_vec(), 
        vertex_format: vk::Format::R32G32B32_SFLOAT, 
        index_data: i_data.to_vec(), 
        inital_pos_data: vec![cgmath::vec4(0.0, 0.0, 1.0, 0.0)],
        sbt_hit_group_offset: 0, }];
    let store = core::ray_tracing::ObjectStore::new(&engine, &objects);

    let tlas = core::ray_tracing::Tlas::new_immediate::<core::Engine,Vertex>(&engine, store.0.get_instance_count(), store.0.get_instance_address());
    
    let d_store = core::memory::DescriptorDataStore::new(&engine);
    let mut tlas_outline = [core::memory::DescriptorSetOutline::new(vk::DescriptorSetLayoutCreateFlags::empty(), 0 as *const c_void, 0 as *const c_void)];
    tlas_outline[0].add_binding(tlas.get_binding(vk::ShaderStageFlags::RAYGEN_KHR));
    let d_stack = d_store.get_descriptor_stack(&tlas_outline, vk::DescriptorPoolCreateFlags::empty(), 0 as *const c_void, 0 as *const c_void);


    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_target_spirv(shaderc::SpirvVersion::V1_6);
    let ray_gen = core::Shader::new(&engine, String::from(r#"
    #version 460
    #extension GL_EXT_ray_tracing : require
    #extension GL_KHR_vulkan_glsl : enable

    layout(binding = 0, set = 0) uniform accelerationStructureEXT topLevelAS;
    layout(binding = 1, set = 0, rgba32f) uniform image2D image;
    void main() 
        {
            imageStore(image, ivec2(gl_LaunchIDEXT.xy), vec4(0.5, 0.5, 0.5, 1.0));
        }
    
    "#), shaderc::ShaderKind::RayGeneration, "main", Some(&options));
    let closest_hit = core::Shader::new(&engine, String::from(r#"
    #version 460
    #extension GL_EXT_ray_tracing : require
    #extension GL_EXT_nonuniform_qualifier : enable
    
    layout(location = 0) rayPayloadInEXT vec3 hitValue;
    hitAttributeEXT vec3 attribs;
    
    void main()
    {
      hitValue = vec3(0.2, 0.5, 0.5);
    }"#), shaderc::ShaderKind::ClosestHit, "main", Some(&options));
    let miss = core::Shader::new(&engine, String::from(r#"
    #version 460
    #extension GL_EXT_ray_tracing : require
    
    layout(location = 0) rayPayloadInEXT vec3 hitValue;
    
    void main()
    {
        hitValue = vec3(0.0, 0.1, 0.3);
    }"#), shaderc::ShaderKind::Miss, "main", Some(&options));

    let main = CString::new("main").unwrap();

    let misses = [miss.get_stage(vk::ShaderStageFlags::MISS_KHR, main.as_c_str())];
    let group_1: [(Option<vk::PipelineShaderStageCreateInfo>, Option<vk::PipelineShaderStageCreateInfo>, Option<vk::PipelineShaderStageCreateInfo>);1] = 
    [(Some(closest_hit.get_stage(vk::ShaderStageFlags::CLOSEST_HIT_KHR, main.as_c_str())), None, None)];

    let sbt_outline = core::ray_tracing::SbtOutline::new(ray_gen.get_stage(vk::ShaderStageFlags::RAYGEN_KHR, &main), &misses, &group_1);

    let ray_pipeline = core::ray_tracing::RayTracingPipeline::new_immediate(&engine, sbt_outline, &[d_stack.get_set_layout(0)], &[]);


    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            winit::event::Event::NewEvents(_) => {},
            winit::event::Event::WindowEvent {event, .. } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                        drop(&store);
                        drop(&tlas);
                        drop(&d_stack);
                        drop(&ray_gen);
                        drop(&closest_hit);
                        drop(&miss);
                        drop(&ray_pipeline)
                },
                    winit::event::WindowEvent::Resized(_) => {
                        engine.refresh_swapchain();
                    
                    }
                    _ => {}
                }
            },
            winit::event::Event::DeviceEvent { .. } => {},
            winit::event::Event::UserEvent(_) => {},
            winit::event::Event::Suspended => {},
            winit::event::Event::Resumed => {},
            winit::event::Event::MainEventsCleared => {},
            winit::event::Event::RedrawRequested(_) => {},
            winit::event::Event::RedrawEventsCleared => {},
            winit::event::Event::LoopDestroyed => {
                println!("Shutting down program")
            },
        }
    });

}