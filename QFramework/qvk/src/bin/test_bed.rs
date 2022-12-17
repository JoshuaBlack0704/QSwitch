use std::{mem::size_of, sync::Arc};

use ash::vk;
use qvk::{instance, Instance, device::{self, DeviceProvider}, Device, swapchain::{self, SwapchainProvider}, Swapchain, commandpool, CommandPool, commandbuffer, CommandBufferSet, memory::{self, buffer::bufferpartition::BufferPartitionProvider}, sync};
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
    settings.add_extension(ash::extensions::khr::BufferDeviceAddress::name().as_ptr());
    settings.add_extension(ash::extensions::khr::Swapchain::name().as_ptr());
    let device = Device::new(&settings, &instance).expect("Could not create device");

    let settings = swapchain::SettingsProvider::default();
    let swapchain = Swapchain::new(&device, &settings, None).expect("Could not create swapchain");
    
    let settings = commandpool::SettingsProvider::new(device.grahics_queue().unwrap().1);
    let cmdpool = CommandPool::new(&settings, &device).unwrap();
    
    let settings = commandbuffer::SettingsProvider::default();
    let _cmds = CommandBufferSet::new(&settings, &device, &cmdpool);
    
    let mut settings = memory::memory::SettingsProvider::new(1024 * 1024 * 100, device.host_memory_index());
    settings.use_alloc_flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
    let mem = memory::Memory::new(&settings, &device).expect("Could not allocate memory");
    
    let settings = memory::buffer::buffer::SettingsProvider::new(16000*2, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST);
    let buf = memory::buffer::Buffer::new(&settings, &device, &mem).expect("Could not bind buffer");
    
    let data = [5u64;200];
    let part1 = memory::buffer::BufferPartition::new(&buf, (data.len() * size_of::<u64>()) as u64, None).expect("Could not get partition");
    let part2 = memory::buffer::BufferPartition::new(&buf, (data.len() * size_of::<u64>()) as u64, None).expect("Could not get partition");
    let mut dst = [20u64;200];
    part1.copy_from_ram(&data).unwrap();
    part1.copy_to_partition_internal(&part2).expect("Could not copy");
    part2.copy_to_ram(dst.as_mut_slice()).unwrap();
    println!("{:?}", dst);


    let aquire = sync::Semaphore::new(&device);
    
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
                
                let image = swapchain.aquire_next_image(u64::MAX, None::<&Arc<sync::Fence<Device<Instance>>>>, Some(&aquire));
                let wait = [&aquire];
                swapchain.present(image, Some(&wait));
            }
            _ => {}
        }
    })
    
}