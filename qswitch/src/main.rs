 use ash;
 use ash::vk;
 extern crate pretty_env_logger;
 extern crate log;
 use winit;
 

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
    let (event_loop, window, mut engine) = qforce::core::Engine::init(get_vulkan_validate());
    let mut mem = qforce::core::Memory::new(&engine, vk::MemoryPropertyFlags::HOST_COHERENT);
    mem.get_buffer(vk::BufferCreateInfo::builder().size(10000).usage(vk::BufferUsageFlags::STORAGE_BUFFER).build());
    {

        event_loop.run(move |event, _, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Poll;
            match event {
                winit::event::Event::NewEvents(_) => {},
                winit::event::Event::WindowEvent {event, .. } => {
                    match event {
                        winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                        drop(&mem);
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