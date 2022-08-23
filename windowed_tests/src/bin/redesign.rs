use qforce::init;
use ash::vk;

#[cfg(debug_assertions)]
fn get_vulkan_validate(options: &mut Vec<init::EngineInitOptions>){
    println!("Validation Layers Active");
    let validation_features = [
                vk::ValidationFeatureEnableEXT::BEST_PRACTICES,
                vk::ValidationFeatureEnableEXT::GPU_ASSISTED,
                vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION,
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
            let device_extension_names_raw = [
                ash::extensions::khr::AccelerationStructure::name().as_ptr(),
                ash::extensions::khr::DeferredHostOperations::name().as_ptr(),
                ash::extensions::khr::RayTracingPipeline::name().as_ptr(),
            ];
            let ray_features = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder().ray_tracing_pipeline(true).build();
            let acc_features = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder().acceleration_structure(true).build();
            let features12 = vk::PhysicalDeviceVulkan12Features::builder().timeline_semaphore(true).buffer_device_address(true).build();
            let mut engine_options = vec![
                init::EngineInitOptions::UseDebugUtils,
                init::EngineInitOptions::DeviceExtensions(device_extension_names_raw.to_vec()),
                init::EngineInitOptions::DeviceFeatures12(features12),
                init::EngineInitOptions::DeviceFeaturesRayTracing(ray_features),
                init::EngineInitOptions::DeviceFeaturesAccelerationStructure(acc_features)];
            get_vulkan_validate(&mut engine_options);
            (event_loop, engine) = init::WindowedEngine::init(&mut engine_options);
        }
        
        
        
    let mut swapchain = init::SwapchainStore::new(&engine, &[init::CreateSwapchainOptions::ImageUsages(vk::ImageUsageFlags::TRANSFER_DST)]);
    let mut mem = qforce::memory::Allocation::new::<u8,_>(&engine, vk::MemoryPropertyFlags::HOST_COHERENT, 10000, &mut []);
    
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            winit::event::Event::NewEvents(_) => {},
            winit::event::Event::WindowEvent {event, .. } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                        drop(&mem);
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                    },
                    winit::event::WindowEvent::Resized(_) => {
                        swapchain = init::SwapchainStore::new(&engine, &[init::CreateSwapchainOptions::OldSwapchain(&swapchain),init::CreateSwapchainOptions::ImageUsages(vk::ImageUsageFlags::TRANSFER_DST)]);
                    }
                    winit::event::WindowEvent::KeyboardInput { device_id, input, is_synthetic } => {},
                    _ => {}
                }
            },
            winit::event::Event::DeviceEvent { .. } => {},
            winit::event::Event::UserEvent(_) => {},
            winit::event::Event::Suspended => {},
            winit::event::Event::Resumed => {},
            winit::event::Event::MainEventsCleared => {
                let mut b1 = mem.create_buffer::<u8>(vk::BufferUsageFlags::STORAGE_BUFFER, 1001, &[]);
                let mut r1 = b1.get_region::<u8>(120, qforce::memory::AlignmentType::Free, &[]);
                let mut r2 = b1.get_region::<u8>(100, qforce::memory::AlignmentType::Free, &[]);
                let r4 = r1.get_region::<u8>(50, qforce::memory::AlignmentType::Free, &[]);
                let r5 = r1.get_region::<u8>(50, qforce::memory::AlignmentType::Free, &[]);
            },
            winit::event::Event::RedrawRequested(_) => {},
            winit::event::Event::RedrawEventsCleared => {},
            winit::event::Event::LoopDestroyed => {
                println!("Shutting down program");
            },
        }
    });
}