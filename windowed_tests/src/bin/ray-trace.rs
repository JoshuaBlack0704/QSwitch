use std::{ffi::{c_void, CString}, sync::Arc, rc::Rc, cell::RefCell};
use ash::{self, vk};
use qforce::{core::{self}, traits::{IWindowedEngineData, IEngineData, ICommandPool}};
use cgmath;
use shaderc;
use time::Instant;


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

    let swapchain_extent_3d = vk::Extent3D::builder()
    .width(engine.swapchain_info().image_extent.width)
    .height(engine.swapchain_info().image_extent.height)
    .depth(1)
    .build();
    let render_target;
    let render_target_view;
    let render_target_memory;
    let subresource = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1)
        .build();
{
    let ic_info = vk::ImageCreateInfo::builder()
    .image_type(vk::ImageType::TYPE_2D)
    .format(engine.swapchain_info().image_format)
    .extent(swapchain_extent_3d)
    .mip_levels(1)
    .array_layers(1)
    .samples(vk::SampleCountFlags::TYPE_1)
    .tiling(vk::ImageTiling::OPTIMAL)
    .usage(vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC)
    .initial_layout(vk::ImageLayout::UNDEFINED)
    .build();
    println!("{:?}", ic_info);
    render_target = unsafe{engine.device().create_image(&ic_info, None).expect("Could not create image")};
    
    let reqs = unsafe{engine.device().get_image_memory_requirements(render_target)};
    let aloc_info = vk::MemoryAllocateInfo::builder()
    .allocation_size(reqs.size)
    .memory_type_index(core::memory::AllocationDataStore::new(&engine).get_type(vk::MemoryPropertyFlags::DEVICE_LOCAL))
    .build();

    render_target_memory = unsafe{engine.device().allocate_memory(&aloc_info, None).expect("Could not allocate memory")};
    println!("Allocated memory {:?}", render_target_memory);
    unsafe{engine.device().bind_image_memory(render_target, render_target_memory, 0).expect("Could not bind render target")}
    

    let sizzle = vk::ComponentMapping::builder()
        .a(vk::ComponentSwizzle::A)
        .r(vk::ComponentSwizzle::R)
        .g(vk::ComponentSwizzle::G)
        .b(vk::ComponentSwizzle::B)
        .build();

    let subresource = vk::ImageSubresourceRange::builder()
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_mip_level(0)
        .level_count(1)
        .base_array_layer(0)
        .layer_count(1)
        .build();

    let c_info = vk::ImageViewCreateInfo::builder()
        .image(render_target)
        .view_type(vk::ImageViewType::TYPE_2D)
        .format(engine.swapchain_info().image_format)
        .components(sizzle)
        .subresource_range(subresource)
        .build();
    render_target_view = unsafe{engine.device().create_image_view(&c_info, None).expect("Could not create image view")};
    println!("Created image {:?} and view {:?}", render_target, render_target_view);

}
    


    let v_data = [
        Vertex{pos: [ 0.0, 1.0, 0.0]}, //top
        Vertex{pos: [ -1.0, -1.0,0.5]},  //left
        Vertex{pos: [1.0,-1.0,0.5]}, //right
        Vertex{pos: [0.0, -1.0, -0.5]}, //front  
    ];
    let i_data = [
        3, 2, 0, //fr
        1, 0, 2, //back
        1, 3, 0, //fl
        1,2,3 ]; //bottom

    let mut positions = vec![cgmath::vec4(0.0, 0.0, 2.0, 1.0)];

    for x in -10..10{
        for y in -10..10{
            for z in 10..500{
                //positions.push(cgmath::vec4(x as f32*3.0, y as f32*3.0, z as f32*2.0, 1.0));
                //println!("{:?}", positions.last().unwrap());
            }
        }
    }

    let objects = [core::ray_tracing::ObjectOutline{ 
        vertex_data: v_data.to_vec(), 
        vertex_format: vk::Format::R32G32B32_SFLOAT, 
        index_data: i_data.to_vec(), 
        inital_pos_data: positions,
        sbt_hit_group_offset: 0, }];
    let store = core::ray_tracing::ObjectStore::new(&engine, &objects);

    let tlas = core::ray_tracing::Tlas::new_immediate::<core::Engine,Vertex>(&engine, store.0.get_instance_count(), store.0.get_instance_address());
    
    let d_store = core::memory::DescriptorDataStore::new(&engine);
    let mut tlas_outline = [core::memory::DescriptorSetOutline::new(vk::DescriptorSetLayoutCreateFlags::empty(), 0 as *const c_void, 0 as *const c_void)];
    tlas_outline[0].add_binding(tlas.get_binding(vk::ShaderStageFlags::RAYGEN_KHR));
    tlas_outline[0].add_binding((
        vk::DescriptorType::STORAGE_IMAGE, 
        1, 
        vk::ShaderStageFlags::RAYGEN_KHR, 
        core::memory::DescriptorWriteType::Image([vk::DescriptorImageInfo::builder()
        .image_layout(vk::ImageLayout::GENERAL)
        .image_view(render_target_view)
        .build()])));
    let d_stack = d_store.get_descriptor_stack(&tlas_outline, vk::DescriptorPoolCreateFlags::empty(), 0 as *const c_void, 0 as *const c_void);


    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_target_spirv(shaderc::SpirvVersion::V1_6);
    let ray_gen = core::Shader::new(&engine, String::from(r#"
    #version 460
    #extension GL_EXT_ray_tracing : require
    #extension GL_KHR_vulkan_glsl : enable

    layout(binding = 0, set = 0) uniform accelerationStructureEXT topLevelAS;
    layout(binding = 1, set = 0, rgba32f) uniform image2D image;

    struct hitPayload
    {
        bool hit;
        vec3 hitValue;
    };

    layout(location = 0) rayPayloadEXT hitPayload prd;

    void main() 
        {
            const vec2 pixelCenter = vec2(gl_LaunchIDEXT.xy) + vec2(0.5);
            const vec2 inUV = pixelCenter/vec2(gl_LaunchSizeEXT.xy);
            vec2 d = inUV * 2.0 - 1.0;
            vec4 origin    = vec4(0, 0, -1, 1);
            vec4 target    = vec4(d.x, -d.y, 0, 1);
            vec4 direction = vec4(normalize(target.xyz - origin.xyz), 0);
            uint  rayFlags = gl_RayFlagsOpaqueEXT;
            float tMin     = 0.001;
            float tMax     = 100000.0;
            traceRayEXT(topLevelAS, // acceleration structure
                rayFlags,       // rayFlags
                0xFF,           // cullMask
                0,              // sbtRecordOffset
                0,              // sbtRecordStride
                0,              // missIndex
                origin.xyz,     // ray origin
                tMin,           // ray min range
                direction.xyz,  // ray direction
                tMax,           // ray max range
                0               // payload (location = 0)
        );
            if (d.x > 0 && prd.hit){
                prd.hitValue = prd.hitValue * 0.5;
            }
            imageStore(image, ivec2(gl_LaunchIDEXT.xy), vec4(prd.hitValue,1.0));
        }
    
    "#), shaderc::ShaderKind::RayGeneration, "main", Some(&options));
    let closest_hit = core::Shader::new(&engine, String::from(r#"
    #version 460
    #extension GL_EXT_ray_tracing : require
    #extension GL_EXT_nonuniform_qualifier : enable
    
    struct hitPayload
    {
        bool hit;
        vec3 hitValue;
    };

    layout(location = 0) rayPayloadInEXT hitPayload hitdata;
    hitAttributeEXT vec3 attribs;
    
    void main()
    {
        hitdata.hit = true;
        hitdata.hitValue = vec3(0.2, 0.5, 0.5);
    }"#), shaderc::ShaderKind::ClosestHit, "main", Some(&options));
    let miss = core::Shader::new(&engine, String::from(r#"
    #version 460
    #extension GL_EXT_ray_tracing : require
    
    struct hitPayload
    {
        bool hit;
        vec3 hitValue;
    };

    layout(location = 0) rayPayloadInEXT hitPayload hitdata;
    
    void main()
    {
        hitdata.hit = false;
        hitdata.hitValue = vec3(0.0, 0.1, 0.3);
    }"#), shaderc::ShaderKind::Miss, "main", Some(&options));

    let main = CString::new("main").unwrap();
    let misses = [miss.get_stage(vk::ShaderStageFlags::MISS_KHR, main.as_c_str())];
    let group_1: [(Option<vk::PipelineShaderStageCreateInfo>, Option<vk::PipelineShaderStageCreateInfo>, Option<vk::PipelineShaderStageCreateInfo>);1] = 
    [(Some(closest_hit.get_stage(vk::ShaderStageFlags::CLOSEST_HIT_KHR, main.as_c_str())), None, None)];

    let sbt_outline = core::ray_tracing::SbtOutline::new(ray_gen.get_stage(vk::ShaderStageFlags::RAYGEN_KHR, &main), &misses, &group_1);

    let ray_pipeline = core::ray_tracing::RayTracingPipeline::new_immediate(&engine, sbt_outline, &[d_stack.get_set_layout(0)], &[]);

    let pool = core::CommandPool::new(&engine, vk::CommandPoolCreateInfo::builder().build());
    let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];
    let aquire_semaphore = core::sync::Semaphore::new(&engine);
    let render_complete = core::sync::Semaphore::new(&engine);
    let loop_complete = core::sync::Fence::new(&engine, true);



    let aquire_semaphores = [aquire_semaphore.semaphore];
    let render_complete_semaphores = [render_complete.semaphore];



    let mut instant = Box::new(Instant::now());
    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            winit::event::Event::NewEvents(_) => {},
            winit::event::Event::WindowEvent {event, .. } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                        loop_complete.wait();
                        unsafe{engine.device().device_wait_idle().expect("Device could not wait");}
                        drop(&store);
                        drop(&tlas);
                        drop(&d_stack);
                        drop(&ray_gen);
                        drop(&closest_hit);
                        drop(&miss);
                        drop(&ray_pipeline);
                        println!("Destroying image {:?} and view {:?}", render_target, render_target_view);
                        unsafe{engine.device().destroy_image_view(render_target_view, None);
                        engine.device().destroy_image(render_target, None);}
                        println!("Destroying render target memory {:?}", render_target_memory);
                        unsafe{engine.device().free_memory(render_target_memory, None)};

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
            winit::event::Event::MainEventsCleared => {
                loop_complete.wait_reset();
                println!("Time elapsed last frame {} us", instant.elapsed().whole_microseconds());
                *instant = Instant::now();
                let index = unsafe{[engine.swapchain_loader().acquire_next_image(engine.swapchain(), u64::MAX, aquire_semaphore.semaphore, vk::Fence::null()).expect("Could not get next image index").0]};
                let swapchain = [engine.swapchain()];
                let device = engine.device();

                let render_target_transition = vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::NONE)
                .dst_access_mask(vk::AccessFlags::MEMORY_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::GENERAL)
                .image(render_target)
                .subresource_range(subresource)
                .build();

                let render_target_transfer = [vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ)
                .old_layout(vk::ImageLayout::GENERAL)
                .new_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL)
                .image(render_target)
                .subresource_range(subresource)
                .build()];


                let swap_to_transfer = vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::NONE)
                .dst_access_mask(vk::AccessFlags::MEMORY_WRITE)
                .old_layout(vk::ImageLayout::UNDEFINED)
                .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .image(engine.swapchain_images()[*index.get(0).unwrap() as usize])
                .subresource_range(subresource)
                .build();

                let swap_to_present = [vk::ImageMemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::MEMORY_WRITE)
                .dst_access_mask(vk::AccessFlags::MEMORY_READ)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                .image(engine.swapchain_images()[*index.get(0).unwrap() as usize])
                .subresource_range(subresource)
                .build()];


                let sub_layers = vk::ImageSubresourceLayers::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .mip_level(0)
                .base_array_layer(0)
                .layer_count(1)
                .build();
                let image_init = [render_target_transition, swap_to_transfer];
                let image_copy = [vk::ImageCopy::builder()
                .src_subresource(sub_layers)
                .dst_subresource(sub_layers)
                .src_offset(vk::Offset3D::builder().build())
                .dst_offset(vk::Offset3D::builder().build())
                .extent(swapchain_extent_3d)
                .build()];

                let d_sets = [d_stack.get_set(0)];

                pool.reset();
                unsafe{
                    device.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).expect("Could not begin command buffer");

                    device.cmd_pipeline_barrier(
                        cmd, 
                        vk::PipelineStageFlags::TRANSFER, 
                        vk::PipelineStageFlags::TRANSFER, 
                        vk::DependencyFlags::empty(), 
                        &[], 
                        &[], 
                        &image_init);
                    
                        device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::RAY_TRACING_KHR, ray_pipeline.pipeline);
                        device.cmd_bind_descriptor_sets(cmd, vk::PipelineBindPoint::RAY_TRACING_KHR, ray_pipeline.layout, 0, &d_sets, &[]);
                        ash::extensions::khr::RayTracingPipeline::new(&engine.instance(), &device).cmd_trace_rays(
                            cmd, 
                            &ray_pipeline.shader_addresses.0, 
                            &ray_pipeline.shader_addresses.1, 
                            &ray_pipeline.shader_addresses.2, 
                            &vk::StridedDeviceAddressRegionKHR::builder().build(), 
                            engine.swapchain_info().image_extent.width, engine.swapchain_info().image_extent.height, 1);

                        
                        device.cmd_pipeline_barrier(
                            cmd, 
                            vk::PipelineStageFlags::TRANSFER, 
                            vk::PipelineStageFlags::TRANSFER, 
                            vk::DependencyFlags::empty(), 
                            &[], 
                            &[], 
                            &render_target_transfer);

                        
                        device.cmd_copy_image(
                            cmd, 
                            render_target, 
                            vk::ImageLayout::TRANSFER_SRC_OPTIMAL, 
                            engine.swapchain_images()[*index.get(0).unwrap() as usize], 
                            vk::ImageLayout::TRANSFER_DST_OPTIMAL, 
                            &image_copy);

                        device.cmd_pipeline_barrier(
                            cmd, 
                            vk::PipelineStageFlags::TRANSFER, 
                            vk::PipelineStageFlags::TRANSFER, 
                            vk::DependencyFlags::empty(), 
                            &[], 
                            &[], 
                            &swap_to_present);
                            
                        device.end_command_buffer(cmd).expect("Could not end command buffer");
                        let cmd = [cmd];
                        let wait_at = [vk::PipelineStageFlags::TRANSFER];
                        let submit = [vk::SubmitInfo::builder()
                        .command_buffers(&cmd)
                        .wait_semaphores(&aquire_semaphores)
                        .signal_semaphores(&render_complete_semaphores)
                        .wait_dst_stage_mask(&wait_at)
                        .build()];
                        
                        device.queue_submit(engine.queue_data().graphics.0, &submit, loop_complete.get_fence()).expect("Could not submit render loop");
                    }


                unsafe{engine.swapchain_loader().queue_present(engine.queue_data().graphics.0, &vk::PresentInfoKHR::builder()
                    .image_indices(&index)
                    .swapchains(&swapchain)
                    .wait_semaphores(&render_complete_semaphores)
                    .build()).expect("Could not present from swapchain")};

            },
            winit::event::Event::RedrawRequested(_) => {},
            winit::event::Event::RedrawEventsCleared => {},
            winit::event::Event::LoopDestroyed => {
                println!("Shutting down program")
            },
        }
    });

}