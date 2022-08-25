use qforce::{init::{self, IEngine, WindowedEngine}, memory::{AlignmentType, Allocation}, sync, IDisposable};
use ash::vk;

#[cfg(debug_assertions)]
fn get_vulkan_validate(options: &mut Vec<init::EngineInitOptions>){
    println!("Validation Layers Active");
    let validation_features = [
                vk::ValidationFeatureEnableEXT::BEST_PRACTICES,
                vk::ValidationFeatureEnableEXT::GPU_ASSISTED,
                //vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION,
            ];
    options.push(init::EngineInitOptions::UseValidation(Some(validation_features.to_vec()), None))
}
#[cfg(not(debug_assertions))]
fn get_vulkan_validate(options: &mut Vec<init::EngineInitOptions>){
    println!("Validation Layers Inactive");
}

#[allow(unused)]
fn main(){

    let (event_loop, engine);
        {
                
            match pretty_env_logger::try_init(){
                Ok(_) => {},
                Err(_) => {},
            };
            let mut engine_options = vec![
                init::EngineInitOptions::UseDebugUtils,
                init::EngineInitOptions::DeviceExtensions(vec![])];
            get_vulkan_validate(&mut engine_options);
            (event_loop, engine) = init::WindowedEngine::init(&mut engine_options);
        }
        
        
    let allocator = qforce::memory::Allocator::new(&engine);
    let mut swapchain = init::SwapchainStore::new(&engine, &[init::CreateSwapchainOptions::ImageUsages(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT)]);
    
    let pool = unsafe{engine.get_device().create_command_pool(&vk::CommandPoolCreateInfo::builder().queue_family_index(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER).unwrap().1).build(), None).expect("Could not create command pool")};
    let cmd = unsafe{engine.get_device().allocate_command_buffers(&vk::CommandBufferAllocateInfo::builder().command_pool(pool).command_buffer_count(1).build()).expect("Could not allocate command buffers")}[0];
    let mut width:u32 = swapchain.get_extent().width;
    let mut height:u32 = swapchain.get_extent().height;
    let mut extent = vk::Extent3D::builder().width(width).height(height).depth(1).build();

    let mut data:Vec<u32> = vec![u32::from_be_bytes([255,255,0,0]);(width*height) as usize];

    let mut cpu_mem = Allocation::new::<u32, WindowedEngine>(&engine, vk::MemoryPropertyFlags::HOST_COHERENT, data.len()*20, &mut []);
    let mut cpu_buffer = cpu_mem.create_buffer::<u32>(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, data.len(), &[]).unwrap();
    let mut staging = cpu_buffer.get_region::<u32>(data.len(), AlignmentType::Free, &[]).unwrap();

    let mut gpu_mem = Allocation::new::<u32, WindowedEngine>(&engine, vk::MemoryPropertyFlags::DEVICE_LOCAL, data.len()*20, &mut []);
    let mut image = gpu_mem.create_image(vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::STORAGE, swapchain.get_format(), extent, &[]).unwrap();
    let mut processing = image.get_resources(
        vk::ImageAspectFlags::COLOR, 
        0, 
        1, 
        0, 
        1, 
        vk::ImageViewType::TYPE_2D, 
        swapchain.get_format(), 
        &[]);




    let mut copy_done = sync::Fence::new(&engine, true);
    let mut aquire_semaphore = sync::Semaphore::new(&engine);
    let mut present_semaphore = sync::Semaphore::new(&engine);


    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            winit::event::Event::WindowEvent {event, .. } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                        
                    },
                    winit::event::WindowEvent::Resized(_) => {
                        copy_done.wait();
                        swapchain = init::SwapchainStore::new(&engine, &[init::CreateSwapchainOptions::OldSwapchain(&swapchain),init::CreateSwapchainOptions::ImageUsages(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT)]);
                        
                        width = swapchain.get_extent().width;
                        height = swapchain.get_extent().height;
                        data = vec![u32::from_le_bytes([255,0,0,0]);(width*height) as usize];
                        extent = vk::Extent3D::builder().width(width).height(height).depth(1).build();
                    
                        cpu_buffer.dispose();
                        image.dispose();
                        processing.dispose();

                        cpu_mem = Allocation::new::<u32, WindowedEngine>(&engine, vk::MemoryPropertyFlags::HOST_COHERENT, data.len(), &mut []);
                        cpu_buffer = cpu_mem.create_buffer::<u32>(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, data.len(), &[]).unwrap();
                        staging = cpu_buffer.get_region::<u32>(data.len(), AlignmentType::Free, &[]).unwrap();
                        gpu_mem = Allocation::new::<u32, WindowedEngine>(&engine, vk::MemoryPropertyFlags::DEVICE_LOCAL, data.len()*20, &mut []);
                        image = gpu_mem.create_image(vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::STORAGE, swapchain.get_format(), extent, &[]).unwrap();
                        processing = image.get_resources(
                            vk::ImageAspectFlags::COLOR, 
                            0, 
                            1, 
                            0, 
                            1, 
                            vk::ImageViewType::TYPE_2D, 
                            swapchain.get_format(), 
                            &[]);


                    }
                    winit::event::WindowEvent::KeyboardInput { device_id, input, is_synthetic } => {},
                    _ => {}
                }
            },
            winit::event::Event::MainEventsCleared => {
                cpu_mem.copy_from_ram_slice(&data, &staging);

                unsafe{
                    copy_done.wait_reset();
                    engine.get_device().reset_command_pool(pool, vk::CommandPoolResetFlags::empty()).expect("Could not reset command pool");
                    let (index, next_image) = swapchain.get_next_image(u64::MAX, Some(aquire_semaphore.semaphore), None);

                    engine.get_device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).unwrap();
                    let (processing_dst, _) = processing.transition(vk::AccessFlags::NONE, vk::AccessFlags::TRANSFER_WRITE, vk::ImageLayout::TRANSFER_DST_OPTIMAL); 
                    let (swap_chain_dst, _) = next_image.transition(vk::AccessFlags::NONE, vk::AccessFlags::TRANSFER_WRITE, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
                    let image_barriers = [processing_dst, swap_chain_dst];
                    engine.get_device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TOP_OF_PIPE,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &image_barriers);
                    
                    staging.copy_to_image(cmd, &processing);
                    
                    //let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::TRANSFER_WRITE).dst_access_mask(vk::AccessFlags::MEMORY_READ).build();
                    let (processing_src, _) = processing.transition(vk::AccessFlags::TRANSFER_WRITE, vk::AccessFlags::TRANSFER_READ, vk::ImageLayout::TRANSFER_SRC_OPTIMAL); 
                    let image_barriers = [processing_src];
                    engine.get_device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &image_barriers);
                    
                    processing.copy_to_image(cmd, &next_image);

                    let (swapchain_present,_) = next_image.transition(vk::AccessFlags::TRANSFER_WRITE, vk::AccessFlags::NONE, vk::ImageLayout::PRESENT_SRC_KHR);
                    let image_barriers = [swapchain_present];
                    engine.get_device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::BOTTOM_OF_PIPE, vk::DependencyFlags::empty(), &[], &[], &image_barriers);
                    
                    engine.get_device().end_command_buffer(cmd).unwrap();
                    let cmds = [cmd];
                    let wait_semaphores = [aquire_semaphore.semaphore];
                    let wait_masks = [vk::PipelineStageFlags::TOP_OF_PIPE];
                    let signal_semaphores = [present_semaphore.semaphore];

                    let submit = [vk::SubmitInfo::builder()
                    .wait_semaphores(&wait_semaphores)
                    .wait_dst_stage_mask(&wait_masks)
                    .signal_semaphores(&signal_semaphores)
                    .command_buffers(&cmds)
                    .build()];
                    engine.get_device().queue_submit(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER).unwrap().0, &submit, copy_done.get_fence()).unwrap();
                
                   

                    swapchain.present(engine.get_queue_store().get_queue(vk::QueueFlags::GRAPHICS).unwrap().0, index, &signal_semaphores);
                    
                
                }
            },
            winit::event::Event::LoopDestroyed => {
                unsafe{engine.get_device().device_wait_idle().expect("Could not wait for device to idle")};
                unsafe{
                    engine.get_device().destroy_command_pool(pool, None);
                }
                processing.dispose();
                cpu_buffer.dispose();
                image.dispose();
                cpu_mem.dispose();
                gpu_mem.dispose();
                copy_done.dispose();
                aquire_semaphore.dispose();
                present_semaphore.dispose();
            }
            _ => {}
        }
    });
    {
        
    }
    println!("Hello World");
    engine.get_device();
}