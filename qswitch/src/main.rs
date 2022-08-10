 use ash;
 use ash::vk;
 extern crate pretty_env_logger;
 extern crate log;
 use qforce::traits::IEngineData;
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
    let mut cpu_mem = qforce::core::Memory::new(&engine, vk::MemoryPropertyFlags::HOST_COHERENT);
    let mut gpu_mem = qforce::core::Memory::new(&engine, vk::MemoryPropertyFlags::DEVICE_LOCAL);

    let shader = qforce::core::Shader::new(&engine, String::from("#version 460\n void main() {}"), shaderc::ShaderKind::Compute, "main", None);

    let pool = qforce::core::CommandPool::new(&engine, vk::CommandPoolCreateInfo::builder().queue_family_index(engine.queue_data().transfer.1).build());
    let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().level(vk::CommandBufferLevel::PRIMARY).command_buffer_count(1).build())[0];

    let mut data:Vec<u64> = (0..100).collect();
    let mut b1 = cpu_mem.get_buffer(vk::BufferCreateInfo::builder().size((std::mem::size_of::<u64>() * data.len()) as u64).usage(vk::BufferUsageFlags::STORAGE_BUFFER).build());
    let mut b2 = gpu_mem.get_buffer(vk::BufferCreateInfo::builder().size((std::mem::size_of::<u64>() * data.len()) as u64).usage(vk::BufferUsageFlags::STORAGE_BUFFER).build());
    let mut b3 = cpu_mem.get_buffer(vk::BufferCreateInfo::builder().size((std::mem::size_of::<u64>() * data.len()) as u64).usage(vk::BufferUsageFlags::STORAGE_BUFFER).build());

    let mut d_sys = qforce::core::DescriptorSystem::new(&engine);
    let s1 = d_sys.create_new_set();
    d_sys.set_active_set(s1);
    b2.add_descriptor_block(0, (std::mem::size_of::<u64>() * data.len()) as u64, vk::ShaderStageFlags::ALL, &mut d_sys);
    println!("Pulled set {:?}", d_sys.get_set(s1));

    cpu_mem.copy_from_ram(data.as_ptr() as *const u8, std::mem::size_of::<u64>() * data.len(), b1.get_sector(), 0);

    unsafe{
        let cmds = vec![cmd];
        engine.device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().build()).unwrap();
        b2.transfer_from_buffer(cmd, &mut b1, 0, (std::mem::size_of::<u64>() * data.len()) as u64, 0);
        let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::NONE).dst_access_mask(vk::AccessFlags::MEMORY_READ).build();
        engine.device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);
        b3.transfer_from_buffer(cmd, &mut b2, 0, (std::mem::size_of::<u64>() * data.len()) as u64, 0);
        engine.device().end_command_buffer(cmd).unwrap();
        let submit = vk::SubmitInfo::builder().command_buffers(&cmds).build();
        engine.device().queue_submit(engine.queue_data().transfer.0, &[submit], vk::Fence::null()).unwrap();
        engine.device().queue_wait_idle(engine.queue_data().transfer.0).unwrap();
    }
    
    data = vec![100;100];
    cpu_mem.copy_to_ram(data.as_mut_ptr() as *mut u8, std::mem::size_of::<u64>() * data.len(), b3.get_sector(), 0);

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
                        drop(&shader)
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