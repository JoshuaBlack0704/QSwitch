 use ash;
 use ash::vk;
 extern crate pretty_env_logger;
 use qforce::traits::{IEngineData, ICommandPool};
use winit;
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


 fn main(){



    pretty_env_logger::init();
    let (event_loop, _window, mut engine) = qforce::core::Engine::init(get_vulkan_validate());
    let mut data:Vec<u32> = (0..100).collect();
    let mut cpu_mem = qforce::core::Memory::new(&engine, vk::MemoryPropertyFlags::HOST_COHERENT);
    let mut gpu_mem = qforce::core::Memory::new(&engine, vk::MemoryPropertyFlags::DEVICE_LOCAL);
    let mut b1 = cpu_mem.get_buffer(vk::BufferCreateInfo::builder().size((std::mem::size_of::<u32>() * data.len()) as u64).usage(vk::BufferUsageFlags::STORAGE_BUFFER).build());
    let mut b2 = gpu_mem.get_buffer(vk::BufferCreateInfo::builder().size((std::mem::size_of::<u32>() * data.len()) as u64).usage(vk::BufferUsageFlags::STORAGE_BUFFER).build());
    let mut b3 = cpu_mem.get_buffer(vk::BufferCreateInfo::builder().size((std::mem::size_of::<u32>() * data.len()) as u64).usage(vk::BufferUsageFlags::STORAGE_BUFFER).build());
    let pool = qforce::core::CommandPool::new(&engine, vk::CommandPoolCreateInfo::builder().queue_family_index(engine.queue_data().graphics.1).build());
    let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1).build())[0];
    let mut d_sys = qforce::core::DescriptorSystem::new(&engine);
    let s1 = d_sys.create_new_set();
    d_sys.set_active_set(s1);
    b2.add_descriptor_block(0, (std::mem::size_of::<u32>() * data.len()) as u64, vk::ShaderStageFlags::ALL, &mut d_sys);
    let shader = qforce::core::Shader::new(&engine, String::from(r#"
    #version 460
    #extension GL_KHR_vulkan_glsl : enable

    layout(local_size_x = 1) in;

    layout(set = 0, binding = 0) buffer Data {
        uint[] values;
    } data;

    void main(){
        data.values[gl_GlobalInvocationID.x] += 10;
    }"#), shaderc::ShaderKind::Compute, "main", None);

    let push = vec![vk::PushConstantRange::builder().size(std::mem::size_of::<usize>() as u32).stage_flags(vk::ShaderStageFlags::COMPUTE).offset(0).build()];
    let set_layout = vec![d_sys.get_set_layout(s1)];
    let compute = qforce::core::ComputePipeline::new(&engine, &push, &set_layout, shader.get_stage(vk::ShaderStageFlags::COMPUTE, &std::ffi::CString::new("main").unwrap()));
    cpu_mem.copy_from_ram(data.as_ptr() as *const u8, std::mem::size_of::<u32>() * data.len(), b1.get_sector(), 0);
    let store:qforce::core::ObjectStore<qforce::core::Vertex, qforce::core::CommandPool> = qforce::core::ObjectStore::new(&engine, qforce::core::CommandPool::new(&engine, vk::CommandPoolCreateInfo::builder().build()));


    unsafe{
        let cmds = vec![cmd];
        engine.device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().build()).unwrap();
        b2.transfer_from_buffer(cmd, &mut b1, 0, (std::mem::size_of::<u32>() * data.len()) as u64, 0);
        let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::NONE).dst_access_mask(vk::AccessFlags::MEMORY_READ).build();
        engine.device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);
        engine.device().cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, compute.get_pipeline());
        engine.device().cmd_bind_descriptor_sets(cmd, vk::PipelineBindPoint::COMPUTE, compute.get_layout(), 0, &vec![d_sys.get_set(s1)], &[]);
        engine.device().cmd_dispatch(cmd, data.len() as u32, 1, 1);
        engine.device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);
        b3.transfer_from_buffer(cmd, &mut b2, 0, (std::mem::size_of::<u32>() * data.len()) as u64, 0);
        engine.device().end_command_buffer(cmd).unwrap();
        let submit = vk::SubmitInfo::builder().command_buffers(&cmds).build();
        engine.device().queue_submit(engine.queue_data().graphics.0, &[submit], vk::Fence::null()).unwrap();
        engine.device().queue_wait_idle(engine.queue_data().graphics.0).unwrap();
    }
    



    data = vec![100;data.len()];
    cpu_mem.copy_to_ram(data.as_mut_ptr() as *mut u8, std::mem::size_of::<u32>() * data.len(), b3.get_sector(), 0);
    println!("{}", data.last().unwrap());
    
    
    {
        event_loop.run(move |event, _, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Poll;
            match event {
                winit::event::Event::NewEvents(_) => {},
                winit::event::Event::WindowEvent {event, .. } => {
                    match event {
                        winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                        drop(&pool);
                        drop(&d_sys);
                        drop(&b1);
                        drop(&b2);
                        drop(&b3);
                        drop(&cpu_mem);
                        drop(&gpu_mem);
                        drop(&shader);
                        drop(&compute);
                        drop(&store);
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
    

 }