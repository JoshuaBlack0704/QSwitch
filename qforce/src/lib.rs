extern crate pretty_env_logger;
#[macro_use] extern crate log;

pub mod enums{
    use ash;
    use ash::vk;
    pub enum EngineMessage{}
    pub enum MemoryMessage{

    }
}

pub mod traits{
    use flume;
    use ash;
    use ash::vk;
    use crate::{core, enums};

    pub trait WindowEventCallback{
        fn window_event_callback(&mut self, event: &winit::event::WindowEvent);
    }

    pub trait IWindowEventsChannelGroup<T> {
        fn new_event(&self) -> &(flume::Sender<T>, flume::Receiver<T>);
        fn window_event(&self) -> &(flume::Sender<T>, flume::Receiver<T>);
        fn device_event(&self) -> &(flume::Sender<T>, flume::Receiver<T>);
        fn user_event(&self) -> &(flume::Sender<T>, flume::Receiver<T>);
        fn suspended(&self) -> &(flume::Sender<T>, flume::Receiver<T>);
        fn resumed(&self) -> &(flume::Sender<T>, flume::Receiver<T>);
        fn main_events_cleared(&self) -> &(flume::Sender<T>, flume::Receiver<T>);
        fn redraw_requested(&self) -> &(flume::Sender<T>, flume::Receiver<T>);
        fn redraw_events_cleared(&self) -> &(flume::Sender<T>, flume::Receiver<T>);
        fn loop_destroyed(&self) -> &(flume::Sender<T>, flume::Receiver<T>);
    }
    pub trait IEngineData {
        fn entry(&self) -> ash::Entry;
        fn instance(&self) -> ash::Instance;
        fn physical_device(&self) -> ash::vk::PhysicalDevice;
        fn device(&self) -> ash::Device;
        fn queue_data(&self) -> core::QueueCache;
        fn dubug(&self) -> ash::vk::DebugUtilsMessengerEXT;
        fn debug_loader(&self) -> ash::extensions::ext::DebugUtils;
    }
    pub trait IWindowedEngineData {
        fn surface_loader(&self) -> ash::extensions::khr::Surface;
        fn surface(&self) -> ash::vk::SurfaceKHR;
        fn swapchain_loader(&self) -> ash::extensions::khr::Swapchain;
        fn swapchain(&self) -> ash::vk::SwapchainKHR;
        fn swapchain_info(&self) -> vk::SwapchainCreateInfoKHR;
        fn swapchain_images(&self) -> Vec<vk::Image>;
    }

}

#[allow(dead_code)]
pub mod core{
    use ash;
    use ash::{vk, Entry};
    use std::f32::consts::E;
    use std::{ffi::CStr, os::raw::c_char};
    use std::borrow::{Cow, Borrow};
    use crate::traits;
    use crate::enums;

    unsafe extern "system" fn vulkan_debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
        _user_data: *mut std::os::raw::c_void,
    ) -> vk::Bool32 {
        let callback_data = *p_callback_data;
        let message_id_number: i32 = callback_data.message_id_number as i32;
    
        let message_id_name = if callback_data.p_message_id_name.is_null() {
            Cow::from("")
        } else {
            CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
        };
    
        let message = if callback_data.p_message.is_null() {
            Cow::from("")
        } else {
            CStr::from_ptr(callback_data.p_message).to_string_lossy()
        };
    
        println!(
            "{:?}:\n{:?} [{} ({})] : {}\n",
            message_severity,
            message_type,
            message_id_name,
            &message_id_number.to_string(),
            message,
        );
    
        vk::FALSE
    }
    fn get_physical_device_surface(instance: ash::Instance, surface_loader: ash::extensions::khr::Surface, surface: vk::SurfaceKHR)->Option<vk::PhysicalDevice>{
        unsafe {
            let potential_devices = instance.enumerate_physical_devices().unwrap();
    
            for physical_device in potential_devices.iter(){
                let props = instance.get_physical_device_properties(*physical_device);
                let name = String::from_utf8(props.device_name.iter().map(|&c| c as u8).collect()).unwrap().replace("\0", "");
                let queue_families = instance.get_physical_device_queue_family_properties(*physical_device);
    
                let mut has_graphics_present = false;
                let mut has_transfer = false;
                let mut has_compute = false;
    
                for (index, family) in queue_families.iter().enumerate(){
                    if has_graphics_present && has_transfer && has_compute {
                        debug!("All queue type available, Device {:?} selected", name);
                        return Some(*physical_device)
                    }
                    if !has_graphics_present {
                        if  family.queue_flags.contains(vk::QueueFlags::GRAPHICS) && surface_loader.get_physical_device_surface_support(*physical_device, index as u32, surface).unwrap(){
                            debug!("Queue family {} on device {:?} contains graphics and surface", index, name);
                            has_graphics_present = true;
                        }
                        
                    }
                    if !has_transfer {
                        if  family.queue_flags.contains(vk::QueueFlags::TRANSFER){
                            debug!("Queue family {} on device {:?} contains transfer", index, name);
                            has_transfer = true;
                        }
                    }
                    if  !has_compute{
                        if  family.queue_flags.contains(vk::QueueFlags::COMPUTE){
                            debug!("Queue family {} on device {:?} contains compute", index, name);
                            has_compute = true;
                        }
                    }
                }
    
            }
            debug!("No suitable device found");
            None
        }
    }
    fn get_physical_device_nosurface(instance: ash::Instance) -> Option<vk::PhysicalDevice>{
    
    
        unsafe {
            let potential_devices = instance.enumerate_physical_devices().unwrap();
    
            for physical_device in potential_devices.iter(){
                let props = instance.get_physical_device_properties(*physical_device);
                let name = String::from_utf8(props.device_name.iter().map(|&c| c as u8).collect()).unwrap().replace("\0", "");
                let queue_families = instance.get_physical_device_queue_family_properties(*physical_device);
    
                let mut has_graphics_present = false;
                let mut has_transfer = false;
                let mut has_compute = false;
    
                for (index, family) in queue_families.iter().enumerate(){
                    if has_graphics_present && has_transfer && has_compute {
                        debug!("All queue type available, Device {:?} selected", name);
                        return Some(*physical_device)
                    }
                    if !has_graphics_present {
                        if  family.queue_flags.contains(vk::QueueFlags::GRAPHICS){
                            debug!("Queue family {} on device {:?} contains graphics", index, name);
                            has_graphics_present = true;
                        }
                        
                    }
                    if !has_transfer {
                        if  family.queue_flags.contains(vk::QueueFlags::TRANSFER){
                            debug!("Queue family {} on device {:?} contains transfer", index, name);
                            has_transfer = true;
                        }
                    }
                    if  !has_compute{
                        if  family.queue_flags.contains(vk::QueueFlags::COMPUTE){
                            debug!("Queue family {} on device {:?} contains compute", index, name);
                            has_compute = true;
                        }
                    }
                }
    
            }
            debug!("No suitable device found");
            None
        }
    }
    fn get_queue_info(instance: ash::Instance, physical_device: vk::PhysicalDevice, priorities: &[f32]) -> (Vec<vk::DeviceQueueCreateInfo>, [u32; 3]){ 
        unsafe{
            let queue_families = instance.get_physical_device_queue_family_properties(physical_device);
            let mut graphics_family: Option<vk::DeviceQueueCreateInfo> = None;
            let mut transfer_family: Option<vk::DeviceQueueCreateInfo> = None;
            let mut compute_family: Option<vk::DeviceQueueCreateInfo> = None;
    
    
    
            for (index, family) in queue_families.iter().enumerate(){
                
                if family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    match graphics_family {
                        Some(_) => {},
                        None => {
                            graphics_family = Some(vk::DeviceQueueCreateInfo::builder().queue_family_index(index as u32).queue_priorities(&priorities).build());
                        },
                    }
                }
                if family.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                    match transfer_family {
                        Some(_) => {},
                        None => {
                            transfer_family = Some(vk::DeviceQueueCreateInfo::builder().queue_family_index(index as u32).queue_priorities(&priorities).build());
                        },
                    }
                }
                if family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                    match compute_family {
                        Some(_) => {},
                        None => {
                            compute_family = Some(vk::DeviceQueueCreateInfo::builder().queue_family_index(index as u32).queue_priorities(&priorities).build());
                        },
                    }
                }
    
                if family.queue_flags.contains(vk::QueueFlags::TRANSFER) && !family.queue_flags.contains(vk::QueueFlags::GRAPHICS)  && !family.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                    transfer_family = Some(vk::DeviceQueueCreateInfo::builder().queue_family_index(index as u32).queue_priorities(&priorities).build());
                }
                if family.queue_flags.contains(vk::QueueFlags::COMPUTE) && !family.queue_flags.contains(vk::QueueFlags::GRAPHICS)  && !family.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                    compute_family = Some(vk::DeviceQueueCreateInfo::builder().queue_family_index(index as u32).queue_priorities(&priorities).build());
                }
    
    
            }
    
            let mut infos: Vec<vk::DeviceQueueCreateInfo> = Vec::with_capacity(3);
            infos.push(graphics_family.unwrap());
            let duplicate = infos.iter().filter(|&i| i.queue_family_index == transfer_family.unwrap().queue_family_index).count();
            if duplicate == 0 {
                infos.push(transfer_family.unwrap())
            }
            let duplicate = infos.iter().filter(|&i| i.queue_family_index == compute_family.unwrap().queue_family_index).count();
            if duplicate == 0 {
                infos.push(compute_family.unwrap())
            }
            (infos, [graphics_family.unwrap().queue_family_index, transfer_family.unwrap().queue_family_index, compute_family.unwrap().queue_family_index])
        }

    }

    #[derive(Clone)]
    pub struct WindowEventsChannelGroup<T>{
        new_event: (flume::Sender<T>, flume::Receiver<T>),
        window_event: (flume::Sender<T>, flume::Receiver<T>),
        device_event: (flume::Sender<T>, flume::Receiver<T>),
        user_event: (flume::Sender<T>, flume::Receiver<T>),
        suspended: (flume::Sender<T>, flume::Receiver<T>),
        resumed: (flume::Sender<T>, flume::Receiver<T>),
        main_events_cleared: (flume::Sender<T>, flume::Receiver<T>),
        redraw_requested: (flume::Sender<T>, flume::Receiver<T>),
        redraw_events_cleared: (flume::Sender<T>, flume::Receiver<T>),
        loop_destroyed: (flume::Sender<T>, flume::Receiver<T>),
    }
    impl<T> WindowEventsChannelGroup<T>{
        pub fn new() -> WindowEventsChannelGroup<T>{
            WindowEventsChannelGroup { 
                new_event: flume::unbounded(), 
                window_event: flume::unbounded(), 
                device_event: flume::unbounded(), 
                user_event: flume::unbounded(), 
                suspended: flume::unbounded(), 
                resumed: flume::unbounded(), 
                main_events_cleared: flume::unbounded(), 
                redraw_requested: flume::unbounded(), 
                redraw_events_cleared: flume::unbounded(), 
                loop_destroyed: flume::unbounded() }
        }
    }
    impl<T> traits::IWindowEventsChannelGroup<T> for WindowEventsChannelGroup<T>{
        fn new_event(&self) -> &(flume::Sender<T>, flume::Receiver<T>){
            self.new_event.borrow()
        }
        fn window_event(&self) -> &(flume::Sender<T>, flume::Receiver<T>){
            self.window_event.borrow()
        }
        fn device_event(&self) -> &(flume::Sender<T>, flume::Receiver<T>){
            self.device_event.borrow()
        }
        fn user_event(&self) -> &(flume::Sender<T>, flume::Receiver<T>){
            self.user_event.borrow()
        }
        fn suspended(&self) -> &(flume::Sender<T>, flume::Receiver<T>){
            self.suspended.borrow()
        }
        fn resumed(&self) -> &(flume::Sender<T>, flume::Receiver<T>){
            self.resumed.borrow()
        }
        fn main_events_cleared(&self) -> &(flume::Sender<T>, flume::Receiver<T>){
            self.main_events_cleared.borrow()
        }
        fn redraw_requested(&self) -> &(flume::Sender<T>, flume::Receiver<T>){
            self.redraw_requested.borrow()
        }
        fn redraw_events_cleared(&self) -> &(flume::Sender<T>, flume::Receiver<T>){
            self.redraw_events_cleared.borrow()
        }
        fn loop_destroyed(&self) -> &(flume::Sender<T>, flume::Receiver<T>){
            self.loop_destroyed.borrow()
        }
    }

    #[derive(Clone)]
    pub struct QueueCache{
        pub graphics: (vk::Queue, u32),
        pub transfer: (vk::Queue, u32),
        pub compute: (vk::Queue, u32),
        
    }

    pub struct Engine{
        entry: ash::Entry,
        instance: ash::Instance,
        physical_device: ash::vk::PhysicalDevice,
        device: ash::Device,
        queue_data: QueueCache,
        dubug: ash::vk::DebugUtilsMessengerEXT,
        debug_loader: ash::extensions::ext::DebugUtils,
        surface_loader: ash::extensions::khr::Surface,
        surface: ash::vk::SurfaceKHR,
        swapchain_loader: ash::extensions::khr::Swapchain,
        swapchain:ash::vk::SwapchainKHR,
        swapchain_info: vk::SwapchainCreateInfoKHR,
        swapchain_images: Vec<vk::Image>,
    }
    impl Engine{
        pub fn init(validate: bool) -> (winit::event_loop::EventLoop<()>, winit::window::Window, Engine){
            let engine:Engine;

            let event_loop = winit::event_loop::EventLoop::new();
            let window = winit::window::WindowBuilder::new()
                .with_title("Ray tracer!")
                .build(&event_loop)
                .unwrap();


            unsafe{
                let entry = ash::Entry::linked();
                let app_name = CStr::from_bytes_with_nul_unchecked(b"VulkanTriangle\0");

                let layer_names = [CStr::from_bytes_with_nul_unchecked(
                    b"VK_LAYER_KHRONOS_validation\0",
                )];
                let layers_names_raw: Vec<*const c_char> = layer_names
                    .iter()
                    .map(|raw_name| raw_name.as_ptr())
                    .collect();

                let mut extension_names = ash_window::enumerate_required_extensions(&window)
                    .unwrap()
                    .to_vec();
                extension_names.push(ash::extensions::ext::DebugUtils::name().as_ptr());
                //extension_names.push(ash::extensions::khr::AccelerationStructure::name().as_ptr());
                //extension_names.push(ash::extensions::khr::RayTracingPipeline::name().as_ptr());
                //extension_names.push(ash::extensions::khr::DeferredHostOperations::name().as_ptr());


                #[cfg(any(target_os = "macos", target_os = "ios"))]
                {
                    extension_names.push(KhrPortabilityEnumerationFn::name().as_ptr());
                    // Enabling this extension is a requirement when using `VK_KHR_portability_subset`
                    extension_names.push(KhrGetPhysicalDeviceProperties2Fn::name().as_ptr());
                }

                let appinfo = vk::ApplicationInfo::builder()
                    .application_name(app_name)
                    .application_version(0)
                    .engine_name(app_name)
                    .engine_version(0)
                    .api_version(vk::API_VERSION_1_3);

                let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
                    vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
                } else {
                    vk::InstanceCreateFlags::default()
                };

                let create_info: vk::InstanceCreateInfoBuilder;
                if validate {
                    create_info = vk::InstanceCreateInfo::builder()
                        .application_info(&appinfo)
                        .enabled_layer_names(&layers_names_raw)
                        .enabled_extension_names(&extension_names)
                        .flags(create_flags);
                }
                else {
                    create_info = vk::InstanceCreateInfo::builder()
                        .application_info(&appinfo)
                        .enabled_extension_names(&extension_names)
                        .flags(create_flags);
                }

                let instance: ash::Instance = entry
                    .create_instance(&create_info, None)
                    .expect("Instance creation error");


                let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                    )
                    .pfn_user_callback(Some(vulkan_debug_callback));

                let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &instance);
                let debug_call_back = debug_utils_loader
                    .create_debug_utils_messenger(&debug_info, None)
                    .unwrap();


                let surface = ash_window::create_surface(&entry, &instance, &window, None).unwrap();
                let surface_loader = ash::extensions::khr::Surface::new(&entry, &instance);

                let pdevice = get_physical_device_surface(instance.clone(), surface_loader.clone(), surface.clone()).unwrap();
                
                let device_extension_names_raw = [
                    ash::extensions::khr::Swapchain::name().as_ptr(),
                    #[cfg(any(target_os = "macos", target_os = "ios"))]
                        KhrPortabilitySubsetFn::name().as_ptr(),
                ];
                let mut features13 = vk::PhysicalDeviceVulkan13Features::builder().dynamic_rendering(true).build();
                let mut features12 = vk::PhysicalDeviceVulkan12Features::builder().timeline_semaphore(true).build();
                let mut features11 = vk::PhysicalDeviceVulkan11Features::builder().build();
                let mut features = vk::PhysicalDeviceFeatures2::builder()
                    .push_next(&mut features11)
                    .push_next(&mut features12)
                    .push_next(&mut features13);
                let priorities = [1.0];

                let (queue_infos, queue_families) = get_queue_info(instance.clone(), pdevice, &priorities);

                let device_create_info = vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queue_infos)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut features);

                let device: ash::Device = instance
                    .create_device(pdevice, &device_create_info, None)
                    .unwrap();
                
                let qcache:QueueCache = QueueCache{ graphics: (device.get_device_queue(queue_families[0], 0), queue_families[0]),
                    transfer: (device.get_device_queue(queue_families[1], 0), queue_families[1]),
                    compute: (device.get_device_queue(queue_families[2], 0), queue_families[2] )};
                let swapchain_loader = ash::extensions::khr::Swapchain::new(&instance, &device);
                let (swapchain_info, swapchain, swapchain_images) = Engine::get_swapchain(&pdevice, &surface, &surface_loader, &swapchain_loader, None);
                engine = Engine{ entry, 
                    instance, 
                    physical_device: pdevice, 
                    device, 
                    queue_data: qcache, 
                    dubug: debug_call_back, 
                    debug_loader: debug_utils_loader, 
                    surface_loader, 
                    surface, 
                    swapchain_loader, 
                    swapchain: swapchain, 
                    swapchain_info: swapchain_info, 
                    swapchain_images: swapchain_images,
                    
                 }
            }

            (event_loop, window, engine)
        }
        pub fn refresh_swapchain(&mut self){
            let (swapchain_info, swapchain, swapchain_images) = Engine::get_swapchain(&&self.physical_device, &self.surface, &self.surface_loader, &self.swapchain_loader, Some(self.swapchain));
            self.swapchain = swapchain;
            self.swapchain_info = swapchain_info;
            self.swapchain_images = swapchain_images;
            debug!("Refreshed swapchain to size: {} x {}", swapchain_info.image_extent.width, swapchain_info.image_extent.height);
        }
        pub fn get_swapchain(physical_device: &vk::PhysicalDevice, surface: &vk::SurfaceKHR, surface_loader: &ash::extensions::khr::Surface, swapchain_loader: &ash::extensions::khr::Swapchain, old_swapchain: Option<ash::vk::SwapchainKHR>)-> (ash::vk::SwapchainCreateInfoKHR, ash::vk::SwapchainKHR, Vec<vk::Image>){
            unsafe {
                
                //clearing the swapchain
                match old_swapchain {
                    Some(swapchain) => {swapchain_loader.destroy_swapchain(swapchain, None);},
                    None => {}
                }
    
                let surface_format = surface_loader
                    .get_physical_device_surface_formats(*physical_device, *surface)
                    .unwrap()[0];
    
                let surface_capabilities = surface_loader
                    .get_physical_device_surface_capabilities(*physical_device, *surface)
                    .unwrap();
                let mut desired_image_count = surface_capabilities.min_image_count + 1;
                if surface_capabilities.max_image_count > 0
                    && desired_image_count > surface_capabilities.max_image_count
                {
                    desired_image_count = surface_capabilities.max_image_count;
                }
                let surface_resolution = surface_capabilities.current_extent;
                let pre_transform = if surface_capabilities
                    .supported_transforms
                    .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
                {
                    vk::SurfaceTransformFlagsKHR::IDENTITY
                } else {
                    surface_capabilities.current_transform
                };
                let present_modes = surface_loader
                    .get_physical_device_surface_present_modes(*physical_device, *surface)
                    .unwrap();
                let present_mode = present_modes
                    .iter()
                    .cloned()
                    .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                    .unwrap_or(vk::PresentModeKHR::FIFO);
                let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
                    .surface(*surface)
                    .min_image_count(desired_image_count)
                    .image_color_space(surface_format.color_space)
                    .image_format(surface_format.format)
                    .image_extent(surface_resolution)
                    .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST)
                    .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                    .pre_transform(pre_transform)
                    .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                    .present_mode(present_mode)
                    .clipped(true)
                    .image_array_layers(1);
    
    
    
                let swapchain = swapchain_loader
                    .create_swapchain(&swapchain_create_info, None);
                let images = swapchain_loader.get_swapchain_images(swapchain.unwrap()).unwrap();
                (swapchain_create_info.build(), swapchain.unwrap(), images)
        }
    }
    }
    impl Clone for Engine{

        fn clone(&self) -> Self {
        Self { entry: self.entry.clone(), 
            instance: self.instance.clone(), 
            physical_device: self.physical_device.clone(), 
            device: self.device.clone(), 
            queue_data: self.queue_data.clone(), 
            dubug: self.dubug.clone(), 
            debug_loader: self.debug_loader.clone(), 
            surface_loader: self.surface_loader.clone(), 
            surface: self.surface.clone(), 
            swapchain_loader: self.swapchain_loader.clone(), 
            swapchain: self.swapchain.clone(), 
            swapchain_info: self.swapchain_info.clone(), 
            swapchain_images: self.swapchain_images.clone(),
        }
    }
    }
    impl Drop for Engine{
        fn drop(&mut self) {
        unsafe{
                self.swapchain_loader.destroy_swapchain(self.swapchain, None);
                self.device.destroy_device(None);
                self.surface_loader.destroy_surface(self.surface, None);
                self.debug_loader.destroy_debug_utils_messenger(self.dubug, None);
                self.instance.destroy_instance(None);
            }
            debug!("Engine Destroyed");
    }
    }
    impl traits::IEngineData for Engine {
        fn entry(&self) -> ash::Entry {
            self.entry.clone()
        }

        fn instance(&self) -> ash::Instance {
            self.instance.clone()
        }

        fn physical_device(&self) -> ash::vk::PhysicalDevice {
            self.physical_device.clone()
        }

        fn device(&self) -> ash::Device {
            self.device.clone()
        }

        fn queue_data(&self) -> self::QueueCache {
            self.queue_data.clone()
        }

        fn dubug(&self) -> ash::vk::DebugUtilsMessengerEXT {
            self.dubug.clone()
        }

        fn debug_loader(&self) -> ash::extensions::ext::DebugUtils {
            self.debug_loader.clone()
        }

    }
    impl traits::IWindowedEngineData for Engine{
        fn surface_loader(&self) -> ash::extensions::khr::Surface {
            self.surface_loader.clone()
        }

        fn surface(&self) -> ash::vk::SurfaceKHR {
            self.surface().clone()
        }

        fn swapchain_loader(&self) -> ash::extensions::khr::Swapchain {
            self.swapchain_loader.clone()
        }

        fn swapchain(&self) -> ash::vk::SwapchainKHR {
            self.swapchain.clone()
        }

        fn swapchain_info(&self) -> vk::SwapchainCreateInfoKHR {
            self.swapchain_info.clone()
        }

        fn swapchain_images(&self) -> Vec<vk::Image> {
            self.swapchain_images.clone()
        }
    }
    
    pub struct WindowlessEngine{
        entry: ash::Entry,
        instance: ash::Instance,
        physical_device: ash::vk::PhysicalDevice,
        device: ash::Device,
        queue_data: QueueCache,
        dubug: ash::vk::DebugUtilsMessengerEXT,
        debug_loader: ash::extensions::ext::DebugUtils,
    }
    impl WindowlessEngine{
        pub fn init(validate: bool) -> WindowlessEngine{
            let engine: WindowlessEngine;
            unsafe {
    
                let entry = Entry::linked();
                let app_name = CStr::from_bytes_with_nul_unchecked(b"VulkanTriangle\0");
    
                let layer_names = [CStr::from_bytes_with_nul_unchecked(
                    b"VK_LAYER_KHRONOS_validation\0",
                )];
                let layers_names_raw: Vec<*const c_char> = layer_names
                    .iter()
                    .map(|raw_name| raw_name.as_ptr())
                    .collect();
    
                let extension_names = vec![ash::extensions::ext::DebugUtils::name().as_ptr()];
    
    
                #[cfg(any(target_os = "macos", target_os = "ios"))]
                {
                    extension_names.push(KhrPortabilityEnumerationFn::name().as_ptr());
                    // Enabling this extension is a requirement when using `VK_KHR_portability_subset`
                    extension_names.push(KhrGetPhysicalDeviceProperties2Fn::name().as_ptr());
                }
    
                let appinfo = vk::ApplicationInfo::builder()
                    .application_name(app_name)
                    .application_version(0)
                    .engine_name(app_name)
                    .engine_version(0)
                    .api_version(vk::API_VERSION_1_3);
    
                let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
                    vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
                } else {
                    vk::InstanceCreateFlags::default()
                };
    
                let create_info: vk::InstanceCreateInfoBuilder;
                if validate {
                    create_info = vk::InstanceCreateInfo::builder()
                        .application_info(&appinfo)
                        .enabled_layer_names(&layers_names_raw)
                        .enabled_extension_names(&extension_names)
                        .flags(create_flags);
                }
                else {
                    create_info = vk::InstanceCreateInfo::builder()
                        .application_info(&appinfo)
                        .enabled_extension_names(&extension_names)
                        .flags(create_flags);
                }
    
                let instance: ash::Instance = entry
                    .create_instance(&create_info, None)
                    .expect("Instance creation error");
    
    
                let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO,
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                    )
                    .pfn_user_callback(Some(vulkan_debug_callback));
    
                let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &instance);
                let debug_call_back = debug_utils_loader
                    .create_debug_utils_messenger(&debug_info, None)
                    .unwrap();
    
    
                let pdevice = get_physical_device_nosurface(instance.clone()).unwrap();
                let device_extension_names_raw = [
                    #[cfg(any(target_os = "macos", target_os = "ios"))]
                        KhrPortabilitySubsetFn::name().as_ptr(),
                ];
                let mut features13 = vk::PhysicalDeviceVulkan13Features::builder().dynamic_rendering(true).build();
                let mut features12 = vk::PhysicalDeviceVulkan12Features::builder().timeline_semaphore(true).build();
                let mut features11 = vk::PhysicalDeviceVulkan11Features::builder().build();
                let mut features = vk::PhysicalDeviceFeatures2::builder()
                    .push_next(&mut features11)
                    .push_next(&mut features12)
                    .push_next(&mut features13);
                let priorities = [1.0];
    
                let (queue_infos, queue_families) = get_queue_info(instance.clone(), pdevice, &priorities);
    
                let device_create_info = vk::DeviceCreateInfo::builder()
                    .queue_create_infos(&queue_infos)
                    .enabled_extension_names(&device_extension_names_raw)
                    .push_next(&mut features);
    
                let device: ash::Device = instance
                    .create_device(pdevice, &device_create_info, None)
                    .unwrap();
    
                let qcache:QueueCache = QueueCache{ graphics: (device.get_device_queue(queue_families[0], 0), queue_families[0]),
                    transfer: (device.get_device_queue(queue_families[1], 0), queue_families[1]),
                     compute: (device.get_device_queue(queue_families[2], 0), queue_families[2] )};
    
    
                engine = WindowlessEngine{ entry, 
                    instance, 
                    physical_device: pdevice, 
                    device, 
                    queue_data: qcache, 
                    dubug: debug_call_back, 
                    debug_loader: debug_utils_loader,
                 };
    
            }
    
            engine
        }
    }
    impl Clone for WindowlessEngine{
        fn clone(&self) -> Self {

        Self { entry: self.entry.clone(), 
            instance: self.instance.clone(), 
            physical_device: self.physical_device.clone(), 
            device: self.device.clone(), 
            queue_data: self.queue_data.clone(), 
            dubug: self.dubug.clone(), 
            debug_loader: self.debug_loader.clone(),
         }
    }
    }
    impl Drop for WindowlessEngine{
        fn drop(&mut self) {
            unsafe{
                self.device.destroy_device(None);
                self.debug_loader.destroy_debug_utils_messenger(self.dubug, None);
                self.instance.destroy_instance(None);
            }
            debug!("Engine Destroyed");
    }
    }
    impl traits::IEngineData for WindowlessEngine {
        fn entry(&self) -> ash::Entry {
            self.entry.clone()
        }

        fn instance(&self) -> ash::Instance {
            self.instance.clone()
        }

        fn physical_device(&self) -> ash::vk::PhysicalDevice {
            self.physical_device.clone()
        }

        fn device(&self) -> ash::Device {
            self.device.clone()
        }

        fn queue_data(&self) -> self::QueueCache {
            self.queue_data.clone()
        }

        fn dubug(&self) -> ash::vk::DebugUtilsMessengerEXT {
            self.dubug.clone()
        }

        fn debug_loader(&self) -> ash::extensions::ext::DebugUtils {
            self.debug_loader.clone()
        }

    }
    pub struct Memory{
        device: ash::Device,
        type_index: u32,
        channels: (flume::Sender<enums::EngineMessage>, flume::Receiver<enums::EngineMessage>),
        semaphore: vk::Semaphore,
    }
    impl Memory{
        pub fn new<T: traits::IEngineData>(engine: &T, required_type: vk::MemoryPropertyFlags) -> Memory{
            let instance = engine.instance();
            let physical_device = engine.physical_device();
            let device = engine.device();
            let channels = flume::unbounded();

            unsafe {
                let mem_props = instance.get_physical_device_memory_properties(physical_device);
                let mut selected_type: usize = 0;
    
                //Selecting the corrent memory type
                for type_index in 0..mem_props.memory_types.len(){
                    let mem_type = &mem_props.memory_types[type_index];
                    let heap = &mem_props.memory_heaps[mem_type.heap_index as usize];
                    if mem_type.property_flags & required_type != vk::MemoryPropertyFlags::empty() {
                        debug!("Found compatible memory");
                        debug!("Type index: {}, Type property: {:?}, Type heap: {}", type_index, mem_props.memory_types[type_index].property_flags, mem_props.memory_types[type_index].heap_index);
                        if mem_props.memory_types[selected_type].property_flags & required_type != vk::MemoryPropertyFlags::empty() {
                            if heap.size > mem_props.memory_heaps[mem_props.memory_types[selected_type].heap_index as usize].size && type_index != selected_type{
                                debug!("  Selecting Memory Type");
                                selected_type = type_index;
                            }
                        }
                        else {
                            debug!("Previously selected memory is of wrong type, selecting current memory type");
                            selected_type = type_index;
                        }
                    }
                }
    
                debug!("Memory targeting heap: {} using type: {}", mem_props.memory_types[selected_type].heap_index, selected_type);
                
                let semaphore = device.create_semaphore(&vk::SemaphoreCreateInfo::builder().build(), None).unwrap();
    
                Memory{ 
                    device, 
                    type_index: selected_type as u32, 
                    channels,
                    semaphore,
                 }
            }


        }
        pub fn get_buffer(&mut self, c_info: vk::BufferCreateInfo){
            let buffer: vk::Buffer;
            unsafe{
                
            }
        }
    }
    impl Drop for Memory{
        fn drop(&mut self) {
        debug!("Dropping Memory");
        unsafe{
            self.device.destroy_semaphore(self.semaphore, None);
        }
    }
    }
}

