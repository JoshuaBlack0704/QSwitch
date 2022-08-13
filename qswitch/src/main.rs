 use std::os::raw::c_void;

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
    let allocator = qforce::core::memory::AllocationDataStore::new(&engine);
    let mut data:Vec<u32> = (0..100).collect();
    let mut cpu_mem = allocator.allocate_typed::<u32>(allocator.get_type(vk::MemoryPropertyFlags::HOST_COHERENT), data.len()*3, 0 as *const c_void);
    let mut gpu_mem = allocator.allocate_typed::<u32>(allocator.get_type(vk::MemoryPropertyFlags::DEVICE_LOCAL), data.len(), 0 as *const c_void);
    let mut b1 = cpu_mem.get_buffer_typed::<u32>(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, data.len() * 2 + 10, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
    let mut b2 = gpu_mem.get_buffer_typed::<u32>(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, data.len(), None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
    let shader = qforce::core::Shader::new(&engine, String::from(r#"
    #version 460
    #extension GL_KHR_vulkan_glsl : enable

    layout(local_size_x = 1) in;

    layout(set = 0, binding = 0) buffer Data {
        uint[] values;
    } data;

    void main(){
        data.values[gl_GlobalInvocationID.x] += 100;
    }"#), shaderc::ShaderKind::Compute, "main", None);

    let descriptor_store = qforce::core::memory::DescriptorDataStore::new(&engine);
    let start_region = b1.get_region_typed::<u32>(data.len(), None);
    cpu_mem.copy_from_ram_typed(data.as_ptr(), data.len(), &start_region);
    let gpu_region = b2.get_region_typed::<u32>(data.len(), None);
    let end_region = b1.get_region_typed::<u32>(data.len(), None);
    let mut outline = qforce::core::memory::DescriptorSetOutline::new(vk::DescriptorSetLayoutCreateFlags::empty(), 0 as *const c_void, 0 as *const c_void);
    outline.add_binding(gpu_region.get_binding(vk::ShaderStageFlags::COMPUTE));
    let descriptor_stack = descriptor_store.get_descriptor_stack(&[outline], vk::DescriptorPoolCreateFlags::empty(), 0 as *const c_void, 0 as *const c_void);

    let compute = qforce::core::ComputePipeline::new(&engine, &[], &[descriptor_stack.get_set_layout(0)], shader.get_stage(vk::ShaderStageFlags::COMPUTE, &std::ffi::CString::new("main").unwrap()));

    let pool = qforce::core::CommandPool::new(&engine, vk::CommandPoolCreateInfo::builder().build());
    let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];

    unsafe{
        let cmds = vec![cmd];
        engine.device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().build()).unwrap();
        start_region.copy_to_region(cmd, &gpu_region);
        let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::NONE).dst_access_mask(vk::AccessFlags::MEMORY_READ).build();
        engine.device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);
        engine.device().cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, compute.get_pipeline());
        engine.device().cmd_bind_descriptor_sets(cmd, vk::PipelineBindPoint::COMPUTE, compute.get_layout(), 0, &vec![descriptor_stack.get_set(0)], &[]);
        engine.device().cmd_dispatch(cmd, data.len() as u32, 1, 1);
        engine.device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);
        gpu_region.copy_to_region(cmd, &end_region);
        engine.device().end_command_buffer(cmd).unwrap();
        let submit = vk::SubmitInfo::builder().command_buffers(&cmds).build();
        engine.device().queue_submit(engine.queue_data().graphics.0, &[submit], vk::Fence::null()).unwrap();
        engine.device().queue_wait_idle(engine.queue_data().graphics.0).unwrap();
    }
    



    data = vec![100;data.len()];
    cpu_mem.copy_to_ram_typed(&end_region, data.len(), data.as_mut_ptr());
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
                        drop(&cpu_mem);
                        drop(&gpu_mem);
                        drop(&b1);
                        drop(&b2);
                        drop(&shader);
                        drop(&compute);
                        drop(&descriptor_stack);
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