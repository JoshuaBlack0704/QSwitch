use qforce::engine;
use quniverse::Universe;
use std::net::{SocketAddr, ToSocketAddrs};
use winit;
fn main() {
    let (evtloop, engine) = engine::new_windowed();
    engine.hello_window();
    let mut swapchain = engine.get_swapchain(None);

    evtloop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            winit::event::Event::WindowEvent { window_id, event } => match event {
                winit::event::WindowEvent::Resized(_) => {
                    swapchain = engine.get_swapchain(Some(&swapchain));
                }
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                },
                _ => {}
            },
            winit::event::Event::MainEventsCleared => {
                let index = swapchain.get_next_image(u64::MAX, )
                swapchain.present(engine.)
            },
            _ => {}
        }
    });
}
