use qvk;
use raw_window_handle::HasRawDisplayHandle;
use winit::{event_loop::EventLoop, window::WindowBuilder, event::{Event, WindowEvent}};

fn main(){
    
    pretty_env_logger::init();
    
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    
    let mut settings = qvk::instance::SettingsProvider::default();
    settings.use_window_extensions(window.raw_display_handle());
    let instance = qvk::Instance::new(&settings);
    
    let mut settings = qvk::device::SettingsProvider::default();
    settings.add_window(&window);
    let device = qvk::Device::new(&settings, &instance).expect("Could not create device");
    
    
    // event_loop.run(move |event, _, flow|{
    //     flow.set_wait();
    //     let device = &device;
    //     match event {
    //         Event::WindowEvent { window_id: _, event } => {
    //             if let WindowEvent::CloseRequested = event{
    //                 flow.set_exit();
    //             }
    //         },
    //         _ => {}
    //     }
    // })
    
}