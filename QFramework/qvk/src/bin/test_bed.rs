use ash::vk;
use qvk::{self, device::{DeviceProvider, self}, commandbuffer, CommandBufferSet, commandpool, CommandPool, Device, instance, Instance, memory, swapchain::{self, SwapchainProvider}, Swapchain, sync::{self, fence::FenceProvider}};
use raw_window_handle::HasRawDisplayHandle;
use winit::{event_loop::EventLoop, window::WindowBuilder, event::{Event, WindowEvent}};

type SemaphoreType = sync::Semaphore<Device<Instance>>;

fn main(){
    
    pretty_env_logger::init();
    
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    
    let mut settings = instance::SettingsProvider::default();
    settings.use_window_extensions(window.raw_display_handle());
    let instance = Instance::new(&settings);
    
    let mut settings = device::SettingsProvider::default();
    settings.add_window(&window);
    settings.add_extension(ash::extensions::khr::BufferDeviceAddress::name().as_ptr());
    settings.add_extension(ash::extensions::khr::Swapchain::name().as_ptr());
    let device = Device::new(&settings, &instance).expect("Could not create device");

    let settings = swapchain::SettingsProvider::default();
    let swapchain = Swapchain::new(&instance, &device, &settings, None).expect("Could not create swapchain");
    
    let settings = commandpool::SettingsProvider::new(device.grahics_queue().unwrap().1);
    let cmdpool = CommandPool::new(&settings, &device).unwrap();
    
    let settings = commandbuffer::SettingsProvider::default();
    let _cmds = CommandBufferSet::new(&settings, &device, &cmdpool);
    
    let mut settings = memory::memory::SettingsProvider::new(1024 * 1024 * 100, device.device_memory_index());
    settings.use_alloc_flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
    let mem = memory::Memory::new(&settings, &device).expect("Could not allocate memory");
    
    let settings = memory::buffer::SettingsProvider::new(10000, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS);
    let _buf = memory::Buffer::new(&settings, &device, &mem).expect("Could not bind buffer");

    let aquire_fence = sync::Fence::new(&device, true);
    
    event_loop.run(move |event, _, flow|{
        flow.set_poll();
        match event {
            Event::WindowEvent { window_id: _, event } => {
                if let WindowEvent::CloseRequested = event{
                    flow.set_exit();
                }
                if let WindowEvent::Resized(_) = event{
                    swapchain.resize();
                }
            },
            Event::MainEventsCleared => {
                
                aquire_fence.reset();
                let image = swapchain.aquire_next_image::<_,sync::Semaphore<Device<Instance>>>(u64::MAX, Some(&aquire_fence), None);
                aquire_fence.wait(None);
                swapchain.present::<SemaphoreType>(image, None);
            }
            _ => {}
        }
    })
    
}