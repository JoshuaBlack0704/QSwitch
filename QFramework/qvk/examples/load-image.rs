use ash::vk;
use qvk::{image::{ImageResource, ImageFactory, ImageResourceFactory}, init::{device, instance, swapchain::{self, SwapchainStore}, Swapchain, InstanceFactory, DeviceFactory}, memory::MemoryFactory, queue::QueueFactory};
use raw_window_handle::HasRawDisplayHandle;
use winit::{event::{Event, WindowEvent}, event_loop::EventLoop, window::WindowBuilder};
use qvk::image::ImageSource;
use qvk::init::DeviceSource;

fn main(){
    
    pretty_env_logger::init();
    
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    
    let mut settings = instance::Settings::default();
    settings.use_window_extensions(window.raw_display_handle());
    let instance = settings.create_instance();
    
    let mut settings = device::Settings::new_simple(instance.clone());
    settings.add_window(&window);
    settings.add_extension(ash::extensions::khr::BufferDeviceAddress::name().as_ptr());
    settings.add_extension(ash::extensions::khr::Swapchain::name().as_ptr());
    let device = settings.create_device().expect("Could not create device");

    let settings = swapchain::SettingsStore::default();
    let swapchain = Swapchain::new(&device, &settings, None).expect("Could not create swapchain");
   
    let dev_mem = device.create_memory(1024 * 1024 * 100, device.device_memory_index(), None).unwrap();

    let image = dev_mem.create_image(&device, vk::Format::B8G8R8A8_SRGB, vk::Extent3D::builder().width(1920).height(1080).depth(1).build(), 1, 1, vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, None).unwrap();
    image.internal_transistion(vk::ImageLayout::TRANSFER_DST_OPTIMAL, None);
    let resource = image.create_resource(vk::Offset3D::default(), image.extent(), 0, vk::ImageAspectFlags::COLOR).unwrap();
    let file = String::from("examples/resources/drone.jpg");
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
                let queue = device.create_queue(vk::QueueFlags::GRAPHICS).unwrap();
                swapchain.present_image(&resource, &queue);
            }
            _ => {}
        }
    })
    
}