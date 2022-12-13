use ash::vk;
use qvk::{self, device::{DeviceProvider, self}, commandbuffer::{CommandBufferProvider, self}, CommandBufferSet, commandpool::{self, CommandPoolProvider}, CommandPool, Device, instance, Instance};
use raw_window_handle::HasRawDisplayHandle;
use winit::{event_loop::EventLoop, window::WindowBuilder, event::{Event, WindowEvent}};

fn main(){
    
    pretty_env_logger::init();
    
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    
    let mut settings = instance::SettingsProvider::default();
    settings.use_window_extensions(window.raw_display_handle());
    let instance = Instance::new(&settings);
    
    let mut settings = device::SettingsProvider::default();
    settings.add_window(&window);
    let device = Device::new(&settings, &instance).expect("Could not create device");
    
    let settings = commandpool::SettingsProvider::new(device.grahics_queue().unwrap().1);
    let cmdpool = CommandPool::new(&settings, &device).unwrap();
    
    let settings = commandbuffer::SettingsProvider::default();
    let mut cmds = CommandBufferSet::new(&settings, &device, &cmdpool);
    
    let mut settings = qvk::memory::memory::SettingsProvider::new(1024, device.device_memory_index());
    settings.use_alloc_flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
    let mem = qvk::memory::Memory::new(&settings, &device).expect("Could not allocate memory");
    
    let settings = qvk::memory::buffer::SettingsProvider::new(10000, vk::BufferUsageFlags::STORAGE_BUFFER);
    let buf = qvk::memory::Buffer::new(&settings, &device, &mem).expect("Could not bind buffer");
    
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