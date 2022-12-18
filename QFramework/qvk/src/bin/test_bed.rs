use std::{mem::size_of, sync::Arc};

use ash::vk;
use qvk::{instance, Instance, device::{self, DeviceProvider}, Device, swapchain::{self, SwapchainProvider}, Swapchain, commandpool, CommandPool, commandbuffer, CommandBufferSet, memory::{self, buffer::{bufferpartition::BufferPartitionProvider, self}}, sync, image::{self, image::ImageProvider, imageresource::ImageSubresourceProvider, ImageResource}};
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
   
    let mut settings = memory::memory::SettingsProvider::new(1024 * 1024 * 100, device.device_memory_index());
    settings.use_alloc_flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS);
    let dev_mem = memory::Memory::new(&settings, &device).expect("Could not allocate memory");

    let image_settings = image::image::SettingsProvider::new_simple(vk::Format::B8G8R8A8_SRGB, vk::Extent3D::builder().width(1920).height(1080).depth(1).build(), vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, Some(vk::ImageLayout::TRANSFER_DST_OPTIMAL));    
    let image = image::Image::new(&device, &dev_mem, &image_settings).unwrap();
    let resource = image::ImageResource::new(&image, vk::ImageAspectFlags::COLOR, 0, 0, 1, vk::Offset3D::default(), image.extent()).unwrap();
    let file = String::from("ship.jpg");
    ImageResource::load_image(&resource, &file);

    event_loop.run(move |event, _, flow|{
        flow.set_poll();
        match event {
            Event::WindowEvent { window_id: _, event } => {
                if let WindowEvent::CloseRequested = event{
                    flow.set_exit();
                }
                if let WindowEvent::Resized(_) = event{
                    swapchain.resize();
                    println!("{:?}", swapchain.extent());
                    // let image_settings = &mut image_settings;
                    // *image_settings = image::image::SettingsProvider::new_simple(vk::Format::B8G8R8A8_SRGB, swapchain.extent(), vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, Some(vk::ImageLayout::TRANSFER_DST_OPTIMAL));    
                    // let image = &mut image;
                    // *image = image::Image::new(&device, &dev_mem, image_settings).unwrap();
                    // let resource = &mut resource;
                    // *resource = image::ImageResource::new(&image, vk::ImageAspectFlags::COLOR, 0, 0, 1, vk::Offset3D::default(), image.extent()).unwrap();
                    // ImageResource::load_image(resource, &file);
                    
                }
            },
            Event::MainEventsCleared => {
                
                swapchain.present_image(&resource);
            }
            _ => {}
        }
    })
    
}