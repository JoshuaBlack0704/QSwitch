
#[allow(dead_code)]
pub mod enums{
    use ash;
    use ash::vk;
    use flume;

    pub enum EngineMessage{}
    pub enum MemoryMessage{
        //Sector index, layout, layers
        ImageInfoUpdate(usize, vk::ImageCreateInfo, vk::ImageSubresourceLayers),
        BindingUpdate(MemorySector),
    }
    #[derive(Clone)]
    pub enum MemorySector{
        //Allocation index, sector index, StartOffset, allocated reqs
        Empty(usize, usize, vk::DeviceSize, vk::MemoryRequirements),
        //(Allocation index, sector index, Object, create info, start offset, ..., memory requirements, channel)
        Buffer(usize, usize, vk::Buffer, vk::BufferCreateInfo, vk::DeviceSize, vk::MemoryRequirements, flume::Sender<MemoryMessage>),
        Image(usize, usize, vk::Image, vk::ImageCreateInfo, vk::DeviceSize, vk::ImageSubresourceLayers, vk::MemoryRequirements, flume::Sender<MemoryMessage>),
    }
    // pub enum DescriptorMessage{
    //     WriteInfoUpdate(core::DescriptorBindingReceipt, DescriptorInfoType),
    // }
    #[derive(Clone)]
    pub enum DescriptorInfoType{
        Image(vk::DescriptorImageInfo),
        Buffer(vk::DescriptorBufferInfo),
    }
}
 
#[allow(dead_code)]
pub mod traits{
    use flume;
    use ash;
    use ash::vk;
    use crate::{core};

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
    // pub trait IDescriptorEntryPoint {
    //     fn add_binding(&mut self, descriptor_type: vk::DescriptorType, stage: vk::ShaderStageFlags, info: enums::DescriptorInfoType, subscriber: flume::Sender<enums::DescriptorMessage>) -> (core::DescriptorBindingReceipt, flume::Sender<enums::DescriptorMessage>);
    // }

    pub trait ICommandPool{
        fn get_command_buffers(&self, a_info: vk::CommandBufferAllocateInfo) -> Vec<vk::CommandBuffer>;
        fn reset(&self);
    }

    pub trait IVulkanVertex {
        fn get_format(&self);
        fn get_pos(&self);
    }
}

#[allow(dead_code, unused)]
pub mod core{
    use log::debug;
    use shaderc;
    use ash;
    use ash::{vk, Entry};
    use winit::dpi::PhysicalSize;
    use std::{string::String, ffi::CStr, os::raw::c_char};
    use std::borrow::{Cow, Borrow};
    use crate::traits::{self, IEngineData};

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
                    if has_graphics_present && has_transfer && has_compute {
                        debug!("All queue type available, Device {:?} selected", name);
                        return Some(*physical_device)
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
        surface_format: vk::Format,
    }
    impl Engine{
        pub fn init(validate: bool) -> (winit::event_loop::EventLoop<()>, winit::window::Window, Engine){
            let engine:Engine;

            let event_loop = winit::event_loop::EventLoop::new();
            let window = winit::window::WindowBuilder::new()
                .with_title("Ray tracer!")
                .with_inner_size(PhysicalSize::new(1000,1000))
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
                    .api_version(vk::API_VERSION_1_3)
                    .build();

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
                            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
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
                    ash::extensions::khr::AccelerationStructure::name().as_ptr(),
                    ash::extensions::khr::DeferredHostOperations::name().as_ptr(),
                    ash::extensions::khr::RayTracingPipeline::name().as_ptr(),
                    ash::extensions::khr::BufferDeviceAddress::name().as_ptr(),
                    #[cfg(any(target_os = "macos", target_os = "ios"))]
                        KhrPortabilitySubsetFn::name().as_ptr(),
                ];
                let mut ray_features = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder().ray_tracing_pipeline(true);
                let mut acc_features = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder().acceleration_structure(true);
                let mut features13 = vk::PhysicalDeviceVulkan13Features::builder().dynamic_rendering(true).build();
                let mut features12 = vk::PhysicalDeviceVulkan12Features::builder().timeline_semaphore(true).buffer_device_address(true).build();
                let mut features11 = vk::PhysicalDeviceVulkan11Features::builder().build();
                let mut features = vk::PhysicalDeviceFeatures2::builder()
                    .push_next(&mut features11)
                    .push_next(&mut features12)
                    .push_next(&mut features13)
                    .push_next(&mut acc_features)
                    .push_next(&mut ray_features);
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
                let (swapchain_info, swapchain, swapchain_images ,surface_format) = Engine::get_swapchain(&pdevice, &surface, &surface_loader, &swapchain_loader, None);
                
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
                    surface_format,
                    
                 }
            }

            (event_loop, window, engine)
        }
        pub fn refresh_swapchain(&mut self){
            let (swapchain_info, swapchain, swapchain_images, format) = Engine::get_swapchain(&&self.physical_device, &self.surface, &self.surface_loader, &self.swapchain_loader, Some(self.swapchain));
            self.swapchain = swapchain;
            self.swapchain_info = swapchain_info;
            self.swapchain_images = swapchain_images;
            self.surface_format = format;
            debug!("Refreshed swapchain to size: {} x {}", swapchain_info.image_extent.width, swapchain_info.image_extent.height);
        }
        pub fn get_swapchain(physical_device: &vk::PhysicalDevice, surface: &vk::SurfaceKHR, surface_loader: &ash::extensions::khr::Surface, swapchain_loader: &ash::extensions::khr::Swapchain, old_swapchain: Option<ash::vk::SwapchainKHR>)-> (ash::vk::SwapchainCreateInfoKHR, ash::vk::SwapchainKHR, Vec<vk::Image>, vk::Format){
            unsafe {
                
                //clearing the swapchain
                match old_swapchain {
                    Some(swapchain) => {swapchain_loader.destroy_swapchain(swapchain, None);},
                    None => {}
                }
                let possible_formats = surface_loader
                .get_physical_device_surface_formats(*physical_device, *surface)
                .unwrap();
                debug!("{:?}", possible_formats);
                let surface_format = possible_formats[0];
    
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
                (swapchain_create_info.build(), swapchain.unwrap(), images, surface_format.format)
        }
    }
        pub fn get_swapchain_images(&self) -> &Vec<vk::Image> {
            &self.swapchain_images
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
            surface_format: self.surface_format.clone(),
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
            self.surface.clone()
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
                    ash::extensions::khr::AccelerationStructure::name().as_ptr(),
                    ash::extensions::khr::DeferredHostOperations::name().as_ptr(),
                    ash::extensions::khr::RayTracingPipeline::name().as_ptr(),
                    ash::extensions::khr::BufferDeviceAddress::name().as_ptr(),
                    #[cfg(any(target_os = "macos", target_os = "ios"))]
                        KhrPortabilitySubsetFn::name().as_ptr(),
                ];
                let mut ray_features = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder().ray_tracing_pipeline(true);
                let mut acc_features = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder().acceleration_structure(true);
                let mut features13 = vk::PhysicalDeviceVulkan13Features::builder().dynamic_rendering(true).build();
                let mut features12 = vk::PhysicalDeviceVulkan12Features::builder().timeline_semaphore(true).buffer_device_address(true).build();
                let mut features11 = vk::PhysicalDeviceVulkan11Features::builder().build();
                let mut features = vk::PhysicalDeviceFeatures2::builder()
                    .push_next(&mut features11)
                    .push_next(&mut features12)
                    .push_next(&mut features13)
                    .push_next(&mut ray_features)
                    .push_next(&mut acc_features);
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

    pub mod memory{
    use std::{ffi::c_void};
    use log::{self, debug};
    use ash::vk::{self};
    use crate::core;

    use crate::traits::{IEngineData, ICommandPool};

    #[derive(Clone)]
        pub enum DescriptorWriteType{
            Buffer([vk::DescriptorBufferInfo;1]),
            Image([vk::DescriptorImageInfo;1]),
            AccelerationStructure(Option<Box<[vk::AccelerationStructureKHR;1]>>, vk::WriteDescriptorSetAccelerationStructureKHR)
        }

        #[doc = "Safe clonable structure that provides helper functions and data needed to resolve different requirements"]
        #[doc = "such as min offset alignments and such"]
        #[derive(Clone)]
        pub struct AllocationDataStore{
            instance: ash::Instance,
            physical_device: vk::PhysicalDevice,
            device: ash::Device,
            props: vk::PhysicalDeviceProperties,
            pd_mem_props: vk::PhysicalDeviceMemoryProperties,
            destroy_allocation: Option<vk::DeviceMemory>,
            destroy_buffer: Option<vk::Buffer>,
            destroy_image: Option<vk::Image>,
        }
        pub struct Allocation{
            store: AllocationDataStore,
            allocation: vk::DeviceMemory,
            alloc_info: vk::MemoryAllocateInfo,
            cursor: u64,
        }
        pub struct Buffer{
            store: AllocationDataStore,
            buffer: vk::Buffer,
            c_info: vk::BufferCreateInfo,
            reqs: vk::MemoryRequirements,
            alloc_info: vk::MemoryAllocateInfo,
            allocation_offset: u64,
            cursor: u64,
            
        }
        #[derive(Clone)]
        pub struct BufferRegion{
            store: AllocationDataStore,
            buffer: vk::Buffer,
            usage: vk::BufferUsageFlags,
            alloc_info: vk::MemoryAllocateInfo,
            allocation_offset: u64,
            buffer_offset: u64,
            size: u64,
        }
        pub struct Image{
            store: AllocationDataStore,

        }

        impl AllocationDataStore{
            pub fn new<T: IEngineData>(engine: &T) -> AllocationDataStore{
                let instance = engine.instance();
                let physical_device = engine.physical_device();
                let device = engine.device();
                
                unsafe{
                    let props = instance.get_physical_device_properties(physical_device);
                    let pd_mem_props = instance.get_physical_device_memory_properties(physical_device);
                    AllocationDataStore { 
                        instance, 
                        physical_device, 
                        device, 
                        props, 
                        pd_mem_props, 
                        destroy_allocation: None, 
                        destroy_buffer: None, 
                        destroy_image: None }
                }
            }
            pub fn get_type(&self, properties: vk::MemoryPropertyFlags) -> u32{
                let mut selected_type: usize = 0;
                    //Selecting the corrent memory type
                    for type_index in 0..self.pd_mem_props.memory_types.len(){
                        let mem_type = &self.pd_mem_props.memory_types[type_index];
                        let heap = &self.pd_mem_props.memory_heaps[mem_type.heap_index as usize];
                        if mem_type.property_flags & properties != vk::MemoryPropertyFlags::empty() {
                            //debug!("Found compatible memory");
                            //debug!("Type index: {}, Type property: {:?}, Type heap: {}", type_index, self.pd_mem_props.memory_types[type_index].property_flags, self.pd_mem_props.memory_types[type_index].heap_index);
                            if self.pd_mem_props.memory_types[selected_type].property_flags & properties != vk::MemoryPropertyFlags::empty() {
                                if heap.size > self.pd_mem_props.memory_heaps[self.pd_mem_props.memory_types[selected_type].heap_index as usize].size && type_index != selected_type{
                                    //debug!("  Selecting Memory Type");
                                    selected_type = type_index;
                                }
                            }
                            else {
                                //debug!("Previously selected memory is of wrong type, selecting current memory type");
                                selected_type = type_index;
                            }
                        }
                    }
                    selected_type as u32
                }
            #[doc = r"Allocates device memory accoring to inputs. **extened_aloc_info** is only used for a p_next chain."]
            pub fn allocate(&self, type_index: u32, byte_size: vk::DeviceSize, a_m_next: *const c_void) -> Allocation{
                let allocation: vk::DeviceMemory;
                let mut aloc_info = vk::MemoryAllocateInfo::builder()
                .allocation_size(byte_size)
                .memory_type_index(type_index)
                .build();
                aloc_info.p_next = a_m_next;
                unsafe{
                    allocation = self.device.allocate_memory(&aloc_info, None).expect("Could not allocate memory");
                    debug!("Allocated memory {:?} on type {} with size {}", allocation, type_index, byte_size);
                }


                let mut store = self.clone();
                store.destroy_allocation = Some(allocation);
                Allocation { store: store, allocation, alloc_info: aloc_info, cursor: 0 }
            }
            #[doc = "Uses a type and count to determine byte size"]
            pub fn allocate_typed<T>(&self, type_index: u32, count: usize, a_m_next: *const c_void) -> Allocation{
                let size = std::mem::size_of::<T>() * count;
                self.allocate(type_index, size as u64, a_m_next)
            }
            pub fn get_device_props(&self) -> vk::PhysicalDeviceProperties{
                self.props
            }
        }
        impl Drop for AllocationDataStore{
            fn drop(&mut self) {
                match self.destroy_allocation {
                    Some(a) => {
                        debug!("Destroying allocation {:?}", a);
                        unsafe{
                            self.device.free_memory(a, None);
                        }
                    },
                    None => {},
                }
                match self.destroy_buffer {
                    Some(b) => {
                        debug!("Destroying buffer {:?}", b);
                        unsafe{
                            self.device.destroy_buffer(b, None);
                        }
                    },
                    None => {},
                }
                match self.destroy_image {
                    Some(i) => {
                        debug!("Destroying image {:?}", i);
                        unsafe{
                            self.device.destroy_image(i, None);
                        }
                    },
                    None => {},
                }
        }
        }
        impl Allocation{
            pub fn get_buffer(&mut self, usage: vk::BufferUsageFlags, size: u64, queue_families: Option<&[u32]>, flags: vk::BufferCreateFlags, p_next: *const c_void) -> Buffer{
                let buffer: vk::Buffer;
                let reqs: vk::MemoryRequirements;
                let mut c_info:vk::BufferCreateInfo;
                let mut target_address: u64 = 0;
                match queue_families{
                    Some(q) => {
                        c_info = vk::BufferCreateInfo::builder()
                        .flags(flags)
                        .size(size)
                        .usage(usage)
                        .sharing_mode(vk::SharingMode::CONCURRENT)
                        .queue_family_indices(q)
                        .build();
                    },
                    None => {
                        c_info = vk::BufferCreateInfo::builder()
                        .flags(flags)
                        .size(size)
                        .usage(usage)
                        .build();
                    },
                }
                c_info.p_next = p_next;
                unsafe{
                    buffer = self.store.device.create_buffer(&c_info, None).expect("Could not create buffer");
                    reqs = self.store.device.get_buffer_memory_requirements(buffer);

                    if self.cursor != 0{
                        target_address = (self.cursor / reqs.alignment + 1) * reqs.alignment;
                    }
                    assert!(target_address + reqs.size <= self.alloc_info.allocation_size);
                    self.cursor = target_address + size;
                    self.store.device.bind_buffer_memory(buffer, self.allocation, target_address).expect("Could not bind buffer");
                }


                let mut store = self.store.clone();
                store.destroy_allocation = None;
                store.destroy_buffer = Some(buffer);

                debug!("Created buffer {:?} on allocation {:?} at {} with size {}", buffer, self.allocation, target_address, reqs.size);

                Buffer { store: store, buffer, c_info, cursor: 0, reqs, allocation_offset: target_address, alloc_info: self.alloc_info }
            }
            pub fn get_buffer_typed<T>(&mut self, usage: vk::BufferUsageFlags, count: usize, queue_families: Option<&[u32]>, flags: vk::BufferCreateFlags, p_next: *const c_void) -> Buffer{
                let size = std::mem::size_of::<T>() * count;
                self.get_buffer(usage, size as u64, queue_families, flags, p_next)
            }
            pub fn copy_from_ram(&self, src: *const u8, byte_count: usize, dst: &BufferRegion){
                let target_allocation = self.allocation;
                let target_offset = dst.allocation_offset;

                let mapped_range = vk::MappedMemoryRange::builder()
                    .memory(target_allocation)
                    .offset(0)
                    .size(vk::WHOLE_SIZE)
                    .build();
        
                unsafe {
                    debug!("Copying {} bytes from {:?} to allocation {:?} at {} targeting buffer {:?}", byte_count, src, target_allocation, target_offset, dst.buffer);
                    let dst = (self.store.device.map_memory(target_allocation, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()).unwrap() as *mut u8).offset(target_offset as isize);
                    std::ptr::copy_nonoverlapping(src, dst, byte_count);
                    self.store.device.flush_mapped_memory_ranges(&vec![mapped_range]).unwrap();
                    self.store.device.unmap_memory(target_allocation);
                }
            }
            pub fn copy_from_ram_typed<T>(&self, src: *const T, count: usize, dst: &BufferRegion){
                let byte_count = std::mem::size_of::<T>() * count;
                let src = src as *const u8;
                self.copy_from_ram(src, byte_count, dst);
            }
            pub fn copy_from_ram_slice<T>(&self, src: &[T], dst: &BufferRegion){
                let count = src.len();
                let src = src.as_ptr();
                self.copy_from_ram_typed(src, count, dst);
            }
            pub fn copy_to_ram(&self, src: &BufferRegion, byte_count: usize, dst: *mut u8){
                let src_allocation = self.allocation;
                let src_offset = src.allocation_offset;
                let mapped_range = vk::MappedMemoryRange::builder()
                .memory(src_allocation)
                .offset(0)
                .size(vk::WHOLE_SIZE)
                .build();
        
                unsafe {
                    debug!("Copying {} bytes to {:?} from allocation {:?} at {}", byte_count, dst, src_allocation, src.allocation_offset);
                    let src = (self.store.device.map_memory(src_allocation, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()).unwrap() as *const u8).offset(src_offset as isize);
                    self.store.device.invalidate_mapped_memory_ranges(&vec![mapped_range]).unwrap();
                    std::ptr::copy_nonoverlapping(src, dst, byte_count);
                    self.store.device.unmap_memory(src_allocation);
                }
            }
            pub fn copy_to_ram_typed<T>(&self, src: &BufferRegion, count: usize, dst: *mut T){
                let byte_count = std::mem::size_of::<T>() * count;
                let dst = dst as *mut u8;
                self.copy_to_ram(src, byte_count, dst);
            }
        }
        impl Buffer {
            pub fn get_region(&mut self, size: u64, custom_alignment: Option<(bool, u64)>) -> BufferRegion{
                let mut target_address = 0;
                match custom_alignment {
                    Some((b,a)) => {
                        if b{
                            //We are aligning to the allocation address for device address operations
                            debug!("Region paritioner of buffer {:?} using custom offset alignment of {} based of the allocation address", self.buffer, a);
                            target_address = (((self.allocation_offset + self.cursor) / a + 1) * a) - self.allocation_offset;
                        }
                        else{
                            //We arent aligning to the allocation address
                            if self.cursor != 0 {
                                debug!("Region paritioner of buffer {:?} using custom offset alignment of {}", self.buffer, a);
                                target_address = (self.cursor / a + 1) * a;    
                            }
                        }
                        
                        assert!(target_address + size <= self.c_info.size);
                    },
                    None => {
                        if self.cursor != 0 {
                            if self.c_info.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
                                debug!("Region paritioner of buffer {:?} using storage buffer offset alignment of {}", self.buffer, self.store.props.limits.min_storage_buffer_offset_alignment);
                                target_address = (self.cursor / self.store.props.limits.min_storage_buffer_offset_alignment + 1) * self.store.props.limits.min_storage_buffer_offset_alignment;
                            }
                            else {
                                target_address = self.cursor;
                            }    
                        }
                        assert!(target_address + size <= self.c_info.size);
                    },
                };
                self.cursor = target_address + size;


                let mut store = self.store.clone();
                store.destroy_buffer = None;
                debug!("Partitioned region from buffer {:?} at {} of size {}", self.buffer, target_address, size);
                BufferRegion { store: store, buffer: self.buffer, usage: self.c_info.usage, allocation_offset: self.allocation_offset + target_address, buffer_offset: target_address, size: size, alloc_info: self.alloc_info }
            }
            pub fn get_region_typed<T>(&mut self, count: usize, custom_alignment: Option<(bool, u64)>) -> BufferRegion{
                let size = std::mem::size_of::<T>() * count;
                self.get_region(size as u64, custom_alignment)
            }
            pub fn get_regions(&mut self, sizes: &[u64], custom_alignment: Option<(bool, u64)>) -> Vec<BufferRegion>{
                let mut regions = vec![];
                for size in sizes.iter(){
                    regions.push(self.get_region(*size, custom_alignment));
                }
                regions
            }
            pub fn get_regions_typed<T>(&mut self, counts: &[usize], custom_alignment: Option<(bool, u64)>) -> Vec<BufferRegion>{
                let mut regions = vec![];
                for count in counts.iter(){
                    regions.push(self.get_region_typed::<T>(*count, custom_alignment));
                }
                regions
            }
        }
        impl BufferRegion{
            pub fn get_binding(&self, stages: vk::ShaderStageFlags) -> (vk::DescriptorType, u32, vk::ShaderStageFlags, DescriptorWriteType) {
                let ty: vk::DescriptorType;
                let count = 1;
                let write: DescriptorWriteType;

                if self.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
                    ty = vk::DescriptorType::STORAGE_BUFFER;
                }
                else {
                    panic!("No identifiable descriptor type")
                }
                
                write = DescriptorWriteType::Buffer([vk::DescriptorBufferInfo::builder()
                .buffer(self.buffer)
                .offset(self.buffer_offset)
                .range(self.size).build()]);

                (ty, count, stages, write)

            }
            pub fn copy_to_region(&self, cmd: vk::CommandBuffer, dst: &BufferRegion){
                let copy = [self.get_copy_info(dst)];
                unsafe{
                    self.store.device.cmd_copy_buffer(cmd, self.buffer, dst.buffer, &copy);
                    debug!("Recorded copy of {} bytes from buffer {:?} at {} to buffer {:?} at {}", copy[0].size, self.buffer, copy[0].src_offset, dst.buffer, copy[0].dst_offset);
                }
            }
            pub fn get_copy_info(&self, tgt: &BufferRegion) -> vk::BufferCopy {
                assert!(tgt.size >= self.size);
                vk::BufferCopy::builder().src_offset(self.buffer_offset).dst_offset(tgt.buffer_offset).size(self.size).build()
            }
            pub fn get_device_address_const(&self) -> vk::DeviceOrHostAddressConstKHR{
                let base_address: vk::DeviceAddress;
                let ba_info = vk::BufferDeviceAddressInfo::builder().buffer(self.buffer);
                unsafe{
                    base_address = self.store.device.get_buffer_device_address(&ba_info);
                }
                let region_address = base_address + self.buffer_offset;
                debug!("buffer {:?} device address {}, region address: {}", self.buffer, base_address, region_address);

                let address;
                if self.store.get_type(vk::MemoryPropertyFlags::HOST_COHERENT) == self.alloc_info.memory_type_index {
                    address = vk::DeviceOrHostAddressConstKHR{host_address: region_address as *const c_void};
                }
                else{
                    address = vk::DeviceOrHostAddressConstKHR{device_address: region_address};
                }
                address
            }
            pub fn get_device_address(&self) -> vk::DeviceOrHostAddressKHR{
                let base_address: vk::DeviceAddress;
                let ba_info = vk::BufferDeviceAddressInfo::builder().buffer(self.buffer);
                unsafe{
                    base_address = self.store.device.get_buffer_device_address(&ba_info);
                }
                let region_address = base_address + self.buffer_offset;
                debug!("buffer {:?} device address {}, region address: {}", self.buffer, base_address, region_address);

                let address;
                if self.store.get_type(vk::MemoryPropertyFlags::HOST_COHERENT) == self.alloc_info.memory_type_index {
                    address = vk::DeviceOrHostAddressKHR{host_address: region_address as *mut c_void};
                }
                else{
                    address = vk::DeviceOrHostAddressKHR{device_address: region_address};
                }
                address
            }
            pub fn get_buffer(&self) -> vk::Buffer {
                self.buffer
            }
            pub fn get_buffer_offset(&self) -> u64 {
                self.buffer_offset
            }
            pub fn get_allocation_offset(&self) -> u64 {
                self.allocation_offset
            }
            pub fn get_size(&self) -> u64 {
                self.size
            }
        }
        #[derive(Clone)]
        pub struct DescriptorDataStore{
            device: ash::Device,
            props: vk::PhysicalDeviceProperties,
            destroy_pool: Option<vk::DescriptorPool>,
            destroy_set_layouts: Option<Vec<vk::DescriptorSetLayout>>,
        }
        
        #[derive(Clone)]
        pub struct DescriptorSetOutline{
            create_set_layout_flags: vk::DescriptorSetLayoutCreateFlags,
            create_set_layout_next: *const c_void,
            allocate_set_next: *const c_void,
            bindings: Vec<(vk::DescriptorSetLayoutBinding, DescriptorWriteType)>
        }
        pub struct DescriptorSet{
            store: DescriptorDataStore,
            outline: DescriptorSetOutline,
            set: vk::DescriptorSet,
            layout: vk::DescriptorSetLayout,
        }
        pub struct DescriptorStack{
            store: DescriptorDataStore,
            pool: vk::DescriptorPool,
            sets: Vec<DescriptorSet>,
        }
        impl DescriptorDataStore{
            pub fn new<T: IEngineData>(engine: &T) -> DescriptorDataStore{
                unsafe{
                    let device = engine.device();
                    let props = engine.instance().get_physical_device_properties(engine.physical_device());
                    DescriptorDataStore { 
                        device, 
                        props, 
                        destroy_pool: None, 
                        destroy_set_layouts: None }
                        
                }
            }
            pub fn get_descriptor_stack(&self, outlines: &[DescriptorSetOutline], c_p_flags: vk::DescriptorPoolCreateFlags, c_p_next: *const c_void, a_s_next: *const c_void) -> DescriptorStack{

                let mut pool_sizes: Vec<vk::DescriptorPoolSize> = Vec::with_capacity(outlines.len());
                let pool: vk::DescriptorPool;
                let mut layouts: Vec<vk::DescriptorSetLayout> = Vec::with_capacity(outlines.len());
                let allocated_sets: Vec<vk::DescriptorSet>;
                let mut sets: Vec<DescriptorSet> = Vec::with_capacity(outlines.len());

                for outline in outlines.iter(){
                    for (binding, _) in outline.bindings.iter(){
                        let found = pool_sizes.iter().enumerate().find(|(_,s)| s.ty == binding.descriptor_type);
                        match found {
                            Some((i, _)) => {pool_sizes[i].descriptor_count += 1;},
                            None => {pool_sizes.push(vk::DescriptorPoolSize::builder().ty(binding.descriptor_type).descriptor_count(1).build());},
                        }
                    }
                    let bindings:Vec<vk::DescriptorSetLayoutBinding> = outline.bindings.iter().map(|(b,_)| *b).collect();
                    let mut c_l_info = vk::DescriptorSetLayoutCreateInfo::builder()
                    .flags(outline.create_set_layout_flags)
                    .bindings(&bindings)
                    .build();
                    c_l_info.p_next = outline.create_set_layout_next;
                    unsafe{
                        layouts.push(self.device.create_descriptor_set_layout(&c_l_info, None).expect("Could not create descriptor set"));
                        debug!("Created descriptor set layout {:?}", layouts.last());
                    }

                }

                let mut c_p_info = vk::DescriptorPoolCreateInfo::builder()
                .flags(c_p_flags)
                .max_sets(outlines.len() as u32)
                .pool_sizes(&pool_sizes)
                .build();
                c_p_info.p_next = c_p_next;
                unsafe{
                    pool = self.device.create_descriptor_pool(&c_p_info, None).expect("Could not create descriptor pool");
                    debug!("Created descriptor pool {:?}", pool);
                }

                let mut a_info = vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(&layouts)
                .build();
                a_info.p_next = a_s_next;
                unsafe{
                    allocated_sets = self.device.allocate_descriptor_sets(&a_info).expect("Could not allocate descriptor sets");
                    debug!("Created descriptor sets {:?}", allocated_sets);
                }

                let mut writes:Vec<vk::WriteDescriptorSet> = Vec::with_capacity(outlines.len()*2);

                for (index,set) in allocated_sets.iter().enumerate(){
                    let mut outline = outlines.get(index).unwrap().clone();
                    let layout = layouts.get(index).unwrap().clone();

                    for binding in outline.bindings.iter_mut(){
                        match &mut binding.1 {
                            DescriptorWriteType::Buffer(b) => {
                                let write = vk::WriteDescriptorSet::builder()
                                .dst_set(*set)
                                .dst_array_element(0)
                                .dst_binding(binding.0.binding)
                                .descriptor_type(binding.0.descriptor_type)
                                .buffer_info(b)
                                .build();
                                debug!("Generated descriptor set write {:?}", write);
                                writes.push(write);
                            },
                            DescriptorWriteType::Image(i) => {
                                let write = vk::WriteDescriptorSet::builder()
                                .dst_set(*set)
                                .dst_array_element(0)
                                .dst_binding(binding.0.binding)
                                .descriptor_type(binding.0.descriptor_type)
                                .image_info(i)
                                .build();
                                debug!("Generated descriptor set write {:?}", write);
                                writes.push(write);
                            },
                            DescriptorWriteType::AccelerationStructure(data, acc) => {
                                let mut write = vk::WriteDescriptorSet::builder()
                                .dst_set(*set)
                                .dst_array_element(0)
                                .dst_binding(binding.0.binding)
                                .descriptor_type(binding.0.descriptor_type)
                                .push_next(acc)
                                .build();
                                println!("Acc_struct write {:?}", acc);

                                write.descriptor_count = 1;
                                debug!("Generated descriptor set write {:?}", write);
                                writes.push(write);
                            },
                        }
                    }
                    let data = DescriptorSet{ 
                        store: self.clone(), 
                        outline, 
                        set: *set, 
                        layout };
                    sets.push(data);
                }

                unsafe{
                    self.device.update_descriptor_sets(&writes, &[]);
                    debug!("Wrote descriptor sets");
                }

                let mut store = self.clone();
                store.destroy_pool = Some(pool);
                store.destroy_set_layouts = Some(layouts);

                DescriptorStack{ store: store, pool, sets }

            }
        }        
        impl Drop for DescriptorDataStore{
            fn drop(&mut self) {
                match self.destroy_pool {
                    Some(p) => {
                        debug!("Destroying descriptor pool {:?}", p);
                        unsafe{
                            self.device.destroy_descriptor_pool(p, None);
                        }
                    },
                    None => {},
                }
                match self.destroy_set_layouts.clone() {
                    Some(l) => {
                        for layout in l.iter(){
                            debug!("Destroying descritor set layout {:?}", l);
                            unsafe{
                                self.device.destroy_descriptor_set_layout(*layout, None);
                            }
                        }
                    },
                    None => {},
                }
    }
        }
        impl DescriptorSetOutline{
            pub fn new(c_l_flags: vk::DescriptorSetLayoutCreateFlags, c_l_next: *const c_void, a_s_next: *const c_void) -> DescriptorSetOutline{
                DescriptorSetOutline { create_set_layout_flags: c_l_flags, create_set_layout_next: c_l_next, allocate_set_next: a_s_next, bindings: vec![] }
            }
            //                                                            ty, count, stages              write
            pub fn add_binding(&mut self, bindable_data: (vk::DescriptorType, u32, vk::ShaderStageFlags, DescriptorWriteType)) -> u32{
                let (ty, count, stage, write) = bindable_data;
                let binding = vk::DescriptorSetLayoutBinding::builder()
                .binding(self.bindings.len() as u32)
                .descriptor_type(ty)
                .descriptor_count(count)
                .stage_flags(stage)
                .build();
                let binding_index = binding.binding;
                self.bindings.push((binding, write));
                binding_index
            }
        }
        impl DescriptorStack{
            pub fn get_set_layout(&self, set_index: usize) -> vk::DescriptorSetLayout{
                self.sets[set_index].layout
            }
            pub fn get_set(&self, set_index:usize) -> vk::DescriptorSet{
                self.sets[set_index].set
            }
        }
        
        
    }
    pub mod ray_tracing{
        use crate::core::Shader;
        use std::ffi::{c_void, CString};
        use cgmath;
        use ash::vk::{self, Packed24_8, PipelineLayoutCreateInfo};
        use log::debug;

        use crate::{core::{self, memory::{Allocation, Buffer, AllocationDataStore, BufferRegion}, CommandPool, sync::{self, Fence}}, traits::{IEngineData, ICommandPool}};

        use super::memory::DescriptorWriteType;

        #[doc = "Must survive as long as the create blas store command buffer is executing"]
    pub struct BlasStoreCreateRecipt{
        scratch_gpu_mem: Allocation,
        cpu_mem: Allocation,
        v_copy: Buffer,
        i_copy: Buffer,
        s_buffer: Buffer,
    }
    pub struct BlasStore<V>{
        allocator: AllocationDataStore,
        gpu_mem: Allocation,
        acc_loader: ash::extensions::khr::AccelerationStructure,
        vertex_buffer: Buffer,
        index_buffer: Buffer,
        blas_buffer: Buffer,
        constucted_structures: Vec<ConstructedBlas<V>>,
    }
    #[derive(Clone)]
    enum BlasGeometeryType {
        Triangles(vk::GeometryFlagsKHR, vk::BuildAccelerationStructureFlagsKHR, *const c_void, *const c_void, *const c_void)
    }
    #[derive(Clone)]
    pub struct BlasOutline<V>{
        vertex_data: Vec<V>,
        vertex_format: vk::Format,
        index_data: Vec<u32>,
        transform: Option<vk::DeviceOrHostAddressConstKHR>,
        geo_type: BlasGeometeryType,
    }
    pub struct ConstructedBlas<V>{
        acc_structure: vk::AccelerationStructureKHR,
        outline: BlasOutline<V>,
        vertex_region: BufferRegion,
        index_region: BufferRegion,
        blas_region: BufferRegion,
        acc_struct_address: vk::DeviceOrHostAddressConstKHR,
    }
    pub struct Tlas{
        device: ash::Device,
        acc_loader: ash::extensions::khr::AccelerationStructure,
        tlas: vk::AccelerationStructureKHR,
        allocator: AllocationDataStore,
        gpu_mem: Allocation,
        tlas_buffer: Buffer,
        tlas_region: BufferRegion,
        update_scratch_region: BufferRegion,
    }
    pub struct TlasBuildRecipt{
        gpu_scratch_mem: Allocation,
        scratch_buffer: Buffer,
    }
    
    impl<V: Clone> BlasOutline<V>{
        pub fn new_triangle(vertex_data: &[V], vertex_format: vk::Format, index_data: &[u32], transform: Option<vk::DeviceOrHostAddressConstKHR>, geo_flags: vk::GeometryFlagsKHR, build_flags: vk::BuildAccelerationStructureFlagsKHR, t_d_next: *const c_void, g_i_next: *const c_void, g_b_next: *const c_void) -> BlasOutline<V>{
            BlasOutline { 
                vertex_data: vertex_data.to_vec(), 
                vertex_format, 
                index_data: index_data.to_vec(), 
                transform,
                geo_type: BlasGeometeryType::Triangles(geo_flags, build_flags, t_d_next, g_i_next, g_b_next)
            }
        }
        fn get_size_info(&self, acc_loader: &ash::extensions::khr::AccelerationStructure) -> (vk::AccelerationStructureBuildSizesInfoKHR, u64, u64){
            let geo_data = match self.geo_type{
                BlasGeometeryType::Triangles(geo_flags, build_flags, t_d_next, g_i_next, g_b_next) => {
                    let mut triangles_data = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
                    .vertex_format(self.vertex_format)
                    .vertex_stride(std::mem::size_of_val(&self.vertex_data[0]) as u64)
                    .max_vertex(self.vertex_data.len() as u32)
                    .index_type(vk::IndexType::UINT32)
                    .build();
                    triangles_data.p_next = t_d_next;
                    let mut geo_data = vk::AccelerationStructureGeometryDataKHR::default();
                    geo_data.triangles = triangles_data;
                    let mut geo_info = vk::AccelerationStructureGeometryKHR::builder()
                    .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                    .geometry(geo_data)
                    .flags(geo_flags)
                    .build();
                    geo_info.p_next = g_i_next;
                    let geo_info_array = vec![geo_info];
                    let mut build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                    .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                    .flags(build_flags)
                    .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
                    .geometries(&geo_info_array)
                    .build();
                    build_info.p_next = g_b_next;
                    let primatives = [(self.index_data.len()/3) as u32];
                    let vertex_data_size = (std::mem::size_of_val(&self.vertex_data[0]) as u64) * self.vertex_data.len() as u64;
                    let index_data_size = (std::mem::size_of_val(&self.index_data[0]) as u64) * self.index_data.len() as u64;
                    unsafe{
                        (acc_loader.get_acceleration_structure_build_sizes(vk::AccelerationStructureBuildTypeKHR::DEVICE, &build_info, &primatives), vertex_data_size, index_data_size)
                    }
                },
            };
            geo_data
        }
        fn record_build(&self, acc_loader: &ash::extensions::khr::AccelerationStructure, cmd: vk::CommandBuffer, vertex_region: &BufferRegion, index_region: &BufferRegion, blas_region: &BufferRegion, scratch_region: &BufferRegion) -> vk::AccelerationStructureKHR {
            let acc_struct = match self.geo_type { BlasGeometeryType::Triangles(geo_flags, build_flags, t_d_next, g_i_next, g_b_next) => {
                
                
                let mut triangles_data = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
                .vertex_format(self.vertex_format)
                .vertex_data(vertex_region.get_device_address_const())
                .vertex_stride(std::mem::size_of_val(&self.vertex_data[0]) as u64)
                .max_vertex(self.vertex_data.len() as u32)
                .index_type(vk::IndexType::UINT32)
                .index_data(index_region.get_device_address_const())
                .build();
                triangles_data.p_next = t_d_next;


                let mut geo_data = vk::AccelerationStructureGeometryDataKHR::default();
                geo_data.triangles = triangles_data;


                let mut geo_info = vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
                .geometry(geo_data)
                .flags(geo_flags)
                .build();
                geo_info.p_next = g_i_next;
                let geo_info_array = vec![geo_info];

                let acceleration_structure;
                let ac_info = vk::AccelerationStructureCreateInfoKHR::builder()
                .buffer(blas_region.get_buffer())
                .offset(blas_region.get_buffer_offset())
                .size(blas_region.get_size())
                .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                .build();
                unsafe{
                    acceleration_structure = acc_loader.create_acceleration_structure(&ac_info, None).expect("Could not create acceleration structure");
                    debug!("Created acceleration structure {:?}", acceleration_structure);
                }
                
                
                let mut build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
                .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                .flags(build_flags)
                .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
                .dst_acceleration_structure(acceleration_structure)
                .geometries(&geo_info_array)
                .scratch_data(scratch_region.get_device_address())
                .build();
                build_info.p_next = g_b_next;



                let primatives = [(self.index_data.len()/3) as u32];
                let build_range = [vk::AccelerationStructureBuildRangeInfoKHR::builder().primitive_count(primatives[0]).build()];


                let build_infos = [build_info];
                let build_ranges = [build_range.as_slice()];
                

                unsafe{
                    acc_loader.cmd_build_acceleration_structures(cmd, &build_infos, &build_ranges);
                }
                acceleration_structure
            },};
            acc_struct
        }
    }
    impl<V: Clone> BlasStore<V>{
        pub fn new<T: IEngineData>(engine: &T, cmd: vk::CommandBuffer, outlines: &[BlasOutline<V>]) -> (BlasStore<V>, BlasStoreCreateRecipt){
            let acc_loader = ash::extensions::khr::AccelerationStructure::new(&engine.instance(), &engine.device());
            let mut requested_vertex_data_regions: Vec<u64> = Vec::with_capacity(outlines.len());
            let mut requested_index_data_regions : Vec<u64> = Vec::with_capacity(outlines.len());
            let mut requested_blas_regions : Vec<u64> = Vec::with_capacity(outlines.len());
            let mut requested_scratch_regions : Vec<u64> = Vec::with_capacity(outlines.len());
            //We need to pull sizing info from all of our blas outlines
            for (index, outline) in outlines.iter().enumerate(){
                let (sizing_info, v_size, i_size) = outline.get_size_info(&acc_loader);
                let blas_size = sizing_info.acceleration_structure_size;
                let scratch_size = sizing_info.build_scratch_size;
                requested_vertex_data_regions.push(v_size);
                requested_index_data_regions.push(i_size);
                requested_blas_regions.push(blas_size);
                requested_scratch_regions.push(scratch_size);
                debug!("Sizing info for blas {}\n   Vertex data: {}\n   index_data: {}\n   blas_data: {}\n   scratch_data: {}", index, v_size, i_size, blas_size, scratch_size);
            }

            let mut total_size = 0;
            let mut vertex_size = 0;
            let mut index_size= 0;
            let mut blas_size= 0;
            let mut scratch_size= 0;
            let mut copy_size = 0;
            for (i,_) in requested_vertex_data_regions.iter().enumerate(){
                vertex_size += requested_vertex_data_regions[i];
                index_size += requested_index_data_regions[i];
                blas_size += requested_blas_regions[i];
                scratch_size += requested_scratch_regions[i];

                copy_size += requested_vertex_data_regions[i];
                copy_size += requested_index_data_regions[i];

                total_size += requested_vertex_data_regions[i];
                total_size += requested_index_data_regions[i];
                total_size += requested_blas_regions[i];
                total_size += requested_scratch_regions[i];
            }
            debug!("Total blas store size: {}\nNeeded copy size: {}\nSub part sizes:\n   Vertex data: {}\n   index_data: {}\n   blas_data: {}\n   scratch_data: {}", total_size, copy_size, vertex_size, index_size, blas_size, scratch_size);

            let allocator = AllocationDataStore::new(engine);

            let mut acc_props = vk::PhysicalDeviceAccelerationStructurePropertiesKHR::builder().build();
            let mut default_props = vk::PhysicalDeviceProperties2::builder().push_next(&mut acc_props);
            unsafe{
                engine.instance().get_physical_device_properties2(engine.physical_device(), &mut default_props);
            }

            let mut alloc_flags = vk::MemoryAllocateFlagsInfo::builder()
            .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS)
            .build();
            let a_m_next = vk::MemoryAllocateInfo::builder().push_next(&mut alloc_flags).build().p_next;

            let mut gpu_mem = allocator.allocate(allocator.get_type(vk::MemoryPropertyFlags::DEVICE_LOCAL), total_size + 1024*1024, a_m_next);
            let mut scratch_gpu_mem = allocator.allocate(allocator.get_type(vk::MemoryPropertyFlags::DEVICE_LOCAL), scratch_size + 1024*1024, a_m_next);
            let mut cpu_mem = allocator.allocate(allocator.get_type(vk::MemoryPropertyFlags::HOST_COHERENT), copy_size + 1024*1024, 0 as *const c_void);

            let mut vertex_copy = cpu_mem.get_buffer(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC, vertex_size + 1024, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
            let mut index_copy = cpu_mem.get_buffer(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC, index_size + 1024, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);

            let vertex_copy_regions = vertex_copy.get_regions(&requested_vertex_data_regions, None);
            let index_copy_regions = index_copy.get_regions(&requested_index_data_regions, None);

            let mut vertex_buffer = gpu_mem.get_buffer(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR | vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS, vertex_size, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
            let mut index_buffer = gpu_mem.get_buffer(vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR | vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS, index_size, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
            let mut blas_buffer = gpu_mem.get_buffer(vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS, blas_size, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
            let mut scratch_buffer = scratch_gpu_mem.get_buffer(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS, scratch_size, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);

            let vertex_regions = vertex_buffer.get_regions(&requested_vertex_data_regions, None);
            let index_regions = index_buffer.get_regions(&requested_index_data_regions, None);
            let blas_regions = blas_buffer.get_regions(&requested_vertex_data_regions, Some((false, 256 as u64)));
            let scratch_regions = scratch_buffer.get_regions(&requested_index_data_regions, Some((true, acc_props.min_acceleration_structure_scratch_offset_alignment as u64)));

            for (index, outline) in outlines.iter().enumerate(){
                cpu_mem.copy_from_ram_slice(&outline.vertex_data, &vertex_copy_regions[index]);
                cpu_mem.copy_from_ram_slice(&outline.index_data, &index_copy_regions[index]);
            }
            for (index, region) in vertex_copy_regions.iter().enumerate(){
                region.copy_to_region(cmd, &vertex_regions[index]);
                index_copy_regions[index].copy_to_region(cmd, &index_regions[index]);
            }
            let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::NONE).dst_access_mask(vk::AccessFlags::MEMORY_WRITE).build();
            
            unsafe {engine.device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER, vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);}
            
            let mut acc_structs = vec![];
            //Now we attempt to build the command buffers
            for (index, outline) in outlines.iter().enumerate(){
                acc_structs.push(outline.record_build(&acc_loader, cmd, &vertex_regions[index], &index_regions[index], &blas_regions[index], &scratch_regions[index]));
            }
            
            let recipt = BlasStoreCreateRecipt{ scratch_gpu_mem, cpu_mem, v_copy: vertex_copy, i_copy: index_copy, s_buffer: scratch_buffer };
            let mut constructed_blas_data = Vec::with_capacity(outlines.len());
            for (index, acc_struct) in acc_structs.iter().enumerate(){
                let info = vk::AccelerationStructureDeviceAddressInfoKHR::builder()
                .acceleration_structure(*acc_struct)
                .build();
                let acc_struct_address;
                unsafe{
                    acc_struct_address = vk::DeviceOrHostAddressConstKHR{device_address: acc_loader.get_acceleration_structure_device_address(&info)};
                }
                let data = ConstructedBlas{ 
                    acc_structure: *acc_struct, 
                    outline: outlines[index].clone(), 
                    vertex_region: vertex_regions[index].clone(), 
                    index_region: index_regions[index].clone(), 
                    blas_region: blas_regions[index].clone(),
                    acc_struct_address
                };
                constructed_blas_data.push(data);
            }
            
            (BlasStore{ 
                allocator, 
                gpu_mem, 
                vertex_buffer, 
                index_buffer, 
                blas_buffer,
                acc_loader,
                constucted_structures: constructed_blas_data},
            recipt)

        }
        pub fn new_immediate<T: IEngineData>(engine: &T, outlines: &[BlasOutline<V>]) -> BlasStore<V>{
            let pool = core::CommandPool::new(engine, vk::CommandPoolCreateInfo::builder().queue_family_index(engine.queue_data().graphics.1).build());
            let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];
            unsafe{
                engine.device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().build()).unwrap();
            }
            let data = BlasStore::new(engine, cmd, outlines);
            unsafe{
                engine.device().end_command_buffer(cmd).unwrap();
            }
            let cmds = [cmd];
            let submit = [vk::SubmitInfo::builder().command_buffers(&cmds).build()];
            let fence = core::sync::Fence::new(engine, false);
            unsafe{
                engine.device().queue_submit(engine.queue_data().graphics.0, &submit, fence.get_fence()).unwrap();
            }
            fence.wait();    
            data.0            
        }
        pub fn get_acc_struct_address(&self, index: usize) -> vk::DeviceOrHostAddressConstKHR {
            self.constucted_structures[index].acc_struct_address
        }
    }
    impl<V> Drop for BlasStore<V>{
        fn drop(&mut self) {
            unsafe{
                for acc in self.constucted_structures.iter(){
                    debug!("Destroying acceleration structure {:?}", (*acc).acc_structure);
                    self.acc_loader.destroy_acceleration_structure((*acc).acc_structure, None);
                }
            }
}
    }
    impl Tlas{
        pub fn new<T: IEngineData, V>(engine: &T, cmd: vk::CommandBuffer, instance_count: u32, instances_address: vk::DeviceOrHostAddressConstKHR) -> (Tlas, TlasBuildRecipt) {
            let acc_loader = ash::extensions::khr::AccelerationStructure::new(&engine.instance(), &engine.device());
            let sizes = Tlas::get_size_info(&acc_loader, instance_count, instances_address);
            let total_size = sizes.acceleration_structure_size + sizes.update_scratch_size * 2;
            let scratch_size = sizes.build_scratch_size;
            let allocator = AllocationDataStore::new(engine);

            let mut acc_props = vk::PhysicalDeviceAccelerationStructurePropertiesKHR::builder().build();
            let mut default_props = vk::PhysicalDeviceProperties2::builder().push_next(&mut acc_props);
            unsafe{
                engine.instance().get_physical_device_properties2(engine.physical_device(), &mut default_props);
            }

            let mut alloc_flags = vk::MemoryAllocateFlagsInfo::builder()
            .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS)
            .build();
            let a_m_next = vk::MemoryAllocateInfo::builder().push_next(&mut alloc_flags).build().p_next;

            let mut gpu_mem = allocator.allocate(allocator.get_type(vk::MemoryPropertyFlags::DEVICE_LOCAL), total_size + 1024*1024, a_m_next);
            let mut gpu_scratch_mem = allocator.allocate(allocator.get_type(vk::MemoryPropertyFlags::DEVICE_LOCAL), scratch_size + 1024*1024,a_m_next);
            
            let mut tlas_buffer = gpu_mem.get_buffer(vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS, total_size + 1024, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
            let tlas_region = tlas_buffer.get_region(sizes.acceleration_structure_size, Some((false, 256 as u64)));
            let update_scratch_region = tlas_buffer.get_region(sizes.update_scratch_size, Some((true, acc_props.min_acceleration_structure_scratch_offset_alignment as u64)));

            let mut build_scratch_buffer = gpu_scratch_mem.get_buffer(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS, scratch_size + 1024, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
            let build_scratch_region = build_scratch_buffer.get_region(scratch_size, Some((true, acc_props.min_acceleration_structure_scratch_offset_alignment as u64)));

            let tlas_c_info = vk::AccelerationStructureCreateInfoKHR::builder()
            .buffer(tlas_region.get_buffer())
            .offset(tlas_region.get_buffer_offset())
            .size(tlas_region.get_size())
            .build();
            let tlas;
            unsafe{
                tlas = acc_loader.create_acceleration_structure(&tlas_c_info, None).expect("Could not create acceleration structure");
                debug!("Built Top Level Acceleration Structure {:?}", tlas);
            }
            let instance_data = vk::AccelerationStructureGeometryInstancesDataKHR::builder()
            .data(instances_address).build();
            let geo_data = vk::AccelerationStructureGeometryDataKHR{instances: instance_data};

            let geo_info = [vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(geo_data)
            .build()];
            let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .geometries(&geo_info)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .dst_acceleration_structure(tlas)
            .scratch_data(build_scratch_region.get_device_address())
            .build();

            let build_range = [vk::AccelerationStructureBuildRangeInfoKHR::builder().primitive_count(instance_count).build()];
            let build_infos = [build_info];
            let build_ranges = [build_range.as_slice()];

            unsafe{
                acc_loader.cmd_build_acceleration_structures(cmd, &build_infos, &build_ranges);
            }

            (Tlas{
                device: engine.device(),
                acc_loader,
                tlas,
                allocator,
                gpu_mem,
                tlas_buffer,
                tlas_region,
                update_scratch_region,
            },
            TlasBuildRecipt{ 
                gpu_scratch_mem, 
                scratch_buffer: build_scratch_buffer })


        }
        pub fn get_size_info(acc_loader: &ash::extensions::khr::AccelerationStructure, instance_count: u32, instances_address:  vk::DeviceOrHostAddressConstKHR) -> vk::AccelerationStructureBuildSizesInfoKHR {
            let instance_data = vk::AccelerationStructureGeometryInstancesDataKHR::builder()
            .data(instances_address).build();
            let geo_data = vk::AccelerationStructureGeometryDataKHR{instances: instance_data};
            let geo_info = [vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(geo_data)
            .build()];
            let build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .geometries(&geo_info)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .build();
            let instance_count = [instance_count;1];
            unsafe{
                acc_loader.get_acceleration_structure_build_sizes(vk::AccelerationStructureBuildTypeKHR::DEVICE, &build_info, &instance_count)
            }
        }
        pub fn new_immediate<T: IEngineData, V>(engine: &T, instance_count: u32, instances_address: vk::DeviceOrHostAddressConstKHR) -> Tlas{
            let pool = core::CommandPool::new(engine, vk::CommandPoolCreateInfo::builder().queue_family_index(engine.queue_data().graphics.1).build());
            let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];
            unsafe{
                engine.device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().build()).unwrap();
            }
            let data = Tlas::new::<T,V>(engine, cmd, instance_count, instances_address);
            unsafe{
                engine.device().end_command_buffer(cmd).unwrap();
            }
            let cmds = [cmd];
            let submit = [vk::SubmitInfo::builder().command_buffers(&cmds).build()];
            let fence = core::sync::Fence::new(engine, false);
            unsafe{
                engine.device().queue_submit(engine.queue_data().graphics.0, &submit, fence.get_fence()).unwrap();
            }
            fence.wait();
            data.0                
        }
        pub fn get_binding(&self, stages: vk::ShaderStageFlags) -> (vk::DescriptorType, u32, vk::ShaderStageFlags, DescriptorWriteType){
            let descriptor_type = vk::DescriptorType::ACCELERATION_STRUCTURE_KHR;
            let count = 1;
            let tlas = Box::new([self.tlas]);
            let write = vk::WriteDescriptorSetAccelerationStructureKHR::builder()
            .acceleration_structures(&(*tlas))
            .build();
            (descriptor_type, count, stages, DescriptorWriteType::AccelerationStructure(Some(tlas), write))
        }
    }
    impl Drop for Tlas{
        fn drop(&mut self) {
            debug!("Destroying Top Level Acceleration Structure {:?}", self.tlas);
            unsafe{
                self.acc_loader.destroy_acceleration_structure(self.tlas, None);
            }
    }           
    }
    #[derive(Clone)]
    pub struct ObjectOutline<V>{
        pub vertex_data: Vec<V>,
        pub vertex_format: vk::Format,
        pub index_data: Vec<u32>,
        pub inital_pos_data: Vec<cgmath::Vector4<f32>>,
        pub sbt_hit_group_offset: u32,
    }
    pub struct ObjectStore<V>{
        device: ash::Device,
        instance_mem: Allocation,
        instance_buffer: Buffer,
        instance_region: BufferRegion,
        instance_count: u32,
        object_outlines: Vec<ObjectOutline<V>>
    }
    impl<V: Clone> ObjectStore<V> {
        pub fn new<T: IEngineData>(engine: &T, objects: &[ObjectOutline<V>]) -> (ObjectStore<V>, BlasStore<V>){
            let mut blas_outlines = Vec::with_capacity(objects.len());
            for object in objects.iter(){
                blas_outlines.push(BlasOutline::new_triangle(
                    &object.vertex_data, 
                    object.vertex_format, 
                    &object.index_data, 
                    None, 
                    vk::GeometryFlagsKHR::OPAQUE, 
                    vk::BuildAccelerationStructureFlagsKHR::empty(), 
                    0 as *const c_void, 
                    0 as *const c_void, 
                    0 as *const c_void))
            }
            let blas = BlasStore::new_immediate(engine, &blas_outlines);

            let mut instance_data = Vec::with_capacity(objects.len() * objects[0].inital_pos_data.len());
            for (index, object) in objects.iter().enumerate(){
                let acc_struct = unsafe{vk::AccelerationStructureReferenceKHR{
                 device_handle: blas.get_acc_struct_address(index).device_address
                }};
                for pos in object.inital_pos_data.iter(){
                    let transform = vk::TransformMatrixKHR{ matrix: 
                        [1.0,0.0,0.0,pos.x,
                         0.0,1.0,0.0,pos.y,
                         0.0,0.0,1.0,pos.z] };
                    let instance = vk::AccelerationStructureInstanceKHR{ 
                        transform, 
                        instance_custom_index_and_mask: Packed24_8::new(0, 0xff), 
                        instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(object.sbt_hit_group_offset, 0x00000002 as u8), 
                        acceleration_structure_reference: acc_struct };
                    instance_data.push(instance);
                }
            }

            let allocator = AllocationDataStore::new(engine);
            let mut acc_props = vk::PhysicalDeviceAccelerationStructurePropertiesKHR::builder().build();
            let mut default_props = vk::PhysicalDeviceProperties2::builder().push_next(&mut acc_props);
            unsafe{
                engine.instance().get_physical_device_properties2(engine.physical_device(), &mut default_props);
            }

            let mut alloc_flags = vk::MemoryAllocateFlagsInfo::builder()
            .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS)
            .build();
            let a_m_next = vk::MemoryAllocateInfo::builder().push_next(&mut alloc_flags).build().p_next;                

            let mut gpu_mem = allocator.allocate_typed::<vk::AccelerationStructureInstanceKHR>(allocator.get_type(vk::MemoryPropertyFlags::DEVICE_LOCAL), instance_data.len() + 10, a_m_next);
            let mut instance_buffer = gpu_mem.get_buffer_typed::<vk::AccelerationStructureInstanceKHR>(
                vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR, 
                instance_data.len(), None, 
                vk::BufferCreateFlags::empty(), 
                0 as *const c_void);
            let instance_region = instance_buffer.get_region_typed::<vk::AccelerationStructureInstanceKHR>(instance_data.len(), None);

            let mut cpu_mem = allocator.allocate_typed::<vk::AccelerationStructureInstanceKHR>(allocator.get_type(vk::MemoryPropertyFlags::HOST_COHERENT), instance_data.len(), 0 as *const c_void);
            let mut instance_copy = cpu_mem.get_buffer_typed::<vk::AccelerationStructureInstanceKHR>(
                vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::STORAGE_BUFFER, 
                instance_data.len(), 
                None, 
                vk::BufferCreateFlags::empty(), 
                0 as *const c_void);
            let instance_copy_region = instance_copy.get_region_typed::<vk::AccelerationStructureInstanceKHR>(instance_data.len(), None);

            cpu_mem.copy_from_ram_slice(&instance_data, &instance_copy_region);

            let pool = core::CommandPool::new(engine, vk::CommandPoolCreateInfo::builder().queue_family_index(engine.queue_data().graphics.1).build());
            let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];
            unsafe{
                engine.device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().build()).unwrap();
            }
            instance_copy_region.copy_to_region(cmd, &instance_region);
            unsafe{
                engine.device().end_command_buffer(cmd);
            }
            let cmds = [cmd];
            let submit = [vk::SubmitInfo::builder().command_buffers(&cmds).build()];
            let fence = core::sync::Fence::new(engine, false);
            unsafe{
                engine.device().queue_submit(engine.queue_data().graphics.0, &submit, fence.get_fence()).unwrap();
            }
            fence.wait();

            (ObjectStore{ 
                device: engine.device(), 
                instance_mem: gpu_mem, 
                instance_buffer, 
                instance_region, 
                object_outlines: objects.to_vec(),
                instance_count: instance_data.len() as u32, },
            blas)
        }
        pub fn get_instance_address(&self) -> vk::DeviceOrHostAddressConstKHR {
            self.instance_region.get_device_address_const()
        }
        pub fn get_instance_count(&self) -> u32 {
            self.instance_count
        }
    }

    pub struct SbtOutline{
        ray_gen: vk::PipelineShaderStageCreateInfo,
        misses: Vec<vk::PipelineShaderStageCreateInfo>,
        hit_groups: Vec<(Option<vk::PipelineShaderStageCreateInfo>, Option<vk::PipelineShaderStageCreateInfo>,Option<vk::PipelineShaderStageCreateInfo>)>
    }
    impl SbtOutline{
        pub fn new(ray_gen: vk::PipelineShaderStageCreateInfo, misses: &[vk::PipelineShaderStageCreateInfo], hit_groups: &[(Option<vk::PipelineShaderStageCreateInfo>, Option<vk::PipelineShaderStageCreateInfo>,Option<vk::PipelineShaderStageCreateInfo>)]) -> SbtOutline {
            SbtOutline{ ray_gen, misses: misses.to_vec(), hit_groups: hit_groups.to_vec() }
        }
    }
    pub struct RayTracingPipelineCreateRecipt{
        cpu_mem: Allocation,
        cpu_buffer: Buffer,
    }
    pub struct RayTracingPipeline{
        device: ash::Device,
        raytracing_loader: ash::extensions::khr::RayTracingPipeline,
        raytracing_props: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
        sbt_outline: SbtOutline,
        pub layout: vk::PipelineLayout,
        pub pipeline: vk::Pipeline,
        gpu_mem: Allocation,
        shaders_buffer: Buffer,
        shader_regions: (BufferRegion, BufferRegion, BufferRegion),
        pub shader_addresses: (vk::StridedDeviceAddressRegionKHR,vk::StridedDeviceAddressRegionKHR,vk::StridedDeviceAddressRegionKHR),
    }
    impl RayTracingPipeline{
        pub fn new<T:IEngineData>(engine: &T, cmd: vk::CommandBuffer, sbt_outline: SbtOutline, set_layouts: &[vk::DescriptorSetLayout], push_constant_ranges: &[vk::PushConstantRange]) -> (RayTracingPipeline, RayTracingPipelineCreateRecipt) {
            let device = engine.device();
            let raytracing_loader = ash::extensions::khr::RayTracingPipeline::new(&engine.instance(), &device);
            let mut raytracing_props = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::builder().build();
            let mut default_props = vk::PhysicalDeviceProperties2::builder().push_next(&mut raytracing_props);
            unsafe{
                engine.instance().get_physical_device_properties2(engine.physical_device(), &mut default_props);
            }
            debug!("Got ray tracing props: {:?}", raytracing_props);

            let mut stages = vec![];
            let mut groups = vec![];
            
            stages.push(sbt_outline.ray_gen);
            groups.push(vk::RayTracingShaderGroupCreateInfoKHR::builder()
            .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
            .general_shader(0)
            .closest_hit_shader(u32::max_value())
            .any_hit_shader(u32::max_value())
            .intersection_shader(u32::max_value()).build());
            debug!("Added ray gen shader at index 0");
            for miss in sbt_outline.misses.iter(){
                groups.push(vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(stages.len() as u32)
                .closest_hit_shader(u32::max_value())
                .any_hit_shader(u32::max_value())
                .intersection_shader(u32::max_value()).build());
                debug!("Added miss shader at index {}", stages.len());
                stages.push(*miss);
            }
            for (closest_hit, any_hit, intersection) in sbt_outline.hit_groups.iter(){
                let mut group_builder = vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP);
                group_builder = group_builder.general_shader(u32::MAX);
                match closest_hit {
                    Some(s) => {
                        group_builder = group_builder.closest_hit_shader(stages.len() as u32);
                        debug!("Added closest_hit shader at index {}", stages.len());
                        stages.push(*s);
                    },
                    None =>  {group_builder = group_builder.closest_hit_shader(u32::MAX);},
                }
                match any_hit {
                    Some(s) => {
                        group_builder = group_builder.any_hit_shader(stages.len() as u32);
                        debug!("Added any_hit shader at index {}", stages.len());
                        
                        stages.push(*s);
                    },
                    None => {group_builder =group_builder.any_hit_shader(u32::MAX);},
                }
                match intersection {
                    Some(s) => {
                        group_builder = group_builder.intersection_shader(stages.len() as u32);
                        debug!("Added intersection shader at index {}", stages.len());
                        stages.push(*s);
                    },
                    None => {group_builder =group_builder.intersection_shader(u32::MAX);},
                }
                groups.push(group_builder.build());
            }
            let lc_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(&set_layouts)
            .push_constant_ranges(&push_constant_ranges)
            .build();
            let layout = unsafe{device.create_pipeline_layout(&lc_info, None).expect("Could not create pipeline layout")};
            debug!("Built ray tracing pipeline layout {:?}", layout);
            let c_info = [vk::RayTracingPipelineCreateInfoKHR::builder()
            .stages(&stages)
            .groups(&groups)
            .max_pipeline_ray_recursion_depth(2)
            .layout(layout)
            .build()];
            let pipeline = unsafe{raytracing_loader.create_ray_tracing_pipelines(
                vk::DeferredOperationKHR::null(), 
                vk::PipelineCache::null(), 
                &c_info, 
                None).expect("Could not create ray tracing pipeline")[0]};
            debug!("Built ray tracing pipeline {:?}", layout);
            
            let handle_data;

            let shaders = unsafe {
                handle_data = raytracing_loader.get_ray_tracing_shader_group_handles(pipeline, 0, groups.len() as u32, groups.len() * raytracing_props.shader_group_handle_size as usize).expect("Could not get shader handles")
            };

            let ray_gen_size = (raytracing_props.shader_group_handle_size * 1) as u64;
            let miss_size = (raytracing_props.shader_group_handle_size * sbt_outline.misses.len() as u32) as u64;
            let hit_group_size = (raytracing_props.shader_group_handle_size * sbt_outline.hit_groups.len() as u32) as u64;

            let mut shaders: Vec<Vec<u8>> = vec![];
            for index in 0..(handle_data.len()/raytracing_props.shader_group_handle_size as usize){
                let shader_count = index*raytracing_props.shader_group_handle_size as usize;
                let shader = &handle_data[shader_count..shader_count+raytracing_props.shader_group_handle_size as usize];
                shaders.push(shader.to_vec());
            }
            let ray_gen_handle = shaders[0].to_vec();
            let miss_handles_seperated = shaders[1..sbt_outline.misses.len()+1 as usize].to_vec();
            let hit_handles_seperated = shaders[sbt_outline.misses.len()+1..shaders.len()].to_vec();
            let mut miss_handles = vec![];
            let mut hit_handles = vec![];
            for sep in miss_handles_seperated.iter(){
                miss_handles.extend_from_slice(&sep);
            }
            for sep in hit_handles_seperated.iter(){
                hit_handles.extend_from_slice(&sep);
            }


            let mut alloc_flags = vk::MemoryAllocateFlagsInfo::builder()
            .flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS)
            .build();
            let a_m_next = vk::MemoryAllocateInfo::builder().push_next(&mut alloc_flags).build().p_next;
            let allocator = AllocationDataStore::new(engine);
            let mut gpu_mem = allocator.allocate_typed::<u8>(allocator.get_type(vk::MemoryPropertyFlags::DEVICE_LOCAL), 1024, a_m_next);
            let mut cpu_mem = allocator.allocate_typed::<u8>(allocator.get_type(vk::MemoryPropertyFlags::HOST_COHERENT), 1024, a_m_next);
            let mut handles_copy = cpu_mem.get_buffer_typed::<u8>(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC, handle_data.len()*3, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
            let mut handles_buffer = gpu_mem.get_buffer_typed::<u8>(
                vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS | vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR | vk::BufferUsageFlags::TRANSFER_DST, 
                handle_data.len()*3, 
                None, 
                vk::BufferCreateFlags::empty(), 
                0 as *const c_void);

            let ray_gen_copy_region = handles_copy.get_region(ray_gen_handle.len() as u64, None);
            let misses_copy_region = handles_copy.get_region(miss_handles.len() as u64, None);
            let hits_copy_region = handles_copy.get_region(hit_handles.len() as u64, None);
            
            let ray_gen_region = handles_buffer.get_region(ray_gen_handle.len() as u64, Some((false, raytracing_props.shader_group_base_alignment.into())));
            let misses_region = handles_buffer.get_region(miss_handles.len() as u64, Some((false, raytracing_props.shader_group_base_alignment.into())));
            let hits_region = handles_buffer.get_region(hit_handles.len() as u64, Some((false, raytracing_props.shader_group_base_alignment.into())));

            cpu_mem.copy_from_ram_slice(&ray_gen_handle, &ray_gen_copy_region);
            cpu_mem.copy_from_ram_slice(&miss_handles, &misses_copy_region);
            cpu_mem.copy_from_ram_slice(&hit_handles, &hits_copy_region);

            ray_gen_copy_region.copy_to_region(cmd, &ray_gen_region);
            misses_copy_region.copy_to_region(cmd, &misses_region);
            hits_copy_region.copy_to_region(cmd, &hits_region);

            let ray_gen_strided = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(unsafe{ray_gen_region.get_device_address().device_address as u64})
            .stride(raytracing_props.shader_group_handle_size as u64)
            .size((raytracing_props.shader_group_handle_size*1) as u64)
            .build();
            let miss_strided = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(unsafe{misses_region.get_device_address().device_address as u64})
            .stride(raytracing_props.shader_group_handle_size as u64)
            .size((raytracing_props.shader_group_handle_size*sbt_outline.misses.len() as u32) as u64)
            .build();
            let hit_strided = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(unsafe{hits_region.get_device_address().device_address as u64})
            .stride(raytracing_props.shader_group_handle_size as u64)
            .size((raytracing_props.shader_group_handle_size*sbt_outline.hit_groups.len() as u32) as u64)
            .build();
            

            (RayTracingPipeline{ 
                device, 
                raytracing_loader, 
                raytracing_props, 
                sbt_outline, 
                layout, 
                pipeline, 
                gpu_mem, 
                shaders_buffer: handles_buffer, 
                shader_regions: (ray_gen_region, misses_region, hits_region),
                shader_addresses: (ray_gen_strided, miss_strided, hit_strided), },
            RayTracingPipelineCreateRecipt{ cpu_mem, cpu_buffer: handles_copy })

        }
        pub fn new_immediate<T: IEngineData>(engine: &T, sbt_outline: SbtOutline, set_layouts: &[vk::DescriptorSetLayout], push_constant_ranges: &[vk::PushConstantRange]) -> RayTracingPipeline{
            let pool = core::CommandPool::new(engine, vk::CommandPoolCreateInfo::builder().queue_family_index(engine.queue_data().graphics.1).build());
            let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];
            unsafe{
                engine.device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().build()).unwrap();
            }
            let data = RayTracingPipeline::new(engine, cmd, sbt_outline, set_layouts, push_constant_ranges);
            unsafe{
                engine.device().end_command_buffer(cmd).unwrap();
            }
            let cmds = [cmd];
            let submit = [vk::SubmitInfo::builder().command_buffers(&cmds).build()];
            let fence = core::sync::Fence::new(engine, false);
            unsafe{
                engine.device().queue_submit(engine.queue_data().graphics.0, &submit, fence.get_fence()).unwrap();
            }
            fence.wait();    
            data.0
        }
        
    }
    impl Drop for RayTracingPipeline{
        fn drop(&mut self) {
        debug!("Destroying ray tracing pipeline {:?}", self.pipeline);
        debug!("Destroying ray tracing pipeline layout {:?}", self.layout);
        unsafe{
            self.device.destroy_pipeline_layout(self.layout, None);
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
    }        
}
    
    pub mod sync{
        use ash::{self, vk};
        use crate::core;
        use crate::traits::IEngineData;
        use log::debug;
        pub struct Fence{
            device: ash::Device,
            fence: ash::vk::Fence,
        }
        impl Fence{
            pub fn new<T: IEngineData>(engine: &T, start_signaled: bool) -> Fence{
                let fence;
                let c_info;
                if start_signaled{
                    c_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED).build();
                }
                else {
                    c_info = vk::FenceCreateInfo::builder().build();
                }

                unsafe{
                    fence = engine.device().create_fence(&c_info, None).expect("Could not create fence");
                }
                Fence{ device: engine.device(), fence }
            }
            pub fn wait(&self){
                unsafe{
                    self.device.wait_for_fences(&[self.fence], true, u64::max_value()).expect("Could not wait on fence");
                }
            }
            pub fn wait_reset(&self){
                self.wait();
                unsafe{
                    self.device.reset_fences(&[self.fence]).expect("Could not reset fence");
                }
            }
            pub fn get_fence(&self) -> vk::Fence{
                self.fence
            }
        }
        impl Drop for Fence{
            fn drop(&mut self) {
                debug!("Destroying fence {:?}", self.fence);
                unsafe{
                    self.device.destroy_fence(self.fence, None);
                }
            }
        }
    
        pub struct Semaphore{
            device: ash::Device,
            pub semaphore: vk::Semaphore,
        }
        impl Semaphore{
            pub fn new<T: IEngineData>(engine: &T) -> Semaphore {
                let device = engine.device();
                let c_info = vk::SemaphoreCreateInfo::builder().build();
                let semaphore = unsafe{device.create_semaphore(&c_info, None).expect("Could not create semaphore")};
                debug!("Created semaphore {:?}", semaphore);

                Semaphore{
                    device,
                    semaphore,
                }
            }
        }
        impl Drop for Semaphore{
            fn drop(&mut self) {
                debug!("Destroying semaphore {:?}", self.semaphore);
                unsafe{self.device.destroy_semaphore(self.semaphore, None)};
    }
        }
    }
    
    pub struct CommandPool{
        device: ash::Device,
        command_pool: ash::vk::CommandPool,
        c_info: ash::vk::CommandPoolCreateInfo
    }
    impl CommandPool{
        pub fn new<T: IEngineData>(engine: &T, c_info: ash::vk::CommandPoolCreateInfo) -> CommandPool {
    
            unsafe {
                let command_pool = engine.device().create_command_pool(&c_info, None).unwrap();
                CommandPool{
                    device: engine.device(),
                    command_pool,
                    c_info
                }
            }
    
        }
    }
    impl Drop for CommandPool {
        fn drop(&mut self) {
            unsafe {
                debug!("Command Pool destroyed");
                self.device.destroy_command_pool(self.command_pool, None);
            }
        }
    }
    impl traits::ICommandPool for CommandPool{
        
        fn get_command_buffers(&self, mut a_info: vk::CommandBufferAllocateInfo) -> Vec<vk::CommandBuffer> {
            a_info.command_pool = self.command_pool;
            unsafe {
                self.device.allocate_command_buffers(&a_info).unwrap()
            }
        }
        fn reset(&self){
            unsafe {
                self.device.reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty()).unwrap();
            }
        }
    }
    pub struct Shader{
        device: ash::Device,
        source: String,
        module: vk::ShaderModule,       
    }
    impl Shader{
        pub fn new<T: IEngineData>(engine: &T, source: String, kind: shaderc::ShaderKind, ep_name: &str, options: Option<&shaderc::CompileOptions>) -> Shader{
            let module: vk::ShaderModule;
            let compiler = shaderc::Compiler::new().unwrap();
            let byte_source = compiler.compile_into_spirv(source.as_str(), kind, "shader.glsl", ep_name, options).unwrap();
            debug!("Compiled shader {} to binary {:?}", source, byte_source.as_binary());
            unsafe{
                let c_info = vk::ShaderModuleCreateInfo::builder().code(byte_source.as_binary()).build();
                module = engine.device().create_shader_module(&c_info, None).unwrap();
            }
            Shader { device: engine.device(), source, module }
        }
        pub fn get_stage(&self, stage: vk::ShaderStageFlags, ep: &CStr) -> vk::PipelineShaderStageCreateInfo{
            vk::PipelineShaderStageCreateInfo::builder()
            .stage(stage)
            .module(self.module)
            .name(ep)
            .build()
        }
    }
    impl Drop for Shader{
        fn drop(&mut self) {
        unsafe{
            debug!("Destroying Shader");
            self.device.destroy_shader_module(self.module, None);
        }
        }
    }

    pub struct ComputePipeline{
        device: ash::Device,
        layout: vk::PipelineLayout,
        pipeline: vk::Pipeline,
        c_info: vk::ComputePipelineCreateInfo,
        push_ranges: Vec<vk::PushConstantRange>,
        descriptor_sets: Vec<vk::DescriptorSetLayout>,
    }
    impl ComputePipeline{
        pub fn new<T: IEngineData>(engine: &T, push_ranges: &[vk::PushConstantRange], descriptor_sets: &[vk::DescriptorSetLayout], shader: vk::PipelineShaderStageCreateInfo) -> ComputePipeline{
            let device = engine.device();
            let lc_info = vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(push_ranges)
            .set_layouts(descriptor_sets)
            .build();
            let layout: vk::PipelineLayout;
            let c_infos: Vec<vk::ComputePipelineCreateInfo>;
            let pipeline: vk::Pipeline;

            unsafe{
                layout = device.create_pipeline_layout(&lc_info, None).unwrap();
                c_infos = vec![vk::ComputePipelineCreateInfo::builder()
                .stage(shader)
                .layout(layout)
                .build()];
                pipeline = device.create_compute_pipelines(vk::PipelineCache::null(), &c_infos, None).unwrap()[0];
            }

            ComputePipeline{ device, layout, pipeline, c_info: c_infos[0], push_ranges: push_ranges.to_vec(), descriptor_sets: descriptor_sets.to_vec() }
        }
        pub fn get_pipeline(&self) -> vk::Pipeline{
            self.pipeline
        }
        pub fn get_layout(&self) -> vk::PipelineLayout{
            self.layout
        }
    }
    impl Drop for ComputePipeline{
        fn drop(&mut self) {
        unsafe{
            debug!("Destroying pipline {:?} with layout {:?}", self.layout, self.pipeline);
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.layout, None);
        }
    }
    }


}

 
pub trait IDisposable {
    fn dispose(&mut self);
}

#[allow(dead_code, unused)]
pub mod init{
    use std::{ffi::CStr, borrow::{Cow, BorrowMut}, os::raw::c_char, f32::consts::E};
    use ash::vk;
    use log::debug;
    use winit::{window::Window, dpi::PhysicalSize};

    use crate::{memory, IDisposable};
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

    pub trait IEngine {
        fn get_instance(&self) -> ash::Instance;
        fn get_physical_device(&self) -> vk::PhysicalDevice;
        fn get_device(&self) -> ash::Device;
        fn get_property_store(&self) -> PhysicalDevicePropertiesStore;
        fn get_queue_store(&self) -> QueueStore;
    }
    pub trait IWindowedEngine {
        fn get_surface_loader(&self) -> ash::extensions::khr::Surface;
        fn get_surface(&self) -> vk::SurfaceKHR;
        fn get_window_size(&self) -> PhysicalSize<u32>;
    }
    pub enum EngineInitOptions<'a>{
        UseValidation(Option<Vec<vk::ValidationFeatureEnableEXT>>, Option<Vec<vk::ValidationFeatureDisableEXT>>),
        UseDebugUtils,
        WindowTitle(&'a str),
        WindowInnerSize(winit::dpi::PhysicalSize<u32>),
        WindowResizable(bool),
        ApplicationName(&'a CStr),
        ApplicationVersion(u32),
        EngineName(&'a CStr),
        EngineVersion(u32),
        ApiVersion(u32),
        InstanceCreateFlags(vk::InstanceCreateFlags),
        QueueCreateFlags(vk::DeviceQueueCreateFlags),
        DeviceExtensions(Vec<*const i8>),
        DeviceFeatures(vk::PhysicalDeviceFeatures),
        DeviceFeatures11(vk::PhysicalDeviceVulkan11Features),
        DeviceFeatures12(vk::PhysicalDeviceVulkan12Features),
        DeviceFeatures13(vk::PhysicalDeviceVulkan13Features),
        DeviceFeaturesRayTracing(vk::PhysicalDeviceRayTracingPipelineFeaturesKHR),
        DeviceFeaturesAccelerationStructure(vk::PhysicalDeviceAccelerationStructureFeaturesKHR),
    }
    pub enum CreateSwapchainOptions<'a>{
        TargetFormat(vk::SurfaceFormatKHR),
        SwapchainExtent(vk::Extent2D),
        OldSwapchain(&'a SwapchainStore),
        ImageUsages(vk::ImageUsageFlags),
    }
    pub struct Engine{
        instance: ash::Instance,
        physical_device: vk::PhysicalDevice,
        device: ash::Device,
        properties_store: PhysicalDevicePropertiesStore,
        queue_store: QueueStore,
        debug_store: Option<DebugStore>
    }
    pub struct WindowedEngine{
        window: Window,
        surface_loader: ash::extensions::khr::Surface,
        surface: vk::SurfaceKHR,
        engine: Engine,
    }
    pub struct SwapchainStore{
        swapchain_loader: ash::extensions::khr::Swapchain,
        swapchain: vk::SwapchainKHR,
        c_info: vk::SwapchainCreateInfoKHR,
        images: Vec<memory::ImageResources>,
        disposed: bool,
    }
    #[derive(Clone)]
    pub struct PhysicalDevicePropertiesStore{
        instance: ash::Instance,
        physical_device: vk::PhysicalDevice,
        pub pd_properties: vk::PhysicalDeviceProperties,
        pub pd_raytracing_properties: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
        pub pd_acc_structure_properties: vk::PhysicalDeviceAccelerationStructurePropertiesKHR,
        pub pd_mem_props: vk::PhysicalDeviceMemoryProperties,
        pub pd_mem_budgets: vk::PhysicalDeviceMemoryBudgetPropertiesEXT
    }
    #[derive(Clone)]
    pub struct QueueStore{
        device: ash::Device,
        family_props: Vec<vk::QueueFamilyProperties>,
        created_families: Vec<usize>,
    }
    #[derive(Clone)]
    pub struct DebugStore{
        debug_loader: ash::extensions::ext::DebugUtils,
        callback: vk::DebugUtilsMessengerEXT,
    }

    impl Engine{
        pub fn init(options: &mut [EngineInitOptions], window: Option<&Window>) -> (Engine, Option<(ash::extensions::khr::Surface, vk::SurfaceKHR)>) {
            let entry = ash::Entry::linked();
            let app_name = unsafe{CStr::from_bytes_with_nul_unchecked(b"VulkanTriangle\0")};


            let mut layer_names = vec![];
            let mut extension_names = vec![]; 
            for option in options.iter(){
                match option{
                    EngineInitOptions::UseValidation(_,_) => {
                        let name = unsafe{CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0")};
                        layer_names.push(name.as_ptr());
                        debug!("Adding Khronos validation layers");
                    },
                    EngineInitOptions::UseDebugUtils => {
                        extension_names.push(ash::extensions::ext::DebugUtils::name().as_ptr())
                    }
                    _ => {}
                }
            }
            match window {
                Some(w) => {
                    let names = ash_window::enumerate_required_extensions(w)
                        .expect("Could not get required window extensions")
                        .to_vec();
                    extension_names.extend_from_slice(&names);
                    debug!("Adding neccesary window extensions");
                },
                None => {},
            }

            let mut app_info = vk::ApplicationInfo::builder()
                    .application_name(app_name)
                    .application_version(0)
                    .engine_name(app_name)
                    .engine_version(0)
                    .api_version(vk::API_VERSION_1_3);
            let mut validation_features = vk::ValidationFeaturesEXT::builder();
            let mut instance_c_info = vk::InstanceCreateInfo::builder()
            .enabled_layer_names(&layer_names)
            .enabled_extension_names(&extension_names);
            for option in options.iter(){
                match option{
                    EngineInitOptions::UseValidation(enables, disables) => {
                        match enables{
                            Some(features) => {
                                debug!("Enabling extra validation features");
                                validation_features = validation_features.enabled_validation_features(&features)},
                            None => {},
                        }
                        match disables{
                            Some(features) => {
                                debug!("Disabling some default validation features");
                                validation_features = validation_features.disabled_validation_features(&features);
                            },
                            None => {},
                        }
                    },
                    EngineInitOptions::ApplicationName(s) => {
                        app_info = app_info.application_name(s);
                        debug!("None standard app name specified");
                    },
                    EngineInitOptions::ApplicationVersion(v) => {
                        app_info = app_info.application_version(*v);
                        debug!("None standard app version specified");
                    },
                    EngineInitOptions::EngineName(s) => {
                        app_info = app_info.engine_name(s);
                        debug!("None standard engine name specified");
                    },
                    EngineInitOptions::EngineVersion(v) => {
                        app_info = app_info.engine_version(*v);
                        debug!("None standard engine version specified");
                    },
                    EngineInitOptions::ApiVersion(v) => {
                        app_info = app_info.api_version(*v);
                        debug!("None standard api version specified");
                    },
                    EngineInitOptions::InstanceCreateFlags(f) => {instance_c_info = instance_c_info.flags(*f);},
                    _ => {},
                }
            }
            instance_c_info = instance_c_info.application_info(&app_info);
            instance_c_info = instance_c_info.push_next(&mut validation_features);
            let instance = unsafe {entry.create_instance(&instance_c_info, None).expect("Could not create instance")};
            debug!("Created instance {:?}", instance.handle());

            let mut debug_store = None;
            match options.iter().find(|option| {
                let ret = match option{
                    EngineInitOptions::UseDebugUtils => true,
                    _ => {false}
                };
                ret
            })
            {
                Some(d) => {
                    let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                    )
                    .pfn_user_callback(Some(vulkan_debug_callback));

                let debug_utils_loader = ash::extensions::ext::DebugUtils::new(&entry, &instance);
                let debug_call_back = unsafe{debug_utils_loader
                    .create_debug_utils_messenger(&debug_info, None)
                    .unwrap()};
                
                debug_store = Some(DebugStore{ debug_loader: debug_utils_loader, callback: debug_call_back });
                debug!("Created debug utils callback");
                },
                None => {},
            }
            let mut surface_support = match window {
                Some(w) => {
                    let surface = unsafe{ash_window::create_surface(&entry, &instance, w, None).expect("Could not create surface")};
                    let loader = ash::extensions::khr::Surface::new(&entry, &instance);
                    debug!("Created surface {:?}", surface);
                    Some((loader, surface))
                },
                None => None,
            };

            let physical_device = QueueStore::choose_physical_device(&instance, &surface_support);
            let queue_infos = QueueStore::get_queue_infos(&instance, &physical_device, options);
            let mut device_info = vk::DeviceCreateInfo::builder().queue_create_infos(&queue_infos.1);
            let mut features = vk::PhysicalDeviceFeatures2::builder();
            for option in options.iter_mut(){
                match option {
                    EngineInitOptions::DeviceExtensions(ext) => {
                        debug!("Adding device extensions");
                        device_info = device_info.enabled_extension_names(ext)
                    },
                    EngineInitOptions::DeviceFeatures(f) => {
                        debug!("Adding device features");
                        features = features.features(*f)
                    },
                    EngineInitOptions::DeviceFeatures12(f) => {
                        debug!("Adding device vulkan 12 features");
                        features = features.push_next(f)
                    },
                    EngineInitOptions::DeviceFeatures11(f) => {
                        debug!("Adding device vulkan 11 features");
                        features = features.push_next(f)
                    },
                    EngineInitOptions::DeviceFeatures13(f) => {
                        debug!("Adding device vulkan 13 features");
                        features = features.push_next(f)
                    },
                    EngineInitOptions::DeviceFeaturesRayTracing(f) => {
                        debug!("Adding device ray tracing features");
                        features = features.push_next(f)
                    },
                    EngineInitOptions::DeviceFeaturesAccelerationStructure(f) => {
                        debug!("Adding device acceleration structure features");
                        features = features.push_next(f)
                    },
                    _ => {}
                }
            }
            device_info = device_info.push_next(&mut features);

            let device = unsafe{instance.create_device(physical_device, &device_info, None).expect("Could not create logical device")};
            debug!("Created logical device {:?}", device.handle());

            let queue_store = QueueStore::new(&instance, &physical_device, &device, &queue_infos.1);
            let props = PhysicalDevicePropertiesStore::new(&instance, &physical_device);
            (Engine{instance, physical_device, device, queue_store, debug_store, properties_store: props  }, surface_support)
        }
    }
    impl IEngine for Engine{
        fn get_instance(&self) -> ash::Instance {
        self.instance.clone()
    }

        fn get_physical_device(&self) -> vk::PhysicalDevice {
        self.physical_device.clone()
    }

        fn get_device(&self) -> ash::Device {
        self.device.clone()
    }

        fn get_property_store(&self) -> PhysicalDevicePropertiesStore {
        self.properties_store.clone()
    }

        fn get_queue_store(&self) -> QueueStore {
        self.queue_store.clone()
    }
    }
    impl Drop for Engine{
        fn drop(&mut self) {
            unsafe{
                match &self.debug_store {
                    Some(store) => {
                        debug!("Destroying debug callback {:?}", store.callback);
                        store.debug_loader.destroy_debug_utils_messenger(store.callback, None);
                    },
                    None => {},
                }
                debug!("Destroying device {:?}", self.device.handle());
                debug!("Destroying instance {:?}", self.instance.handle());
                self.device.destroy_device(None);
                self.instance.destroy_instance(None);
            }
    }
    }
    impl WindowedEngine{
        pub fn init(options: &mut [EngineInitOptions]) -> (winit::event_loop::EventLoop<()>, WindowedEngine) {

            let event_loop = winit::event_loop::EventLoop::new();
            let mut window = winit::window::WindowBuilder::new()
                .with_title("Ray tracer!")
                .with_inner_size(PhysicalSize::new(200 as u32,200 as u32));
            for option in options.iter_mut(){
                match option{
                    EngineInitOptions::WindowTitle(s) => window = window.with_title(*s),
                    EngineInitOptions::WindowInnerSize(s) => window = window.with_inner_size(*s),
                    EngineInitOptions::WindowResizable(b) =>  window = window.with_resizable(*b),
                    EngineInitOptions::DeviceExtensions(ext) => {
                        ext.push(ash::extensions::khr::Swapchain::name().as_ptr());
                    },
                    _ => {}
                }
            }
            
            let window = window.build(&event_loop).expect("Could not create window");
                

            let mut surface = vk::SurfaceKHR::null();

            let (engine, surface_data) = Engine::init(options, Some(&window));

            drop(options);

            let surface_data = surface_data.expect("No surface data found");

            (event_loop, WindowedEngine{ surface_loader: surface_data.0, surface: surface_data.1, engine, window })
        }
    }
    impl IEngine for WindowedEngine{
        fn get_instance(&self) -> ash::Instance {
        self.engine.get_instance()
    }

        fn get_physical_device(&self) -> vk::PhysicalDevice {
        self.engine.get_physical_device()
    }

        fn get_device(&self) -> ash::Device {
        self.engine.get_device()
    }

        fn get_property_store(&self) -> PhysicalDevicePropertiesStore {
        self.engine.get_property_store()
    }

        fn get_queue_store(&self) -> QueueStore {
        self.engine.get_queue_store()
    }
    }
    impl IWindowedEngine for WindowedEngine{
        fn get_surface_loader(&self) -> ash::extensions::khr::Surface {
        self.surface_loader.clone()
    }

        fn get_surface(&self) -> vk::SurfaceKHR {
        self.surface.clone()
    }

        fn get_window_size(&self) -> PhysicalSize<u32> {
        self.window.inner_size()
    }
    }
    impl Drop for WindowedEngine{
        fn drop(&mut self) {
            unsafe{
                debug!("Destroying surface {:?}", self.surface);
                self.surface_loader.destroy_surface(self.surface, None);
            }
    }
    }
    impl QueueStore{
        fn choose_physical_device(instance: &ash::Instance, surface_support: &Option<(ash::extensions::khr::Surface, vk::SurfaceKHR)>) -> vk::PhysicalDevice {
            let physical_devices = unsafe{instance.enumerate_physical_devices().expect("Could not get physical devices")};
            let mut device_properties = vk::PhysicalDeviceProperties::default();
            let chosen_device = physical_devices.iter().find(|&dev|{
                let queue_family_properties = unsafe{instance.get_physical_device_queue_family_properties(*dev)};
                device_properties = unsafe{instance.get_physical_device_properties(*dev)};
                let mut has_graphics = false;
                let mut has_transfer = false;
                let mut has_compute = false;
                let mut has_surface = false;

                for (index, fam) in queue_family_properties.iter().enumerate(){
                    if !has_graphics {
                        has_graphics = fam.queue_flags.contains(vk::QueueFlags::GRAPHICS);
                    }
                    if !has_transfer {
                        has_transfer = fam.queue_flags.contains(vk::QueueFlags::TRANSFER);
                    }
                    if !has_compute {
                        has_compute = fam.queue_flags.contains(vk::QueueFlags::COMPUTE);
                    }
                    match &surface_support {
                        Some((l, s)) => {
                            if !has_surface {
                                has_surface = unsafe{l.get_physical_device_surface_support(*dev, index as u32, *s).expect("Could not get physical device surface support")};
                            }
                        },
                        None => {},
                    }
                }

                let capable = match &surface_support {
                    Some(_) => {
                        //device_properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU &&
                        has_graphics &&
                        has_transfer &&
                        has_compute &&
                        has_surface
                    },
                    None => {
                        //device_properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU &&
                        has_graphics &&
                        has_transfer &&
                        has_compute},
                };
                capable
            });

            let mut physical_device = vk::PhysicalDevice::null();
            match chosen_device {
                Some(dev) => {
                    match surface_support {
                        Some(_) => {
                            debug!("Discrete Device {:?} has graphics, transfer, compute, and surface support", String::from_utf8(device_properties.device_name.iter().map(|&c| c as u8).collect()).unwrap().replace("\0", ""));
                            physical_device = *dev;
                        },
                        None => {
                            debug!("Discrete Device {:?} has graphics, transfer, and compute support", String::from_utf8(device_properties.device_name.iter().map(|&c| c as u8).collect()).unwrap().replace("\0", ""));
                            physical_device = *dev;
                        },
                    }
                },
                None => panic!("No physical devices meet all queue requirements and are discrete"),
            }

            physical_device

        }
        fn get_queue_infos(instance: &ash::Instance, physical_device: &vk::PhysicalDevice, options: &[EngineInitOptions]) -> (Vec<f32>, Vec<vk::DeviceQueueCreateInfo>) {

            let priorites = vec![1.0];

            let create_flags = match options.iter().find(|option| match option {
                EngineInitOptions::QueueCreateFlags(_) => {
                    debug!("Using non-default queue create flags");
                    true
                },
                _ => {false}
            }) {
                Some(option) => {
                    match option {
                        EngineInitOptions::QueueCreateFlags(flags) => Some(*flags),
                        _ => {panic!("What?")}
                    }
                },
                None => None,
            };

            let queue_create_infos:Vec<vk::DeviceQueueCreateInfo> = unsafe{instance.get_physical_device_queue_family_properties(*physical_device)}.iter().enumerate().filter(|(index,props)|{
                props.queue_flags.contains(vk::QueueFlags::GRAPHICS) || props.queue_flags.contains(vk::QueueFlags::TRANSFER) || props.queue_flags.contains(vk::QueueFlags::COMPUTE)
            }).map(|(index, q_props)| {
                let mut builder = vk::DeviceQueueCreateInfo::builder();
                match create_flags {
                    Some(flags) => builder = builder.flags(flags),
                    None => {},
                }
                builder = builder.queue_family_index(index as u32);
                builder = builder.queue_priorities(&priorites);
                builder.build()
            }).collect();

            let indecies:Vec<u32> = queue_create_infos.iter().map(|infos| infos.queue_family_index).collect();
            debug!{"Creating queues from families {:?}", indecies};
            (priorites, queue_create_infos)


        }
        fn new(instance: &ash::Instance, physical_device: &vk::PhysicalDevice, device: &ash::Device, queue_create_infos: &[vk::DeviceQueueCreateInfo]) -> QueueStore {
            let queue_infos = unsafe{instance.get_physical_device_queue_family_properties(*physical_device)};
            QueueStore{ device: device.clone(), family_props: queue_infos, created_families: queue_create_infos.iter().map(|info| info.queue_family_index as usize).collect() }
        }
        pub fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
            let mut best_score = u32::MAX;
            let mut target_queue = None;
            for family in self.created_families.iter(){
                let props = &self.family_props[*family];
                if props.queue_flags.contains(target_flags) {
                    let mut local_score = 0;
                    if props.queue_flags.contains(vk::QueueFlags::GRAPHICS){
                        local_score += 1;
                    }
                    if props.queue_flags.contains(vk::QueueFlags::TRANSFER){
                        local_score += 1;
                    }
                    if props.queue_flags.contains(vk::QueueFlags::COMPUTE){
                        local_score += 1;
                    }
                    if local_score < best_score{
                        best_score = local_score;
                        let queue = unsafe{self.device.get_device_queue((*family) as u32, 0)};
                        target_queue = Some((queue, (*family) as u32));
                    }
                }
            }
            target_queue
        }
    }
    impl PhysicalDevicePropertiesStore{
        fn new(instance: &ash::Instance, physical_device: &vk::PhysicalDevice) -> PhysicalDevicePropertiesStore {
            let mut ray_props = vk::PhysicalDeviceRayTracingPipelinePropertiesKHR::builder().build();
            let mut acc_props = vk::PhysicalDeviceAccelerationStructurePropertiesKHR::builder().build();
            let mut properties2 = vk::PhysicalDeviceProperties2::builder()
            .push_next(&mut ray_props)
            .push_next(&mut acc_props)
            .build();
            

            
            let mut memory_budgets = vk::PhysicalDeviceMemoryBudgetPropertiesEXT::builder().build();
            let mut memory_properties = vk::PhysicalDeviceMemoryProperties2::builder()
            .push_next(&mut memory_budgets)
            .build();

            unsafe{
                instance.get_physical_device_properties2(*physical_device, &mut properties2);
                instance.get_physical_device_memory_properties2(*physical_device, &mut memory_properties);
            }

            PhysicalDevicePropertiesStore{ 
                instance: instance.clone(), 
                physical_device: physical_device.clone(), 
                pd_properties: properties2.properties, 
                pd_raytracing_properties: ray_props, 
                pd_acc_structure_properties: acc_props, 
                pd_mem_props: memory_properties.memory_properties, 
                pd_mem_budgets: memory_budgets }

        }
        pub fn get_memory_index(&self, properties: vk::MemoryPropertyFlags) -> u32 {
            let mut selected_type: usize = 0;
                    //Selecting the corrent memory type
                    for type_index in 0..self.pd_mem_props.memory_types.len(){
                        let mem_type = &self.pd_mem_props.memory_types[type_index];
                        let heap = &self.pd_mem_props.memory_heaps[mem_type.heap_index as usize];
                        if mem_type.property_flags & properties != vk::MemoryPropertyFlags::empty() {
                            //debug!("Found compatible memory");
                            //debug!("Type index: {}, Type property: {:?}, Type heap: {}", type_index, self.pd_mem_props.memory_types[type_index].property_flags, self.pd_mem_props.memory_types[type_index].heap_index);
                            if self.pd_mem_props.memory_types[selected_type].property_flags & properties != vk::MemoryPropertyFlags::empty() {
                                if heap.size > self.pd_mem_props.memory_heaps[self.pd_mem_props.memory_types[selected_type].heap_index as usize].size && type_index != selected_type{
                                    //debug!("  Selecting Memory Type");
                                    selected_type = type_index;
                                }
                            }
                            else {
                                //debug!("Previously selected memory is of wrong type, selecting current memory type");
                                selected_type = type_index;
                            }
                        }
                    }
            selected_type as u32
        }
        pub fn get_image_format_properties(&self, format: vk::Format, typ: vk::ImageType, tiling: vk::ImageTiling, usage: vk::ImageUsageFlags, flags: vk::ImageCreateFlags) -> vk::ImageFormatProperties  {
            unsafe{
                let mut format_props = self.instance.get_physical_device_format_properties(self.physical_device, format);
                debug!("{:?}", format_props);
                self.instance.get_physical_device_image_format_properties(self.physical_device, format, typ, tiling, usage, flags).expect("Could not get format properties")
            }
        }
        pub fn refresh_memory_budgets(&mut self){
            let mut memory_budgets = vk::PhysicalDeviceMemoryBudgetPropertiesEXT::builder().build();
            let mut memory_properties = vk::PhysicalDeviceMemoryProperties2::builder()
            .push_next(&mut memory_budgets)
            .build();
            unsafe{
                self.instance.get_physical_device_memory_properties2(self.physical_device, &mut memory_properties);
            }
            self.pd_mem_budgets = memory_budgets;
        }
        pub fn get_memory_budgets(&self) -> vk::PhysicalDeviceMemoryBudgetPropertiesEXT {
            self.pd_mem_budgets
        }
    }
    impl SwapchainStore{
        pub fn new<T:IEngine + IWindowedEngine>(engine: &T, options: &[CreateSwapchainOptions]) -> SwapchainStore {
            
            let surface_loader = engine.get_surface_loader();
            let surface = engine.get_surface();
            let physical_device = engine.get_physical_device();

            let possible_formats = unsafe{surface_loader.get_physical_device_surface_formats(physical_device, surface).expect("Could not get surface formats")};
            let chosen_format = match options.iter().find(|option| {
                let res = match option {
                    CreateSwapchainOptions::TargetFormat(f) => {true},
                    _ => {false}
                };
                res
            }){
                Some(option) => {
                    let (f,has_format) = match option {
                        CreateSwapchainOptions::TargetFormat(f) => {(*f,possible_formats.contains(f))},
                        _ => {panic!("What?")}
                    };
                    if has_format {
                        debug!("Using target format of {:?}", f);
                        Some(f)
                    }
                    else {
                        None
                    }
                },
                None => {
                    //debug!("Using first available format of {:?}", possible_formats[0]);
                    Some(possible_formats[0])
                },
            }.expect("Could not find a suitable format");

            let surface_capabilties = unsafe{surface_loader.get_physical_device_surface_capabilities(physical_device, surface).expect("Could not get surface capabilities")};
            let mut desired_image_count = surface_capabilties.min_image_count + 1;
            if surface_capabilties.max_image_count > 0
                && desired_image_count > surface_capabilties.max_image_count
            {
                desired_image_count = surface_capabilties.max_image_count;
            }

            let window_size = engine.get_window_size();
            let surface_resolution = vk::Extent2D::builder().width(window_size.width).height(window_size.height).build();

            let pre_transform = if surface_capabilties
                .supported_transforms
                .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
            {
                vk::SurfaceTransformFlagsKHR::IDENTITY
            } else {
                surface_capabilties.current_transform
            };

            let present_modes = unsafe{surface_loader
                    .get_physical_device_surface_present_modes(physical_device, surface)
                    .expect("Could not get present modes")};
            let present_mode = present_modes
                .iter()
                .cloned()
                .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
                .unwrap_or(vk::PresentModeKHR::FIFO);


            let mut swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface)
            .min_image_count(desired_image_count)
            .image_color_space(chosen_format.color_space)
            .image_format(chosen_format.format)
            .image_extent(surface_resolution)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(pre_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

            for option in options.iter().filter(|option| {
                let res = match option {
                    CreateSwapchainOptions::OldSwapchain(_) => {true},
                    CreateSwapchainOptions::ImageUsages(_) => {true},
                    _ => {false}
                };
                res
            }){
                match option {
                    CreateSwapchainOptions::OldSwapchain(s) => {
                        debug!("Creating new swapchain from old swapchain {:?}", s.swapchain);
                        swapchain_create_info = swapchain_create_info.old_swapchain(s.swapchain);
                    },
                    CreateSwapchainOptions::ImageUsages(u) => {
                        debug!("Adding non default image usage flags to swapchain");
                        swapchain_create_info = swapchain_create_info.image_usage(*u);
                    }
                    _ => {panic!("What?")}
                };
            }

            let swapchain_loader = ash::extensions::khr::Swapchain::new(&engine.get_instance(), &engine.get_device());
            let swapchain  = unsafe{swapchain_loader.create_swapchain(&swapchain_create_info, None).expect("Could not create swapchain")};
            debug!("Created swapchain {:?} of resolution {} x {}",swapchain, surface_resolution.width, surface_resolution.height);
            let images = unsafe{swapchain_loader.get_swapchain_images(swapchain).expect("Could not retrieve swapchain images")};

            let images = images.iter().map(|i| {
                memory::ImageResources::new_from_image(
                    engine, 
                    i.clone(), 
                    vk::ImageLayout::UNDEFINED, 
                    surface_resolution.into(), 
                    vk::ImageAspectFlags::COLOR,
                    0, 
                    1, 
                    0, 
                    1, 
                    vk::ImageViewType::TYPE_2D, 
                    chosen_format.format, 
                    &[])
                    
            }).collect();

            SwapchainStore{ swapchain_loader, swapchain, c_info: swapchain_create_info.build(), images, disposed: false }
        }
        pub fn get_extent(&self) -> vk::Extent2D {
            self.c_info.image_extent
        }
        pub fn get_image(&self, index: usize) -> memory::ImageResources {
            self.images[index].clone()
        }
        pub fn get_next_image(&mut self, timeout: u64, semaphore: Option<vk::Semaphore>, fence: Option<vk::Fence> ) -> (u32, &mut memory::ImageResources) {

            let semaphore = match semaphore {
                Some(s) => s,
                None => vk::Semaphore::null(),
            };
            let fence = match fence {
                Some(f) => f,
                None => vk::Fence::null(),
            };

            let next = unsafe{self.swapchain_loader.acquire_next_image(self.swapchain, timeout, semaphore, fence).expect("Could not get next swapchain image index")};
            
            (next.0, &mut self.images[next.0 as usize])
        
        }
        pub fn get_format(&self) -> vk::Format {
            self.c_info.image_format
        }
        pub fn get_swapchain(&self) -> vk::SwapchainKHR {
            self.swapchain
        }
        pub fn present(&self, queue: vk::Queue, index: u32, wait_semaphores: &[vk::Semaphore]){
            let index = [index];
            let swapchain = [self.swapchain];
            
            let present_info = vk::PresentInfoKHR::builder()
            .image_indices(&index)
            .swapchains(&swapchain)
            .wait_semaphores(wait_semaphores);

            unsafe{
                self.swapchain_loader.queue_present(queue, &present_info).expect("Could not present swapchain");
            }
        }
    }
    impl IDisposable for SwapchainStore{
        fn dispose(&mut self) {
            if !self.disposed{
                self.disposed = true;
                debug!("Destroying swapchain {:?}", self.swapchain);
                unsafe{
                    self.swapchain_loader.destroy_swapchain(self.swapchain, None);
                }
            }
        }
    }
    impl Drop for SwapchainStore{
        fn drop(&mut self) {
            self.dispose();
        }
    }
}
#[allow(dead_code, unused)]
pub mod memory{
    use std::{sync::Arc, mem::size_of, collections::HashMap};
    use ash::{self, vk};
    use log::debug;
    use crate::{init::{self,IEngine,PhysicalDevicePropertiesStore, Engine}, IDisposable, descriptor::DescriptorWriteType};

    #[derive(Clone)]
    pub enum AlignmentType{
        Free,
        Allocation(u64),
        User(u64),
    }
    #[derive(Clone)]
    pub enum CreateAllocationOptions{
        MemoryAllocateFlags(vk::MemoryAllocateFlagsInfo),
        MinimumSize(u64),
        SizeOverkillFactor(u64),
    }
    #[derive(Clone)]
    pub enum CreateBufferOptions{
        BufferCreateFlags(vk::BufferCreateFlags),
        SizeOverkillFactor(u64),
        MinimumSize(u64),
    }
    #[derive(Clone)]
    pub enum CreateImageOptions{
        ImageCreateFlags(vk::ImageCreateFlags),
        ImageType(vk::ImageType),
        MipLevels(u32),
        ArrayLevels(u32),
        Samples(vk::SampleCountFlags),
        Tiling(vk::ImageTiling),
        MultiQueueUse(Vec<u32>),
        InitalLayout(vk::ImageLayout),
    }
    pub enum CreateBufferRegionOptions {
        
    }
    #[derive(Debug)]
    pub enum MemoryBlockError{
        NoSpace,
    }
    pub enum CreateImageResourceOptions{
        Swizzle(vk::ComponentMapping),
        Flags(vk::ImageViewCreateFlags),
        Layout(vk::ImageLayout),
    }
    enum MemoryBlockArray{
        Allocation(Vec<AllocationMemoryBlock>),
        Buffer(Vec<BufferMemoryBlock>),
    }
    #[derive(Clone)]
    pub enum AllocatorProfileType{
            Allocation(AllocationAllocatorProfile),
            Buffer(BufferAllocatorProfile),
            Image(ImageAllocatorProfile),
        }
    pub enum AllocatorResourceType{
            Allocation(Allocation),
            Buffer(Buffer),
            Image(Image),
        }
    #[derive(PartialEq, Eq, Hash, Clone)]
    pub enum AllocatorProfileStack{
        TargetBuffer(usize, usize),
        TargetImage(usize, usize),
    }
        
    
    #[derive(Clone)]
    struct AllocationMemoryBlock{
        start_offset: u64,
        size: u64,
        user: Arc<bool>,
    }
    #[derive(Clone)]
    struct BufferMemoryBlock{
        allocation_offset: u64,
        buffer_offset: u64,
        size: u64,
        user: Arc<bool>,
    }
    pub struct Allocation{
        device: ash::Device,
        properties: PhysicalDevicePropertiesStore,
        memory_type: vk::MemoryPropertyFlags,
        allocation: vk::DeviceMemory,
        c_info: vk::MemoryAllocateInfo,
        blocks: MemoryBlockArray,
        disposed: bool,
        allocation_resource_index: usize,
    }
    pub struct Buffer{
        device: ash::Device,
        properties: PhysicalDevicePropertiesStore,
        buffer: vk::Buffer,
        mem_reqs: vk::MemoryRequirements,
        c_info: vk::BufferCreateInfo,
        //memory type, memory type index, home_allocation_block
        memory_info: Option<(vk::MemoryPropertyFlags, u32, AllocationMemoryBlock, MemoryBlockArray)>,
        disposed: bool,
        buffer_resource_index: usize,
        allocation_resource_index: usize
    }
    pub struct BufferRegion{
        device: ash::Device,
        properties: PhysicalDevicePropertiesStore,
        buffer: vk::Buffer,
        memory_type: vk::MemoryPropertyFlags,
        memory_index: u32,
        buffer_usage: vk::BufferUsageFlags,
        home_block: BufferMemoryBlock,
        blocks: MemoryBlockArray,
        buffer_resource_index: usize,
        allocation_resource_index: usize,
    }
    pub struct Image{
        device: ash::Device,
        properties: PhysicalDevicePropertiesStore,
        image: vk::Image,
        mem_reqs: vk::MemoryRequirements,
        c_info: vk::ImageCreateInfo,
        //memory type, memory type index, home_allocation_block
        memory_info: Option<(vk::MemoryPropertyFlags, u32, AllocationMemoryBlock)>,
        disposed: bool,
        allocation_resource_index: usize,
        image_resource_index: usize,
    }
    #[derive(Clone)]
    pub struct ImageResources{
        device: ash::Device,
        properties: PhysicalDevicePropertiesStore,
        image: vk::Image,
        layout: vk::ImageLayout,
        view: vk::ImageView,
        c_info: vk::ImageViewCreateInfo,
        memory_type: vk::MemoryPropertyFlags,
        memory_index: u32,
        target_offset: vk::Offset3D,
        target_extent: vk::Extent3D,
        target_layers: vk::ImageSubresourceLayers,
        disposed: bool,
        allocation_resource_index: usize,
        image_resource_index: usize,
    }
    #[derive(Clone)]
    pub struct AllocationAllocatorProfile{
        dependant_allocations: Vec<usize>,
        memory_properties: vk::MemoryPropertyFlags,
        options: Vec<CreateAllocationOptions>,
    }
    #[derive(Clone)]
    pub struct BufferAllocatorProfile{
        dependant_buffers: Vec<usize>,
        usage: vk::BufferUsageFlags,
        options: Vec<CreateBufferOptions>,
    }
    #[derive(Clone)]
    pub struct ImageAllocatorProfile{
        dependant_images: Vec<usize>,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
        extent: vk::Extent3D,
        options: Vec<CreateImageOptions>,
    }
    pub struct Allocator{
        device: ash::Device,
        properties: PhysicalDevicePropertiesStore,
        settings: Vec<AllocatorProfileType>,
        resources: Vec<AllocatorResourceType>,
        profile_mapping: HashMap<AllocatorProfileStack, (Vec<usize>, Vec<usize>, Vec<usize>)>,
        channel: (flume::Sender<AllocatorProfileStack>, flume::Receiver<AllocatorProfileStack>),
    }
    

    impl Allocator{
        pub fn new<T:IEngine>(engine: &T) -> Allocator {
            let device = engine.get_device();
            let mut properties = engine.get_property_store();
            Allocator{ device, 
                properties, 
                settings: vec![],
                resources: vec![],
                channel: flume::unbounded(),
                profile_mapping: HashMap::new(),
            }
        }
        pub fn add_profile(&mut self, profile: AllocatorProfileType) -> usize {
            self.settings.push(profile);
            self.settings.len()-1
        }
        pub fn update_image(&mut self, image_profile_index: usize, new_profile: &ImageAllocatorProfile){
            match &mut self.settings[image_profile_index] {
                AllocatorProfileType::Allocation(_) => panic!("Profile index mismatch"),
                AllocatorProfileType::Buffer(_) => panic!("Profile index mismatch"),
                AllocatorProfileType::Image(i) => {
                    for image in i.dependant_images.iter(){
                        match &mut self.resources[*image]{
                            AllocatorResourceType::Allocation(_) => panic!("Resource index mismatch"),
                            AllocatorResourceType::Buffer(_) => panic!("Resource index mismatch"),
                            AllocatorResourceType::Image(i) => {
                                i.dispose();

                            },
                        }
                    }

                    for (_,_,images) in self.profile_mapping.values_mut(){
                        *images = images.iter().filter(|index| !i.dependant_images.contains(index)).map(|index| *index).collect();
                    }

                    *i = new_profile.clone();
                },
            }
        }
        pub fn create_allocation<O>(&self, properties: vk::MemoryPropertyFlags, object_count: usize, options: &mut [CreateAllocationOptions]) -> Allocation {
            let type_index = self.properties.get_memory_index(properties);
            let size = size_of::<O>() * object_count;

            let mut c_info = vk::MemoryAllocateInfo::builder()
            .memory_type_index(type_index)
            .allocation_size(size as u64);
            for option in options.iter_mut(){
                c_info = option.add_options(c_info);
            }
            let allocation = unsafe{self.device.allocate_memory(&c_info, None).expect("Could not allocate memory")};
            debug!("Created memory {:?} of size {} on type {}", allocation, c_info.allocation_size, type_index);

            let default_block = AllocationMemoryBlock{ 
                start_offset: 0, 
                size: c_info.allocation_size, 
                user: Arc::new(true) 
            };

            Allocation{ 
                device: self.device.clone(), 
                properties: self.properties.clone(), 
                memory_type: properties, 
                allocation, 
                c_info: c_info.build(), 
                blocks: MemoryBlockArray::Allocation(vec![default_block]),
                disposed: false,
                allocation_resource_index: 0,
                }
        
        }
        pub fn create_buffer<O>(&self, usage: vk::BufferUsageFlags, object_count: usize, options: &[CreateBufferOptions]) -> Buffer {
            let buffer_size = size_of::<O>() * object_count;

            let mut c_info = vk::BufferCreateInfo::builder()
            .size(buffer_size as u64)
            .usage(usage);

            for option in options.iter(){
                c_info = option.add_options(c_info);
            }

            let buffer = unsafe{self.device.create_buffer(&c_info, None).expect("Could not create buffer")};
            let mem_reqs = unsafe{self.device.get_buffer_memory_requirements(buffer)};
            debug!("Created buffer {:?}", buffer);
            
            Buffer{ 
                device: self.device.clone(), 
                properties: self.properties.clone(), 
                buffer, 
                mem_reqs, 
                c_info: c_info.build(), 
                memory_info: None, 
                disposed: false, 
                buffer_resource_index: 0,
                allocation_resource_index: 0, }
        }
        pub fn create_image(&self, usage: vk::ImageUsageFlags, format: vk::Format, extent: vk::Extent3D, options: &[CreateImageOptions]) -> Image {
            let mut c_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::OPTIMAL)
            .usage(usage)
            .initial_layout(vk::ImageLayout::UNDEFINED);

            for option in options {
                match option {
                    CreateImageOptions::ImageCreateFlags(f) => {
                        debug!("Using non-standard image create flags");
                        c_info = c_info.flags(*f);
                    },
                    CreateImageOptions::ImageType(t) => {
                        debug!("Using non-standard image type");
                        c_info = c_info.image_type(*t);
                    },
                    CreateImageOptions::MipLevels(m) => {
                        debug!("Using non-standard mim-levels");
                        c_info = c_info.mip_levels(*m);
                    },
                    CreateImageOptions::ArrayLevels(a) => {
                        debug!("Using non-standard array layers");
                        c_info = c_info.array_layers(*a);
                    },
                    CreateImageOptions::Samples(s) => {
                        debug!("Using non-standard samples count");
                        c_info = c_info.samples(*s);
                    },
                    CreateImageOptions::Tiling(t) => {
                        debug!("Using non-standard image tiling");
                        c_info = c_info.tiling(*t);
                    },
                    CreateImageOptions::MultiQueueUse(q) => {
                        debug!("Using non-standard multi queue use");
                        c_info = c_info.sharing_mode(vk::SharingMode::CONCURRENT);
                        c_info = c_info.queue_family_indices(q);
                    },
                    CreateImageOptions::InitalLayout(l) => {
                        debug!("Using non-standard initial layout");
                        c_info = c_info.initial_layout(*l);
                    },
                    _ => {}
                }
            }

            let image = unsafe{self.device.create_image(&c_info, None).expect("Could not create image")};
            let mem_reqs = unsafe{self.device.get_image_memory_requirements(image)};
            debug!("Created image {:?}", image);

            Image{ 
                device: self.device.clone(), 
                properties: self.properties.clone(), 
                image, 
                mem_reqs, 
                c_info: c_info.build(), 
                memory_info: None, 
                disposed: false, 
                allocation_resource_index: 0, 
                image_resource_index: 0 }

        }
        pub fn get_buffer_region<O>(&mut self, profile: &AllocatorProfileStack, object_count: usize, alignment: &AlignmentType, options: &[CreateBufferRegionOptions]) -> BufferRegion{
            let mut region = None;
            
            match profile{
                AllocatorProfileStack::TargetBuffer(ai, bi) => {
                    //First we need to find and seperate the resources intersection
                    //Then we need to test all buffers or images
                    for buffer in self.get_mapped_buffers(profile){
                        let buffer = &mut self.resources[buffer];
                        match buffer {
                            AllocatorResourceType::Allocation(_) => panic!("Wrong intersection"),
                            AllocatorResourceType::Buffer(b) => {
                                match b.get_region::<O>(object_count, alignment, options){
                                    Ok(mut r) => {
                                        region = Some(r);
                                        break;
                                    },
                                    Err(e) => {
                                        match e {
                                            MemoryBlockError::NoSpace => {},
                                        }
                                    },
                                }
                            },
                            AllocatorResourceType::Image(_) => panic!("Wrong intersection"),
                        }
                    }
                    //If no buffers have space we need to search all allocations for additional buffer
                    
                    match region {
                        Some(_) => {},
                        None => {

                            let (usage, b_options) = match &self.settings[*bi]{
                                AllocatorProfileType::Allocation(_) => panic!("Profile index mismatch"),
                                AllocatorProfileType::Buffer(b) => b.get_settings(),
                                AllocatorProfileType::Image(_) => panic!("Profile index mismatch"),
                            };
                            let mut buffer = self.create_buffer::<O>(usage, object_count, b_options);
                            buffer.buffer_resource_index = self.resources.len();
                            for allocation in self.get_mapped_allocations(profile){
                                let allocation = &mut self.resources[allocation];
                                match allocation {
                                    AllocatorResourceType::Allocation(a) => {
                                        
                                        match a.bind_buffer(&mut buffer){
                                            Ok(_) => {
                                                match buffer.get_region::<O>(object_count, alignment, options){
                                                    Ok(r) => {
                                                        region = Some(r);
                                                        break;
                                                        
                                                    },
                                                    Err(e) => match e {
                                                        MemoryBlockError::NoSpace => panic!("Buffer profile settings needs adjustment"),
                                                    },
                                                }
                                            },
                                            Err(e) => match e {
                                                MemoryBlockError::NoSpace => {},
                                            },
                                        };



                                    },
                                    AllocatorResourceType::Buffer(_) => panic!("Wrong intersection"),
                                    AllocatorResourceType::Image(_) => panic!("Wrong intersection"),
                                }
                            }

                            match region {
                                Some(_) => {
                                    match &mut self.settings[*bi]{
                                        AllocatorProfileType::Allocation(_) => panic!("Profile index mismatch"),
                                        AllocatorProfileType::Buffer(b) => b.dependant_buffers.push(buffer.buffer_resource_index),
                                        AllocatorProfileType::Image(_) => panic!("Profile index mismatch"),
                                    }
                                    self.add_mapped_buffer(profile, buffer.buffer_resource_index);
                                    self.resources.push(AllocatorResourceType::Buffer(buffer));
                                },
                                None => {
                                    //If no allocations have space we need to create both
                                    let mut a_profile = match &self.settings[*ai] {
                                        AllocatorProfileType::Allocation(a) => a.clone(),
                                        AllocatorProfileType::Buffer(_) => panic!("Profile index mismatch"),
                                        AllocatorProfileType::Image(_) => panic!("Profile index mismatch"),
                                    };
                                    let (properties, a_options) = a_profile.get_settings();
                                    let mut allocation = self.create_allocation::<u8>(properties, buffer.mem_reqs.size as usize, a_options);
                                    allocation.allocation_resource_index = self.resources.len() + 1;
                                    match allocation.bind_buffer(&mut buffer){
                                        Ok(_) => {
                                            match buffer.get_region::<O>(object_count, alignment, options){
                                                Ok(r) => {
                                                    region = Some(r);

                                                    match &mut self.settings[*bi]{
                                                        AllocatorProfileType::Allocation(_) => panic!("Profile index mismatch"),
                                                        AllocatorProfileType::Buffer(b) => b.dependant_buffers.push(buffer.buffer_resource_index),
                                                        AllocatorProfileType::Image(_) => panic!("Profile index mismatch"),
                                                    }
                                                    match &mut self.settings[*ai]{
                                                        AllocatorProfileType::Allocation(a) => {
                                                            a.dependant_allocations.push(allocation.allocation_resource_index);
                                                        },
                                                        AllocatorProfileType::Buffer(_) => panic!("Profile index mismatch"),
                                                        AllocatorProfileType::Image(_) => panic!("Profile index mismatch"),
                                                    }
                                                    self.add_mapped_buffer(profile, buffer.buffer_resource_index);
                                                    self.add_mapped_allocation(profile, allocation.allocation_resource_index);
                                                    self.resources.push(AllocatorResourceType::Buffer(buffer));
                                                    self.resources.push(AllocatorResourceType::Allocation(allocation))
                                                },
                                                Err(e) => match e {
                                                    MemoryBlockError::NoSpace => panic!("Buffer profile settings needs adjustment"),
                                                },
                                            }
                                        },
                                        Err(e) => match e {
                                            MemoryBlockError::NoSpace => panic!("Allocation profile settings need ot be adjusted"),
                                        },
                                    };
                                },
                            }
                        },
                    }
                },
                AllocatorProfileStack::TargetImage(_, _) => panic!("Profile stack targeting image in buffer region creation"),
            }
            
            match region {
                Some(r) => r,
                None => panic!("Allocator failure"),
            }
        }
        pub fn get_image_resources(&mut self, profile: &AllocatorProfileStack, aspect: vk::ImageAspectFlags, base_mip_level: u32, mip_level_depth: u32, base_layer: u32, layer_depth: u32, view_type: vk::ImageViewType, format: vk::Format, options: &[CreateImageResourceOptions]) ->ImageResources {
            let mut img_resources = None;

            match profile {
                AllocatorProfileStack::TargetBuffer(_, _) => panic!("Targeting buffer in image resource creation"),
                AllocatorProfileStack::TargetImage(ai, ii) => {
                    for image in self.get_mapped_images(profile){
                        let image = &self.resources[image];
                        match image {
                            AllocatorResourceType::Allocation(_) => panic!("Wrong intersection"),
                            AllocatorResourceType::Buffer(_) => panic!("Wrong intersection"),
                            AllocatorResourceType::Image(i) => {
                                img_resources = Some(i.get_resources(aspect, base_mip_level, mip_level_depth, base_layer, layer_depth, view_type, format, options));
                                break;
                            },
                        }
                    }

                    match img_resources {
                        Some(_) => {},
                        None => {
                            let (usage, format, extent, i_options) = match &self.settings[*ii]{
                                AllocatorProfileType::Allocation(_) => panic!("Profile index mismatch"),
                                AllocatorProfileType::Buffer(_) => panic!("Profile index mismatch"),
                                AllocatorProfileType::Image(i) => i.get_settings(),
                            };
                            let mut image = self.create_image(usage, format, extent, i_options);
                            image.image_resource_index = self.resources.len();

                            for allocation in self.get_mapped_allocations(profile){
                                let allocation = &mut self.resources[allocation];
                                match allocation {
                                    AllocatorResourceType::Allocation(a) => {
                                        
                                        match a.bind_image(&mut image){
                                            Ok(_) => {
                                                img_resources = Some(image.get_resources(aspect, base_mip_level, mip_level_depth, base_layer, layer_depth, view_type, format, options));
                                                break;
                                            },
                                            Err(e) => match e {
                                                MemoryBlockError::NoSpace => {},
                                            },
                                        };



                                    },
                                    AllocatorResourceType::Buffer(_) => panic!("Wrong intersection"),
                                    AllocatorResourceType::Image(_) => panic!("Wrong intersection"),
                                }
                            }

                            match img_resources {
                                Some(_) => {
                                    match &mut self.settings[*ii]{
                                        AllocatorProfileType::Allocation(_) => panic!("Profile index mismatch"),
                                        AllocatorProfileType::Buffer(_) => panic!("Profile index mismatch"),
                                        AllocatorProfileType::Image(i) => i.dependant_images.push(image.image_resource_index),
                                    }
                                    self.add_mapped_image(profile, image.image_resource_index);
                                    self.resources.push(AllocatorResourceType::Image(image))
                                },
                                None => {
                                    //If no allocations have space we need to create both
                                    let mut a_profile = match &self.settings[*ai] {
                                        AllocatorProfileType::Allocation(a) => a.clone(),
                                        AllocatorProfileType::Buffer(_) => panic!("Profile index mismatch"),
                                        AllocatorProfileType::Image(_) => panic!("Profile index mismatch"),
                                    };
                                    let (properties, a_options) = a_profile.get_settings();
                                    let mut allocation = self.create_allocation::<u8>(properties, image.mem_reqs.size as usize, a_options);
                                    allocation.allocation_resource_index = self.resources.len() + 1;
                                    match allocation.bind_image(&mut image){
                                        Ok(_) => {
                                            img_resources = Some(image.get_resources(aspect, base_mip_level, mip_level_depth, base_layer, layer_depth, view_type, format, options));
                                            
                                            match &mut self.settings[*ii]{
                                                AllocatorProfileType::Allocation(_) => panic!("Profile index mismatch"),
                                                AllocatorProfileType::Buffer(_) => panic!("Profile index mismatch"),
                                                AllocatorProfileType::Image(i) => i.dependant_images.push(image.image_resource_index),
                                            }
                                            match &mut self.settings[*ai]{
                                                AllocatorProfileType::Allocation(a) => {
                                                    a.dependant_allocations.push(allocation.allocation_resource_index);
                                                },
                                                AllocatorProfileType::Buffer(_) => panic!("Profile index mismatch"),
                                                AllocatorProfileType::Image(_) => panic!("Profile index mismatch"),
                                            }
                                            self.add_mapped_image(profile, image.image_resource_index);
                                            self.add_mapped_allocation(profile, allocation.allocation_resource_index);
                                            self.resources.push(AllocatorResourceType::Image(image));
                                            self.resources.push(AllocatorResourceType::Allocation(allocation));
                                        },
                                        Err(e) => match e {
                                            MemoryBlockError::NoSpace => panic!("Allocation profile settings needs to be adjusted"),
                                        },
                                    };
                                },
                            }
                        },
                    }
                },
            }

            


            match img_resources {
                Some(i) => i,
                None => panic!("Allocator failure"),
            }
        }
        pub fn copy_from_ram<O>(&mut self, src: *const O, object_count: usize, dst: &BufferRegion){
            match &mut self.resources[dst.allocation_resource_index] {
                AllocatorResourceType::Allocation(a) => {
                    a.copy_from_ram(src, object_count, dst)
                },
                AllocatorResourceType::Buffer(_) => panic!("Resource index mismatch"),
                AllocatorResourceType::Image(_) => panic!("Resource index mismatch"),
            }
        }
        pub fn copy_from_ram_slice<O>(&mut self, objects: &[O], dst: &BufferRegion){
            match &mut self.resources[dst.allocation_resource_index] {
                AllocatorResourceType::Allocation(a) => {
                    a.copy_from_ram_slice(objects, dst);
                },
                AllocatorResourceType::Buffer(_) => panic!("Resource index mismatch"),
                AllocatorResourceType::Image(_) => panic!("Resource index mismatch"),
            }
        }
        pub fn copy_to_ram<O>(&self, src: &BufferRegion, dst: *mut O, object_count: usize, ){
            match &self.resources[src.allocation_resource_index] {
                AllocatorResourceType::Allocation(a) => {
                    a.copy_to_ram(src, dst, object_count)
                },
                AllocatorResourceType::Buffer(_) => panic!("Resource index mismatch"),
                AllocatorResourceType::Image(_) => panic!("Resource index mismatch"),
            }
        }
        pub fn copy_to_ram_slice<O>(&self, src: &BufferRegion, dst: &mut [O]){
            match &self.resources[src.allocation_resource_index] {
                AllocatorResourceType::Allocation(a) => {
                    a.copy_to_ram_slice(src, dst)
                },
                AllocatorResourceType::Buffer(_) => panic!("Resource index mismatch"),
                AllocatorResourceType::Image(_) => panic!("Resource index mismatch"),
            }
        }
        fn get_resource_intersection(&self, profile: &AllocatorProfileStack) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
            let mut allocations = vec![];
            let mut buffers = vec![];
            let mut images = vec![];
            for (index, resource) in self.resources.iter().enumerate(){

                match profile{
                    AllocatorProfileStack::TargetBuffer(ai, bi) => {
                        match &self.settings[*ai]{
                            AllocatorProfileType::Allocation(a) => {
                                if a.dependant_allocations.contains(&index){
                                    allocations.push(index);
                                }
                            },
                            AllocatorProfileType::Buffer(_) => panic!("Profile index mismatch"),
                            AllocatorProfileType::Image(_) => panic!("Profile index mismatch"),
                        }
                        match &self.settings[*bi]{
                            AllocatorProfileType::Allocation(_) => panic!("Profile index mismatch"),
                            AllocatorProfileType::Buffer(b) => {
                                if b.dependant_buffers.contains(&index){
                                    buffers.push(index);
                                }
                            },
                            AllocatorProfileType::Image(_) => panic!("Profile index mismatch"),
                        }
                    },
                    AllocatorProfileStack::TargetImage(ai, ii) => {
                        match &self.settings[*ai]{
                            AllocatorProfileType::Allocation(a) => {
                                if a.dependant_allocations.contains(&index){
                                    allocations.push(index);
                                }
                            },
                            AllocatorProfileType::Buffer(_) => panic!("Profile index mismatch"),
                            AllocatorProfileType::Image(_) => panic!("Profile index mismatch"),
                        }
                        match &self.settings[*ii]{
                            AllocatorProfileType::Allocation(_) => panic!("Profile index mismatch"),
                            AllocatorProfileType::Buffer(_) => panic!("Profile index mismatch"),
                            AllocatorProfileType::Image(i) => {
                                if i.dependant_images.contains(&index){
                                    images.push(index);
                                }
                            },
                        }
                    },
                }
            }
            (allocations,buffers,images)
        }
        fn add_profile_mapping(&mut self, profile: &AllocatorProfileStack){
            let (allocations, _, _) = match self.profile_mapping.insert(profile.clone(), (vec![], vec![], vec![])){
                Some(_) => panic!("profile already mapped"),
                None => {
                    self.profile_mapping.get_mut(profile).expect("Could not map profile")
                },
            };
            match profile {
                AllocatorProfileStack::TargetBuffer(ai, _) => {
                    match &self.settings[*ai]{
                        AllocatorProfileType::Allocation(a) => {
                            for allocation in a.dependant_allocations.iter(){
                                allocations.push(*allocation);
                            }
                        },
                        AllocatorProfileType::Buffer(_) => panic!("mismatched profile index"),
                        AllocatorProfileType::Image(_) => panic!("mismatched profile index"),
                    }
                },
                AllocatorProfileStack::TargetImage(ai, _) => {
                    match &self.settings[*ai]{
                        AllocatorProfileType::Allocation(a) => {
                            for allocation in a.dependant_allocations.iter(){
                                allocations.push(*allocation);
                            }
                        },
                        AllocatorProfileType::Buffer(_) => panic!("mismatched profile index"),
                        AllocatorProfileType::Image(_) => panic!("mismatched profile index"),
                    }
                },
            }
        }
        fn get_mapped_allocations(&mut self, profile: &AllocatorProfileStack) -> Vec<usize> {
            let (a,_,_) = match self.profile_mapping.get(profile){
                Some(data) => data,
                None => {
                    self.add_profile_mapping(profile);
                    self.profile_mapping.get(profile).expect("Did not enter new profile mapping")
                },
            };
            a.clone()
        }
        fn get_mapped_buffers(&mut self, profile: &AllocatorProfileStack) -> Vec<usize> {
            let (_,b,_) = match self.profile_mapping.get(profile){
                Some(data) => data,
                None => {
                    self.add_profile_mapping(profile);
                    self.profile_mapping.get(profile).expect("Did not enter new profile mapping")
                },
            };
            b.clone()
        }
        fn get_mapped_images(&mut self, profile: &AllocatorProfileStack) -> Vec<usize> {
            let (_,_,i) = match self.profile_mapping.get_mut(profile){
                Some(data) => data,
                None => {
                    self.add_profile_mapping(profile);
                    self.profile_mapping.get(profile).expect("Did not enter new profile mapping")
                },
            };
            i.clone()
        }
        fn add_mapped_allocation(&mut self, profile: &AllocatorProfileStack, resource_index: usize){
            let (a,_,_) = match self.profile_mapping.get_mut(profile){
                Some(data) => data,
                None => {
                    self.add_profile_mapping(profile);
                    self.profile_mapping.get_mut(profile).expect("Did not enter new profile mapping")
                },
            };
            a.push(resource_index);
        }
        fn add_mapped_buffer(&mut self, profile: &AllocatorProfileStack, resource_index: usize){
            let (_,b,_) = match self.profile_mapping.get_mut(profile){
                Some(data) => data,
                None => {
                    self.add_profile_mapping(profile);
                    self.profile_mapping.get_mut(profile).expect("Did not enter new profile mapping")
                },
            };
            b.push(resource_index);
        }
        fn add_mapped_image(&mut self, profile: &AllocatorProfileStack, resource_index: usize){
            let (_,_,i) = match self.profile_mapping.get_mut(profile){
                Some(data) => data,
                None => {
                    self.add_profile_mapping(profile);
                    self.profile_mapping.get_mut(profile).expect("Did not enter new profile mapping")
                },
            };
            i.push(resource_index);
        }
    }
    impl AllocationAllocatorProfile{
        pub fn new(memory_properties: vk::MemoryPropertyFlags, options: &[CreateAllocationOptions]) -> AllocationAllocatorProfile {
            AllocationAllocatorProfile{ dependant_allocations: vec![], memory_properties, options: options.to_vec() }
        }
        pub fn get_settings(&mut self) -> (vk::MemoryPropertyFlags, &mut [CreateAllocationOptions]) {
            (self.memory_properties, &mut self.options)
        }
    }
    impl BufferAllocatorProfile{
        pub fn new(usage: vk::BufferUsageFlags, options: &[CreateBufferOptions]) -> BufferAllocatorProfile {
            BufferAllocatorProfile{ dependant_buffers: vec![], usage, options: options.to_vec() }
        }
        fn get_settings(&self) -> (vk::BufferUsageFlags, &[CreateBufferOptions]) {
            (self.usage, &self.options)
        }
    }
    impl ImageAllocatorProfile{
        pub fn new(usage: vk::ImageUsageFlags, format: vk::Format, extent: vk::Extent3D, options: &[CreateImageOptions]) -> ImageAllocatorProfile {
            ImageAllocatorProfile{ 
                dependant_images: vec![], 
                usage, 
                format, 
                extent, 
                options: options.to_vec() }
        }
        fn get_settings(&self) -> (vk::ImageUsageFlags, vk::Format, vk::Extent3D, &[CreateImageOptions]) {
            (self.usage, self.format, self.extent, &self.options)
        }
    }
    impl AllocatorProfileStack{
        pub fn target_buffer(allocation_profile_index: usize, buffer_profile_index:usize) -> AllocatorProfileStack {
            AllocatorProfileStack::TargetBuffer(allocation_profile_index, buffer_profile_index)
        }
        pub fn target_image(allocation_profile_index: usize, image_profile_index:usize) -> AllocatorProfileStack {
            AllocatorProfileStack::TargetImage(allocation_profile_index, image_profile_index)
        }
    }
    
    impl MemoryBlockArray{
        fn merge_unused(&mut self){
            match self {
                MemoryBlockArray::Allocation(a) => {
                    let mut fixed_array:Vec<AllocationMemoryBlock> = vec![];
                    let mut merge_block: Option<AllocationMemoryBlock> = None;
                    for (index, block) in a.iter().enumerate(){
                        if Arc::strong_count(&block.user) == 1{
                            match &mut merge_block {
                                Some(b) => {
                                    debug!("Adding size {} to merge block from empty block at offset {}", block.size, block.start_offset);
                                    b.size += block.size;
                                },
                                None => {
                                    debug!("Empty block at index {} and offset {} and of size {} starting new merge_block", index, block.start_offset, block.size);
                                    merge_block = Some(block.clone());
                                },
                            }
                        }
                        else {
                            match &mut merge_block {
                                Some(b) => {
                                    debug!("Pushing merge block at offset {} and of size {} to the new block array at index {}", b.start_offset, b.size, fixed_array.len());
                                    fixed_array.push(b.clone());
                                    debug!("Pushing in-use block at offset {} and of size {} to the new block array at index {}", block.start_offset, block.size, fixed_array.len());
                                    fixed_array.push(block.clone());
                                    merge_block = None;
                                },
                                None => {
                                    debug!("Pushing in use block at offset {} and of size {} to the new block array at index {}", block.start_offset, block.size, fixed_array.len());
                                    fixed_array.push(block.clone());
                                },
                            }
                        }
                    }
                    match &mut merge_block {
                        Some(block) => {
                            debug!("Pushing merge block at offset {} and of size {} to the new block array at index {}", block.start_offset, block.size, fixed_array.len());
                            fixed_array.push(block.clone())
                        },
                        None => {},
                    }
                    *a = fixed_array;
                },
                MemoryBlockArray::Buffer(a) => {
                    let mut fixed_array:Vec<BufferMemoryBlock> = vec![];
                    let mut merge_block: Option<BufferMemoryBlock> = None;
                    for (index, block) in a.iter().enumerate(){
                        if Arc::strong_count(&block.user) == 1{
                            match &mut merge_block {
                                Some(b) => {
                                    debug!("Adding size {} to merge block from empty block at offset {}", block.size, block.buffer_offset);
                                    b.size += block.size
                                },
                                None => {
                                    debug!("Empty block at index {} and offset {} and of size {} starting new merge_block", index, block.buffer_offset, block.size);
                                    merge_block = Some(block.clone())
                                },
                            }
                        }
                        else {
                            match &mut merge_block {
                                Some(b) => {
                                    fixed_array.push(b.clone());
                                    debug!("Pushing merge block at offset {} and of size {} to the new block array at index {}", b.buffer_offset, b.size, fixed_array.len());
                                    fixed_array.push(block.clone());
                                    debug!("Pushing in-use block at offset {} and of size {} to the new block array at index {}", block.buffer_offset, block.size, fixed_array.len());
                                    merge_block = None;
                                },
                                None => {
                                    debug!("Pushing in use block at offset {} and of size {} to the new block array at index {}", block.buffer_offset, block.size, fixed_array.len());
                                    fixed_array.push(block.clone());
                                },
                            }
                        }
                    }
                    match &mut merge_block {
                        Some(block) => {
                            debug!("Pushing merge block at offset {} and of size {} to the new block array at index {}", block.buffer_offset, block.size, fixed_array.len());
                            fixed_array.push(block.clone())
                        },
                        None => {},
                    }
                    *a = fixed_array;
                },
            }
        }
        fn try_get_region(&mut self, size: u64, alignment: AlignmentType) -> Result<usize, MemoryBlockError> {
            debug!("Trying to get a region\n");
            self.merge_unused();
            let mut selected_index:Result<usize, MemoryBlockError> = Result::Err(MemoryBlockError::NoSpace);
            

            match self {
                MemoryBlockArray::Allocation(a) => {
                    let (mut target_offset, mut block_size) = (0,0);


                    for (index, block) in a.iter().enumerate(){
                        if (Arc::strong_count(&block.user) == 1){
                            (target_offset, block_size) = block.get_offset_and_remaining_size(&alignment);
                            if block_size >= size{
                            debug!("Found unused allocation block with adjusted offset {} and of size {} that satifies needed size of {}", target_offset, block_size, size);
                            selected_index = Ok(index);
                            break;
                        }
                        }
                        
                    }   
                    
                    match selected_index {
                        Ok(i) => {
                            let old_block = a[i].clone();

                            let new_block = AllocationMemoryBlock{ 
                                start_offset: target_offset, 
                                size, 
                                user: old_block.user };
                            let unused_block = AllocationMemoryBlock{ 
                                start_offset: target_offset + size, 
                                size: old_block.size - ((target_offset-old_block.start_offset) + size), 
                                user: Arc::new(true) };
                            
                            debug!("Created new allocation block at offset {} and of size {} and an unused block at offset {} and size {}", new_block.start_offset, new_block.size, unused_block.start_offset, unused_block.size);
                            
                            if i > 0 {
                                let previous_block = &mut a[i-1];
                                previous_block.size += (target_offset - (previous_block.start_offset + previous_block.size));
                            }

                            a[i] = new_block;
                            a.insert(i+1, unused_block);
                        },
                        Err(_) => {},
                    }

                },
                MemoryBlockArray::Buffer(a) => {
                    let (mut target_allocation_offset, mut target_buffer_offset, mut block_size) = (0,0,0);

                    for (index, block) in a.iter().enumerate(){
                        if Arc::strong_count(&block.user) == 1{
                            (target_allocation_offset, target_buffer_offset, block_size) = block.get_offset_and_remaining_size(&alignment);
                            if block_size >= size{
                                debug!("Found unused allocation block at offset {} and of size {} that satifies needed size of {}", target_buffer_offset, block_size, size);
                                selected_index = Ok(index);
                                break;
                            }
                        }
                        
                    }   
                    
                    match selected_index {
                        Ok(i) => {
                            let old_block = a[i].clone();

                            let new_block = BufferMemoryBlock { 
                                allocation_offset: target_allocation_offset, 
                                buffer_offset: target_buffer_offset, 
                                size: size, 
                                user: old_block.user };
                            let unused_block = BufferMemoryBlock { 
                                allocation_offset: target_allocation_offset + size, 
                                buffer_offset: target_buffer_offset + size, 
                                size: old_block.size - ((target_buffer_offset - old_block.buffer_offset) + size), 
                                user: Arc::new(false) };
                            
                            debug!("Created new buffer block at offset {} and of size {} and an unused block at offset {} and size {}", new_block.buffer_offset, new_block.size, unused_block.buffer_offset, unused_block.size);
                            
                            if i > 0 {
                                let previous_block = &mut a[i-1];
                                previous_block.size += (target_buffer_offset - (previous_block.buffer_offset + previous_block.size));
                            }

                            a[i] = new_block;
                            a.insert(i+1, unused_block);
                        },
                        Err(_) => {},
                    }
                },
            }

            selected_index
        }
    }
    impl AllocationMemoryBlock{
        fn get_offset_and_remaining_size(&self, alignment: &AlignmentType) -> (u64, u64) {
            let data = match alignment {
                AlignmentType::Free => {
                    (self.start_offset, self.size)
                },
                AlignmentType::Allocation(a) => {
                    if *a == 1 || self.start_offset == 0 || *a % self.start_offset == 0{
                        (self.start_offset, self.size)
                    }
                    else {
                        let offset = (self.start_offset/ *a + 1) *  *a;
                        let size;
                        if self.size < (offset - self.start_offset){
                            size = 0;
                        }
                        else {
                            size = self.size - (offset - self.start_offset);
                        }
                        (offset, size)
                    }
                },
                AlignmentType::User(_) => {panic!("Cannot use User alignment type on allocation")},
            };
            data
        }
    }
    impl BufferMemoryBlock{
        fn get_offset_and_remaining_size(&self, alignment: &AlignmentType) -> (u64, u64, u64) {
            let (allocation_offset, buffer_offset, remaining_size) = match alignment {
                AlignmentType::Free => {
                    (self.allocation_offset, self.buffer_offset, self.size)
                },
                AlignmentType::Allocation(a) => {
                    if *a == 1 || self.allocation_offset == 0 || *a % self.allocation_offset == 0{
                        (self.allocation_offset, self.buffer_offset, self.size)
                    }
                    else {
                        let allocation_offset = ((self.allocation_offset/ *a + 1) * *a);
                        let buffer_offset = allocation_offset + (self.buffer_offset - self.allocation_offset);

                        let size;
                        if self.size < (buffer_offset - self.buffer_offset){
                            size = 0;
                        }
                        else {
                            size = self.size - (buffer_offset - self.buffer_offset);
                        }
                        
                        (allocation_offset, buffer_offset, size)
                    }
                },
                AlignmentType::User(a) => {
                    if *a == 1 || self.buffer_offset == 0 || *a % self.buffer_offset == 0{
                        (self.allocation_offset, self.buffer_offset, self.size)
                    }
                    else{
                        let buffer_offset = (self.buffer_offset / *a + 1) * *a;
                        let allocation_offset = buffer_offset + (self.allocation_offset - self.buffer_offset);

                        let size;
                        if self.size < (buffer_offset - self.buffer_offset){
                            size = 0;
                        }
                        else {
                            size = self.size - (buffer_offset - self.buffer_offset);
                        }

                        (allocation_offset, buffer_offset, size)
                    }
                },
            };
            (allocation_offset, buffer_offset, remaining_size)
        }
    }
    
    
    impl Allocation{
        pub fn bind_buffer(&mut self, buffer: &mut Buffer) -> Result<(), MemoryBlockError> {
            match self.blocks.try_get_region(buffer.mem_reqs.size, AlignmentType::Allocation(buffer.mem_reqs.alignment)){
                Ok(block_index) => {
                    let block = match &self.blocks {
                        MemoryBlockArray::Allocation(a) => {
                            a[block_index].clone()
                        },
                        MemoryBlockArray::Buffer(_) => panic!("What?"),
                    };
        
                    unsafe{
                        self.device.bind_buffer_memory(buffer.buffer, self.allocation, block.start_offset);
                    }
        
                    let default_block = BufferMemoryBlock{ 
                        allocation_offset: block.start_offset, 
                        buffer_offset: 0, 
                        size: buffer.c_info.size, 
                        user: Arc::new(true) };

                    buffer.memory_info = Some((self.memory_type, self.c_info.memory_type_index, block, MemoryBlockArray::Buffer(vec![default_block])));
                    buffer.allocation_resource_index = self.allocation_resource_index;
                    Ok(())
                },
                Err(e) => Err(e),
            }
        }
        pub fn bind_image(&mut self, image: &mut Image) -> Result<(), MemoryBlockError> {
            match self.blocks.try_get_region(image.mem_reqs.size, AlignmentType::Allocation(image.mem_reqs.alignment)){
                Ok(block_index) => {

                    let block = match &self.blocks {
                        MemoryBlockArray::Allocation(a) => {
                            a[block_index].clone()
                        },
                        MemoryBlockArray::Buffer(_) => panic!("What?"),
                    };
        
                    unsafe{
                        self.device.bind_image_memory(image.image, self.allocation, block.start_offset);
                    }
                    
                    image.memory_info = Some((self.memory_type, self.c_info.memory_type_index, block));
                    image.allocation_resource_index = self.allocation_resource_index;
                    Ok(())
                },
                Err(e) => Err(e),
            }
        }
        pub fn copy_from_ram<O>(&mut self, src: *const O, object_count: usize, dst: &BufferRegion){
            assert!((size_of::<O>() * object_count) as u64 <= dst.home_block.size);
            let target_allocation = self.allocation;
            let target_offset = dst.home_block.allocation_offset;
            let mapped_range = vk::MappedMemoryRange::builder()
                .memory(target_allocation)
                .offset(0)
                .size(vk::WHOLE_SIZE)
                .build();
    
            unsafe {
                debug!("Copying {} objects of size {} from {:?} to allocation {:?} at {} targeting buffer {:?}", object_count, size_of::<O>(), src, target_allocation, target_offset, dst.buffer);
                let dst = (self.device.map_memory(target_allocation, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()).unwrap() as *mut u8).offset(target_offset as isize) as *mut O;
                std::ptr::copy_nonoverlapping(src, dst, object_count);
                self.device.flush_mapped_memory_ranges(&vec![mapped_range]).unwrap();
                self.device.unmap_memory(target_allocation);
            }
        }
        pub fn copy_from_ram_slice<O>(&mut self, objects: &[O], dst: &BufferRegion){
            let src = objects.as_ptr();
            let object_count = objects.len();
            self.copy_from_ram(src, object_count, dst);
        }
        pub fn copy_to_ram<O>(&self, src: &BufferRegion, dst: *mut O, object_count: usize, ){
            assert!((size_of::<O>() * object_count) as u64 <= src.home_block.size);
            let src_allocation = self.allocation;
            let src_offset = src.home_block.allocation_offset;
            let mapped_range = vk::MappedMemoryRange::builder()
            .memory(src_allocation)
            .offset(0)
            .size(vk::WHOLE_SIZE)
            .build();
    
            unsafe {
                debug!("Copying {} objects of size {} to {:?} from allocation {:?} at {}", object_count, size_of::<O>(), dst, src_allocation, src.home_block.allocation_offset);
                let src = (self.device.map_memory(src_allocation, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()).unwrap() as *const u8).offset(src_offset as isize) as *const O;
                self.device.invalidate_mapped_memory_ranges(&vec![mapped_range]).unwrap();
                std::ptr::copy_nonoverlapping(src, dst, object_count);
                self.device.unmap_memory(src_allocation);
            }
        }
        pub fn copy_to_ram_slice<O>(&self, src: &BufferRegion, dst: &mut [O]){
            let object_count = dst.len();
            let dst = dst.as_mut_ptr();
            self.copy_to_ram(src, dst, object_count);
        }
    }
    impl CreateAllocationOptions {
        fn add_options<'a>(&'a mut self, mut info: vk::MemoryAllocateInfoBuilder<'a>) -> vk::MemoryAllocateInfoBuilder {
            match self {
                CreateAllocationOptions::MemoryAllocateFlags(f) => info = info.push_next(f),
                CreateAllocationOptions::MinimumSize(s) => {if info.allocation_size < *s {
                    info = info.allocation_size(*s);
                }},
                CreateAllocationOptions::SizeOverkillFactor(f) => {
                    let size = info.allocation_size;
                    info = info.allocation_size(size * *f);
                },
            }
            info
        }
    }
    
    impl Buffer{
        pub fn get_region<T>(&mut self, object_count: usize, alignment: &AlignmentType, options: &[CreateBufferRegionOptions]) -> Result<BufferRegion, MemoryBlockError> {

            match &mut self.memory_info  {
                Some((t,ti,b,a)) => {
                    let size = size_of::<T>() * object_count;
            let alignment = match alignment {
                AlignmentType::Free => {
                    if self.c_info.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER){
                        AlignmentType::User(self.properties.pd_properties.limits.min_storage_buffer_offset_alignment)
                    }
                    else {
                        AlignmentType::Free
                    }
                },
                AlignmentType::Allocation(a) => AlignmentType::Allocation(*a),
                AlignmentType::User(a) => AlignmentType::User(*a),
            };
            match a.try_get_region(size as u64, alignment){
                Ok(block_index) => {
                    let block = match &a {
                        MemoryBlockArray::Allocation(_) => panic!("Should not be here"),
                        MemoryBlockArray::Buffer(a) => a[block_index].clone(),
                    };
                    let default_block = BufferMemoryBlock{ 
                        allocation_offset: block.allocation_offset, 
                        buffer_offset: block.buffer_offset, 
                        size: size as u64, 
                        user: Arc::new(true) };
                    Ok(BufferRegion{ 
                        device: self.device.clone(), 
                        properties: self.properties.clone(), 
                        buffer: self.buffer, 
                        memory_type: *t, 
                        memory_index: *ti, 
                        buffer_usage: self.c_info.usage, 
                        home_block: block,
                        blocks: MemoryBlockArray::Buffer(vec![default_block]),
                        buffer_resource_index: self.buffer_resource_index,
                        allocation_resource_index: self.allocation_resource_index, })
                
                },
                Err(e) => Err(e),
            }
            
                },
                None => panic!("Trying to use unbound buffer"),
            }

            }
    }
    impl BufferRegion{
        pub fn get_region<T>(&mut self, object_count: usize, alignment: AlignmentType, options: &[CreateBufferRegionOptions]) -> Result<BufferRegion, MemoryBlockError> {
            let size = size_of::<T>() * object_count;
            let alignment = match alignment {
                AlignmentType::Free => {
                    if self.buffer_usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER){
                        AlignmentType::User(self.properties.pd_properties.limits.min_storage_buffer_offset_alignment)
                    }
                    else {
                        AlignmentType::Free
                    }
                },
                AlignmentType::Allocation(a) => AlignmentType::Allocation(a),
                AlignmentType::User(a) => AlignmentType::User(a),
            };
            match self.blocks.try_get_region(size as u64, alignment){
                Ok(block_index) => {
                    let block = match &self.blocks {
                        MemoryBlockArray::Allocation(_) => panic!("Should not be here"),
                        MemoryBlockArray::Buffer(a) => a[block_index].clone(),
                    };
                    let default_block = BufferMemoryBlock{ 
                        allocation_offset: block.allocation_offset, 
                        buffer_offset: block.buffer_offset, 
                        size: size as u64, 
                        user: Arc::new(true) };
                    Ok(BufferRegion{ 
                        device: self.device.clone(), 
                        properties: self.properties.clone(), 
                        buffer: self.buffer, 
                        memory_type: self.memory_type, 
                        memory_index: self.memory_index, 
                        buffer_usage: self.buffer_usage, 
                        home_block: block,
                        blocks: MemoryBlockArray::Buffer(vec![default_block]),
                        buffer_resource_index: 0,
                        allocation_resource_index: 0, })
                
                },
                Err(e) => Err(e),
            }
            }
        pub fn copy_to_region(&self, cmd: vk::CommandBuffer, dst: &BufferRegion){
            let copy = [self.get_copy_info(dst)];
            unsafe{
                self.device.cmd_copy_buffer(cmd, self.buffer, dst.buffer, &copy);
                debug!("Recorded copy of {} bytes from buffer {:?} at {} to buffer {:?} at {}", copy[0].size, self.buffer, copy[0].src_offset, dst.buffer, copy[0].dst_offset);
            }

        }
        pub fn get_copy_info(&self, tgt: &BufferRegion) -> vk::BufferCopy {
            assert!(tgt.home_block.size >= self.home_block.size);
            vk::BufferCopy::builder().src_offset(self.home_block.buffer_offset).dst_offset(tgt.home_block.buffer_offset).size(self.home_block.size).build()
        }
        pub fn copy_to_image(&self, cmd: vk::CommandBuffer, dst: &ImageResources){
            let copy = [vk::BufferImageCopy::builder()
            .buffer_offset(self.home_block.buffer_offset)
            .image_subresource(dst.target_layers)
            .image_offset(dst.target_offset)
            .image_extent(dst.target_extent)
            .build()];
            unsafe{
                self.device.cmd_copy_buffer_to_image(cmd, self.buffer, dst.image, dst.layout, &copy);
            }
        }
        pub fn get_binding(&self) -> DescriptorWriteType {
            let info = vk::DescriptorBufferInfo::builder()
            .buffer(self.buffer)
            .offset(self.home_block.buffer_offset)
            .range(self.home_block.size)
            .build();
            DescriptorWriteType::Buffer([info])
        }
    }
    impl CreateBufferOptions{
        fn add_options<'a>(&'a self, mut info: vk::BufferCreateInfoBuilder<'a>) -> vk::BufferCreateInfoBuilder {
            match self {
                CreateBufferOptions::BufferCreateFlags(f) => {
                    debug!("Using non-standard buffer create flags");
                    info = info.flags(*f);
                },
                CreateBufferOptions::SizeOverkillFactor(factor) => {
                    debug!("Overkilling buffer size by {}", factor);
                    let size = info.size;
                    info = info.size(size * factor);
                },
                CreateBufferOptions::MinimumSize(s) => {
                    if info.size < *s {
                        info = info.size(*s);
                    }
                },
            }
            info
        }
    }
    
    impl Image{
        pub fn get_resources(&self, aspect: vk::ImageAspectFlags, base_mip_level: u32, mip_level_depth: u32, base_layer: u32, layer_depth: u32, view_type: vk::ImageViewType, format: vk::Format, options: &[CreateImageResourceOptions]) -> ImageResources {
            
            match &self.memory_info{
                Some((t,ti,_)) => {
                    let mut layout = self.c_info.initial_layout;
            let sizzle = vk::ComponentMapping::builder()
            .a(vk::ComponentSwizzle::A)
            .r(vk::ComponentSwizzle::R)
            .g(vk::ComponentSwizzle::G)
            .b(vk::ComponentSwizzle::B);
            let subresource = vk::ImageSubresourceRange::builder()
            .aspect_mask(aspect)
            .base_mip_level(base_mip_level)
            .level_count(mip_level_depth)
            .base_array_layer(base_layer)
            .layer_count(layer_depth);
            let mut c_info = vk::ImageViewCreateInfo::builder()
            .image(self.image)
            .view_type(view_type)
            .format(format)
            .components(sizzle.build())
            .subresource_range(subresource.build());
            for option in options.iter(){
                match option {
                    CreateImageResourceOptions::Swizzle(s) => {
                        debug!("Using non standard image view swizzle");
                        c_info = c_info.components(*s);
                    },
                    CreateImageResourceOptions::Flags(f) => {
                        debug!("Using non standard image create flags");
                        c_info = c_info.flags(*f);
                    },
                    CreateImageResourceOptions::Layout(l) => {
                        debug!("Using non standard image layout");
                        layout = *l;
                    },
                }
            }
            
            let view = unsafe{self.device.create_image_view(&c_info, None).expect("Could not create image view")};
            debug!("Created image view {:?}", view);

            let target_extent = self.c_info.extent;
            let target_layers = vk::ImageSubresourceLayers::builder()
            .aspect_mask(aspect)
            .mip_level(base_mip_level)
            .base_array_layer(base_layer)
            .layer_count(layer_depth)
            .build();

            ImageResources{ 
                device: self.device.clone(), 
                properties: self.properties.clone(), 
                image: self.image, 
                layout, 
                view, 
                c_info: c_info.build(), 
                memory_type: *t, 
                memory_index: *ti, 
                target_offset: vk::Offset3D::builder().build(),
                target_extent,
                target_layers,
                disposed: false,
                allocation_resource_index: 0,
                image_resource_index: 0, }

                },
                None => panic!("trying to use unbound image"),
            }
            
            
        }
        pub fn get_extent(&self) -> vk::Extent3D {
            self.c_info.extent
        }
    }
    impl ImageResources{
        pub fn set_target_extent(&mut self, target_extent: vk::Extent3D, target_offset: vk::Offset3D){
            self.target_offset = target_offset;
            self.target_extent = target_extent;
        }
        pub fn set_target_layers(&mut self, aspect: vk::ImageAspectFlags, target_mip_level: u32, start_layer: u32, layer_depth: u32){
            let layers = vk::ImageSubresourceLayers::builder()
            .aspect_mask(aspect)
            .mip_level(target_mip_level)
            .base_array_layer(start_layer)
            .layer_count(layer_depth)
            .build();
            self.target_layers = layers;
        }
        pub fn transition(&mut self, src_access: vk::AccessFlags, dst_access: vk::AccessFlags, new_layout: vk::ImageLayout) -> (vk::ImageMemoryBarrier, vk::ImageLayout) {
            let transition = vk::ImageMemoryBarrier::builder()
            .old_layout(self.layout)
            .new_layout(new_layout)
            .src_queue_family_index(u32::MAX)
            .dst_queue_family_index(u32::MAX)
            .image(self.image)
            .subresource_range(self.c_info.subresource_range).build();
            let old_layout = self.layout;
            self.layout = new_layout;
            (transition, old_layout)
        }   
        pub fn copy_to_buffer(&self, cmd: vk::CommandBuffer, dst: &BufferRegion){
            let copy = [vk::BufferImageCopy::builder()
            .buffer_offset(dst.home_block.buffer_offset)
            .image_subresource(self.target_layers)
            .image_offset(self.target_offset)
            .image_extent(self.target_extent)
            .build()];
            unsafe{
                self.device.cmd_copy_image_to_buffer(cmd, self.image, self.layout, dst.buffer, &copy);
            }
        }
        pub fn copy_to_image(&self, cmd: vk::CommandBuffer, dst: &ImageResources){
            let copy = [vk::ImageCopy::builder()
            .src_subresource(self.target_layers)
            .src_offset(self.target_offset)
            .dst_subresource(dst.target_layers)
            .dst_offset(dst.target_offset)
            .extent(self.target_extent)
            .build()];

            unsafe{
                self.device.cmd_copy_image(cmd, self.image, self.layout, dst.image, dst.layout, &copy);
            }
        }
        pub fn new_from_image<T:IEngine>(engine: &T, image: vk::Image, layout: vk::ImageLayout, extent: vk::Extent3D, aspect: vk::ImageAspectFlags, base_mip_level: u32, mip_level_depth: u32, base_layer: u32, layer_depth: u32, view_type: vk::ImageViewType, format: vk::Format, options: &[CreateImageResourceOptions]) -> ImageResources {
            let mut layout = layout;
            let sizzle = vk::ComponentMapping::builder()
            .a(vk::ComponentSwizzle::A)
            .r(vk::ComponentSwizzle::R)
            .g(vk::ComponentSwizzle::G)
            .b(vk::ComponentSwizzle::B);
            let subresource = vk::ImageSubresourceRange::builder()
            .aspect_mask(aspect)
            .base_mip_level(base_mip_level)
            .level_count(mip_level_depth)
            .base_array_layer(base_layer)
            .layer_count(layer_depth);
            let mut c_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(view_type)
            .format(format)
            .components(sizzle.build())
            .subresource_range(subresource.build());
            for option in options.iter(){
                match option {
                    CreateImageResourceOptions::Swizzle(s) => {
                        debug!("Using non standard image view swizzle");
                        c_info = c_info.components(*s);
                    },
                    CreateImageResourceOptions::Flags(f) => {
                        debug!("Using non standard image create flags");
                        c_info = c_info.flags(*f);
                    },
                    CreateImageResourceOptions::Layout(l) => {
                        debug!("Using non standard image layout");
                        layout = *l;
                    },
                }
            }
            
            let view = unsafe{engine.get_device().create_image_view(&c_info, None).expect("Could not create image view")};
            debug!("Created image view {:?}", view);

            let target_extent = extent;
            let target_layers = vk::ImageSubresourceLayers::builder()
            .aspect_mask(aspect)
            .mip_level(base_mip_level)
            .base_array_layer(base_layer)
            .layer_count(layer_depth)
            .build();

            ImageResources{ 
                device: engine.get_device(), 
                properties: engine.get_property_store(), 
                image, 
                layout, 
                view, 
                c_info: c_info.build(), 
                memory_type: vk::MemoryPropertyFlags::empty(), 
                memory_index: 0, 
                target_offset: vk::Offset3D::builder().build(), 
                target_extent, 
                target_layers,
                disposed: false,
                allocation_resource_index: 0,
                image_resource_index: 0, }
        }
    }
    

    impl IDisposable for Allocation{
            fn dispose(&mut self) {
            if !self.disposed{
                self.disposed = true;
                debug!("Destroying allocation {:?}", self.allocation);
                unsafe{
                    self.device.free_memory(self.allocation, None);
                }
            }
        }
    }
    impl IDisposable for Buffer{
            fn dispose(&mut self) {
            if !self.disposed{
                self.disposed = true;
                debug!("Destroying buffer {:?}", self.buffer);
                match &mut self.memory_info {
                    Some((_,_,block,_)) => drop(*block.user),
                    None => todo!(),
                }
                unsafe{
                    self.device.destroy_buffer(self.buffer, None);
                }
            }
            
        }
    }
    impl IDisposable for Image{
            fn dispose(&mut self) {
            if !self.disposed{
                self.disposed = true;
                debug!("Destroying image {:?}", self.image);
                match &mut self.memory_info {
                    Some((_,_,block)) => drop(*block.user),
                    None => todo!(),
                }
                unsafe{
                    self.device.destroy_image(self.image, None);
                }
            }
        }
    }
    impl IDisposable for ImageResources{
            fn dispose(&mut self) {
            if !self.disposed{
                self.disposed = true;
                debug!("Destroying image view {:?}", self.view);
                unsafe{
                    self.device.destroy_image_view(self.view, None);
                }
            }
        }
    }
    impl IDisposable for Allocator{
        fn dispose(&mut self) {
            let mut allocations = vec![];

            for resource in self.resources.iter_mut(){
                match resource {
                    AllocatorResourceType::Allocation(a) => allocations.push(a),
                    AllocatorResourceType::Buffer(b) => {
                        b.dispose();
                    },
                    AllocatorResourceType::Image(i) => {
                        i.dispose()
                    },
                }
            }

            for allocation in allocations{
                allocation.dispose();
            }
        }
    }
    

    impl Drop for Allocator{
        fn drop(&mut self) {
        self.dispose();
        }
    }
    impl Drop for Allocation{
        fn drop(&mut self) {
            self.dispose();
    }
    }
    impl Drop for Buffer{
            fn drop(&mut self) {
                self.dispose();
        }
    }
    impl Drop for Image{
            fn drop(&mut self) {
                self.dispose();
        }
    }
    impl Drop for ImageResources{
        fn drop(&mut self) {
            self.dispose();
        }
    }
    

}
#[allow(dead_code, unused)]
pub mod descriptor{
    use std::os::raw::c_void;
    use ash::vk;
    use log::debug;

    use crate::{init::IEngine, IDisposable};

    #[derive(Clone)]
    pub enum DescriptorWriteType{
        Buffer([vk::DescriptorBufferInfo;1]),
        Image([vk::DescriptorImageInfo;1]),
        AccelerationStructure(Option<Box<[vk::AccelerationStructureKHR;1]>>, vk::WriteDescriptorSetAccelerationStructureKHR)
    }
    #[derive(Clone)]
    pub enum CreateDescriptorSetLayoutOptions{
        Flags(vk::DescriptorSetLayoutCreateFlags)
    }
    #[derive(Clone)]
    pub enum CreateDescriptorPoolOptions{
        Flags(vk::DescriptorPoolCreateFlags)
    }


    pub struct DescriptorSetOutline{
        device: ash::Device,
        bindings: Vec<vk::DescriptorSetLayoutBinding>,
        options: Vec<CreateDescriptorSetLayoutOptions>,
        layout: Option<vk::DescriptorSetLayout>,
        disposed: bool,
    }

    pub struct DesciptorStack{
        device: ash::Device,
        pool: Option<vk::DescriptorPool>,
        outlines: Vec<DescriptorSetOutline>,
        sets: Vec<vk::DescriptorSet>,
        disposed: bool,
    }
    pub struct DescriptorSet{
        device: ash::Device,
        layout: vk::DescriptorSetLayout,
        set: vk::DescriptorSet,
        bindings: Vec<vk::DescriptorSetLayoutBinding>,
    }



    impl CreateDescriptorSetLayoutOptions{
        pub fn apply_option<'a>(&'a self, mut info: vk::DescriptorSetLayoutCreateInfoBuilder<'a>) -> vk::DescriptorSetLayoutCreateInfoBuilder {
            match self {
                CreateDescriptorSetLayoutOptions::Flags(f) => {
                    info = info.flags(*f);
                },
            }
            info
        }
    }
    impl CreateDescriptorPoolOptions{
        pub fn apply_option<'a>(&'a self, mut info: vk::DescriptorPoolCreateInfoBuilder<'a>) -> vk::DescriptorPoolCreateInfoBuilder{
            match self {
                CreateDescriptorPoolOptions::Flags(f) => {
                    info = info.flags(*f);
                },
            }
            info
        }
    }
    impl DescriptorSetOutline{
        pub fn new<T:IEngine>(engine: &T, options: &[CreateDescriptorSetLayoutOptions]) -> DescriptorSetOutline {
            DescriptorSetOutline{ device: engine.get_device(), bindings: vec![], layout: None, disposed: false, options: options.to_vec() }
        }
        pub fn add_binding(&mut self, ty: vk::DescriptorType, count: u32, stages: vk::ShaderStageFlags) -> usize {
            match self.layout{
                Some(_) => panic!("Descriptor set layout already built"),
                None => {
                    let binding = vk::DescriptorSetLayoutBinding::builder()
                    .binding(self.bindings.len() as u32)
                    .descriptor_type(ty)
                    .descriptor_count(count)
                    .stage_flags(stages)
                    .build();
                    self.bindings.push(binding);
                },
            }
            self.bindings.len()-1
        }
        pub fn get_layout(&mut self) -> vk::DescriptorSetLayout{
            match self.layout {
                Some(l) => l,
                None => {
                    let mut c_info = vk::DescriptorSetLayoutCreateInfo::builder()
                    .bindings(&self.bindings);
                    for option in self.options.iter(){
                        c_info = option.apply_option(c_info);
                    }
        
                    let layout = unsafe{self.device.create_descriptor_set_layout(&c_info, None).expect("Could not create descriptor set layout")};
                    debug!("Created descriptor set layout {:?}", layout);
                    self.layout = Some(layout);
                    layout
                },
            }

            
        }
    }


    impl DesciptorStack {
        pub fn new<T:IEngine>(engine: &T) -> DesciptorStack {
            DesciptorStack{ device: engine.get_device(),
                 pool: None,
                 outlines: vec![],
                 sets: vec![],
                 disposed: false }
        }
        pub fn add_outline(&mut self, outline: DescriptorSetOutline) -> usize {
            self.outlines.push(outline);
            self.outlines.len()-1
        }
        pub fn create_sets(&mut self, options: &[CreateDescriptorPoolOptions]){
            match self.pool {
                Some(_) => {}
                None => {
                    self.create_pool(options);
                },
            }
            
            let layouts: Vec<vk::DescriptorSetLayout> = self.outlines.iter_mut().map(|i| i.get_layout()).collect();
            let a_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(self.pool.expect("Allocating sets with no descriptor pool"))
            .set_layouts(&layouts);
            let allocated_sets = unsafe{self.device.allocate_descriptor_sets(&a_info).expect("Could not create descriptor sets")};
            debug!("Created sets {:?}", allocated_sets);
            self.sets = allocated_sets;
            
        }
        pub fn get_set(&mut self, set_index: usize) -> DescriptorSet {
            DescriptorSet{ device: self.device.clone(),
                 layout: self.outlines[set_index].get_layout(),
                 set: self.sets[set_index],
                 bindings: self.outlines[set_index].bindings.clone() }
            
        }
        fn create_pool(&mut self, options: &[CreateDescriptorPoolOptions]){
            let mut pool_sizes: Vec<vk::DescriptorPoolSize> = Vec::with_capacity(self.outlines.len());         
            let mut pool: vk::DescriptorPool;
            for outline in self.outlines.iter(){
                for (binding) in outline.bindings.iter(){
                    let found = pool_sizes.iter().enumerate().find(|(_,s)| s.ty == binding.descriptor_type);
                     match found {
                        Some((i, _)) => {pool_sizes[i].descriptor_count += 1;},
                        None => {pool_sizes.push(vk::DescriptorPoolSize::builder().ty(binding.descriptor_type).descriptor_count(1).build());},
                    }
                }
            }
            
            let mut c_info = vk::DescriptorPoolCreateInfo::builder()
            .max_sets(self.outlines.len() as u32)
            .pool_sizes(&pool_sizes);
            for option in options.iter(){
                c_info = option.apply_option(c_info);                
            }
            
            let pool = unsafe{self.device.create_descriptor_pool(&c_info, None).expect("Could not create descriptor pool")};
            debug!("Created pool {:?}", pool);
            self.pool = Some(pool);
            
        }
    }

    impl DescriptorSet{
        pub fn get_layout(&self) -> vk::DescriptorSetLayout {
            self.layout
        }
        pub fn get_set(&self) -> vk::DescriptorSet {
            self.set
        }
        //Request: (binding index, dst_array_element, writes)
        pub fn write(&mut self, requests: &mut [(usize, usize, DescriptorWriteType)]){
            let mut set_writes = vec![];
            for (binding, start_array_index, write) in requests.iter_mut(){
                let d_type = self.bindings[*binding].descriptor_type;
                match write {
                    DescriptorWriteType::Buffer(b) => {
                        let write = vk::WriteDescriptorSet::builder()
                        .dst_set(self.set)
                        .descriptor_type(d_type)
                        .dst_binding(*binding as u32)
                        .dst_array_element(*start_array_index as u32)
                        .buffer_info(b)
                        .build();
                        debug!("Generated descriptor set write {:?}", write);
                    
                        set_writes.push(write);
                    },
                    DescriptorWriteType::Image(i) => {
                        let write = vk::WriteDescriptorSet::builder()
                        .dst_set(self.set)
                        .descriptor_type(d_type)
                        .dst_binding(*binding as u32)
                        .dst_array_element(*start_array_index as u32)
                        .image_info(i)
                        .build();
                        debug!("Generated descriptor set write {:?}", write);
                    
                        set_writes.push(write);
                        
                    },
                    DescriptorWriteType::AccelerationStructure(i, a) => {
                        let write = vk::WriteDescriptorSet::builder()
                        .dst_set(self.set)
                        .descriptor_type(d_type)
                        .dst_binding(*binding as u32)
                        .push_next(a)
                        .build();
                        debug!("Generated descriptor set write {:?}", write);
                    
                        set_writes.push(write);
                        
                    },
                }
            }
            unsafe{
                self.device.update_descriptor_sets(&set_writes, &[]);
            }
        }
    }

    impl IDisposable for DescriptorSetOutline{
        fn dispose(&mut self) {
            match self.layout {
                Some(l) => {
                    if !self.disposed{
                        self.disposed = true;
                        debug!("Destroying descriptor set layout {:?}", l);
                        unsafe{
                            self.device.destroy_descriptor_set_layout(l, None);
                        }
                    }
                },
                None => {},
            }
            
    }
    }
    impl IDisposable for DesciptorStack{
        fn dispose(&mut self) {
        if !self.disposed{
                self.disposed = true;
                match self.pool{
                    Some(p) => {
                        debug!("Destroying descriptor pool {:?}", p);
                        unsafe{
                            self.device.destroy_descriptor_pool(p, None);
                        }
                    },
                    None => {}
                }
                debug!("Destroying descriptor sets {:?}", self.sets);    
                for outline in self.outlines.iter_mut(){
                    outline.dispose();
                }
            }   
        }
    }


    impl Drop for DescriptorSetOutline{
        fn drop(&mut self) {
        self.dispose();
    }
    }
    impl Drop for DesciptorStack{
        fn drop(&mut self) {
            self.dispose();
    }
    }

}
pub mod sync{
    use ash::vk;
    use log::debug;

    use crate::{init::IEngine, IDisposable};

    pub struct Fence{
        device: ash::Device,
        fence: ash::vk::Fence,
        disposed: bool,
    }
    impl Fence{
        pub fn new<T: IEngine>(engine: &T, start_signaled: bool) -> Fence{
            let fence;
            let c_info;
            if start_signaled{
                c_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED).build();
            }
            else {
                c_info = vk::FenceCreateInfo::builder().build();
            }

            unsafe{
                fence = engine.get_device().create_fence(&c_info, None).expect("Could not create fence");
            }
            debug!("Created fence {:?}", fence);
            Fence{ device: engine.get_device(), fence, disposed: false }
        }
        pub fn wait(&self){
            unsafe{
                self.device.wait_for_fences(&[self.fence], true, u64::max_value()).expect("Could not wait on fence");
            }
        }
        pub fn wait_reset(&self){
            self.wait();
            unsafe{
                self.device.reset_fences(&[self.fence]).expect("Could not reset fence");
            }
        }
        pub fn get_fence(&self) -> vk::Fence{
            self.fence
        }
    }
    impl IDisposable for Fence{
        fn dispose(&mut self) {
        if !self.disposed{
            self.disposed = true;
            debug!("Destroying fence {:?}", self.fence);
            unsafe{
                self.device.destroy_fence(self.fence, None);
            }
        }
    }
    }
    impl Drop for Fence{
        fn drop(&mut self) {
            self.dispose();
        }
    }

    pub struct Semaphore{
        device: ash::Device,
        pub semaphore: vk::Semaphore,
        disposed: bool,
    }
    impl Semaphore{
        pub fn new<T: IEngine>(engine: &T) -> Semaphore {
            let device = engine.get_device();
            let c_info = vk::SemaphoreCreateInfo::builder().build();
            let semaphore = unsafe{device.create_semaphore(&c_info, None).expect("Could not create semaphore")};
            debug!("Created semaphore {:?}", semaphore);

            Semaphore{
                device,
                semaphore,
                disposed: false,
            }
        }
    }
    impl IDisposable for Semaphore{
        fn dispose(&mut self) {
        if !self.disposed{
            self.disposed = true;
            debug!("Destroying semaphore {:?}", self.semaphore);
            unsafe{self.device.destroy_semaphore(self.semaphore, None)};
        }
    }
    }
    impl Drop for Semaphore{
        fn drop(&mut self) {
            self.dispose();
    }
    }
}
pub mod ray_tracing{}
#[allow(dead_code, unused)]
pub mod shader{
    use std::ffi::CStr;

    use ash::vk;
    use log::debug;

    use crate::init::IEngine;

    pub struct Shader{
        device: ash::Device,
        source: String,
        module: vk::ShaderModule,       
    }
    impl Shader{
        pub fn new<T: IEngine>(engine: &T, source: String, kind: shaderc::ShaderKind, ep_name: &str, options: Option<&shaderc::CompileOptions>) -> Shader{
            let module: vk::ShaderModule;
            let compiler = shaderc::Compiler::new().unwrap();
            let byte_source = compiler.compile_into_spirv(source.as_str(), kind, "shader.glsl", ep_name, options).unwrap();
            debug!("Compiled shader {} to binary {:?}", source, byte_source.as_binary());
            unsafe{
                let c_info = vk::ShaderModuleCreateInfo::builder().code(byte_source.as_binary()).build();
                module = engine.get_device().create_shader_module(&c_info, None).unwrap();
            }
            Shader { device: engine.get_device(), source, module }
        }
        pub fn get_stage(&self, stage: vk::ShaderStageFlags, ep: &CStr) -> vk::PipelineShaderStageCreateInfo{
            vk::PipelineShaderStageCreateInfo::builder()
            .stage(stage)
            .module(self.module)
            .name(ep)
            .build()
        }
    }
    impl Drop for Shader{
        fn drop(&mut self) {
        unsafe{
            debug!("Destroying Shader");
            self.device.destroy_shader_module(self.module, None);
        }
        }
    }
    
}
#[allow(dead_code, unused)]
pub mod compute{
    use ash::vk;
    use log::debug;

    use crate::{init::IEngine, IDisposable};

    pub struct ComputePipeline{
        device: ash::Device,
        layout: vk::PipelineLayout,
        pipeline: vk::Pipeline,
        c_info: vk::ComputePipelineCreateInfo,
        push_ranges: Vec<vk::PushConstantRange>,
        descriptor_sets: Vec<vk::DescriptorSetLayout>,
        disposed: bool,
    }
    impl ComputePipeline{
        pub fn new<T: IEngine>(engine: &T, push_ranges: &[vk::PushConstantRange], descriptor_sets: &[vk::DescriptorSetLayout], shader: vk::PipelineShaderStageCreateInfo) -> ComputePipeline{
            let device = engine.get_device();
            let lc_info = vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(push_ranges)
            .set_layouts(descriptor_sets)
            .build();
            let layout: vk::PipelineLayout;
            let c_infos: [vk::ComputePipelineCreateInfo;1];
            let pipeline: vk::Pipeline;

            unsafe{
                layout = device.create_pipeline_layout(&lc_info, None).unwrap();
                c_infos = [vk::ComputePipelineCreateInfo::builder()
                .stage(shader)
                .layout(layout)
                .build()];
                pipeline = device.create_compute_pipelines(vk::PipelineCache::null(), &c_infos, None).unwrap()[0];
            }
            debug!("Created compute pipeline {:?}", pipeline);

            ComputePipeline{ device,
                 layout,
                 pipeline,
                 c_info: c_infos[0],
                 push_ranges: push_ranges.to_vec(),
                 descriptor_sets: descriptor_sets.to_vec(),
                 disposed: false }
        }
        pub fn get_pipeline(&self) -> vk::Pipeline{
            self.pipeline
        }
        pub fn get_layout(&self) -> vk::PipelineLayout{
            self.layout
        }
    }
    impl IDisposable for ComputePipeline{
        fn dispose(&mut self) {
            if !self.disposed{
                self.disposed = true;
                debug!("Destroying compute pipeline layout {:?}", self.layout);
                debug!("Destroying compute pipeline {:?}", self.pipeline);
                unsafe{
                    self.device.destroy_pipeline(self.pipeline, None);
                    self.device.destroy_pipeline_layout(self.layout, None);
                }
            }
    }
    }
    impl Drop for ComputePipeline{
        fn drop(&mut self) {
            self.dispose();
        }
    }
    
    
}

#[cfg(test)]
mod tests{
    use ash::vk;

    use crate::{init::{self, Engine, IEngine}, memory::{self, AlignmentType, AllocationAllocatorProfile, BufferAllocatorProfile, AllocatorProfileStack, CreateAllocationOptions, CreateBufferOptions, Allocator}, IDisposable, descriptor::{DescriptorSetOutline, DesciptorStack}, shader::{self, Shader}};

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

    #[test]
    fn image_memory_round_trip(){
        match pretty_env_logger::try_init() {
            Ok(_) => {},
            Err(_) => {},
        };
        let mut options = vec![];
        get_vulkan_validate(&mut options);
        let (engine, _) = Engine::init(&mut options, None);

        let pool = unsafe{engine.get_device().create_command_pool(&vk::CommandPoolCreateInfo::builder().queue_family_index(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER).unwrap().1).build(), None).expect("Could not create command pool")};
        let cmd = unsafe{engine.get_device().allocate_command_buffers(&vk::CommandBufferAllocateInfo::builder().command_pool(pool).command_buffer_count(1).build()).expect("Could not allocate command buffers")}[0];
        const WIDTH:u32 = 1024;
        const HEIGHT:u32 = 1024;
        let extent = vk::Extent3D::builder().width(WIDTH).height(HEIGHT).depth(1).build();

        let data:Vec<u32> = vec![u32::from_be_bytes([255,0,0,0]);(WIDTH*HEIGHT) as usize];

        let allocator = Allocator::new(&engine);

        let mut cpu_mem = allocator.create_allocation::<u32>(vk::MemoryPropertyFlags::HOST_COHERENT, data.len() * 2, &mut []);
        let mut cpu_buffer = allocator.create_buffer::<u32>(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, data.len()*2, &[]);
        cpu_mem.bind_buffer(&mut cpu_buffer).expect("Could not bind buffer");
        let staging = cpu_buffer.get_region::<u32>(data.len(), &AlignmentType::Free, &[]).expect("Could not make buffer region");
        let target = cpu_buffer.get_region::<u32>(data.len(), &AlignmentType::Free, &[]).expect("Could not make buffer region");
    
        let mut gpu_mem = allocator.create_allocation::<u32>(vk::MemoryPropertyFlags::DEVICE_LOCAL, data.len()*2, &mut []);
        let mut image = allocator.create_image(vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::STORAGE, vk::Format::R8G8B8A8_UNORM, extent, &[]);
        gpu_mem.bind_image(&mut image).expect("Could not bind image");
        let mut processing = image.get_resources(
            vk::ImageAspectFlags::COLOR, 
            0, 
            1, 
            0, 
            1, 
            vk::ImageViewType::TYPE_2D, 
            vk::Format::R8G8B8A8_UNORM, 
            &[]);
    
        cpu_mem.copy_from_ram_slice(&data, &staging);

        unsafe{
            engine.get_device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).unwrap();
            let (to_transfer_dst, _) = processing.transition(vk::AccessFlags::NONE, vk::AccessFlags::TRANSFER_WRITE, vk::ImageLayout::TRANSFER_DST_OPTIMAL); 
            
            engine.get_device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TOP_OF_PIPE,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[to_transfer_dst]);
            
            staging.copy_to_image(cmd, &processing);
            
            //let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::TRANSFER_WRITE).dst_access_mask(vk::AccessFlags::MEMORY_READ).build();
            let (to_transfer_src, _) = processing.transition(vk::AccessFlags::TRANSFER_WRITE, vk::AccessFlags::TRANSFER_READ, vk::ImageLayout::TRANSFER_SRC_OPTIMAL); 
            engine.get_device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[], &[], &[to_transfer_src]);
            
            processing.copy_to_buffer(cmd, &target);
            
            engine.get_device().end_command_buffer(cmd).unwrap();
            
            
            
            let cmds = [cmd];
            let submit = vk::SubmitInfo::builder().command_buffers(&cmds).build();
            engine.get_device().queue_submit(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER).unwrap().0, &[submit], vk::Fence::null()).unwrap();
            engine.get_device().queue_wait_idle(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER).unwrap().0).unwrap();
        }

        let mut tgt:Vec<u32> = vec![0;(WIDTH*HEIGHT) as usize];

        cpu_mem.copy_to_ram_slice(&target, &mut tgt);

        assert!(tgt.last().unwrap() == data.last().unwrap());

        unsafe{
            engine.get_device().destroy_command_pool(pool, None);
        }
    }

    #[test]
    //Image round trip
    fn memory_round_trip(){
        match pretty_env_logger::try_init() {
            Ok(_) => {},
            Err(_) => {},
        };
        let mut options = vec![];
        get_vulkan_validate(&mut options);
        let (engine, _) = Engine::init(&mut options, None);

        let pool = unsafe{engine.get_device().create_command_pool(&vk::CommandPoolCreateInfo::builder().queue_family_index(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER).unwrap().1).build(), None).expect("Could not create command pool")};
        let cmd = unsafe{engine.get_device().allocate_command_buffers(&vk::CommandBufferAllocateInfo::builder().command_pool(pool).command_buffer_count(1).build()).expect("Could not allocate command buffers")}[0];
        let allocator = Allocator::new(&engine);

        let data:Vec<u64> = (0..100).collect();

        let mut cpu_mem = allocator.create_allocation::<u64>(vk::MemoryPropertyFlags::HOST_COHERENT, data.len() * 2, &mut []);
        let mut cpu_buffer = allocator.create_buffer::<u64>(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, data.len()*2, &[]);
        cpu_mem.bind_buffer(&mut cpu_buffer).expect("Could not bind cpu buffer");
        let staging = cpu_buffer.get_region::<u64>(data.len(), &AlignmentType::Free, &[]).expect("Could not make buffer region");
        let target = cpu_buffer.get_region::<u64>(data.len(), &AlignmentType::Free, &[]).expect("Could not make buffer region");
    
        let mut gpu_mem = allocator.create_allocation::<u64>(vk::MemoryPropertyFlags::DEVICE_LOCAL, data.len(), &mut []);
        let mut gpu_buffer = allocator.create_buffer::<u64>(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, data.len(), &[]);
        gpu_mem.bind_buffer(&mut gpu_buffer).expect("Could not bind gpu buffer");
        let processing = gpu_buffer.get_region::<u64>(data.len(), &AlignmentType::Free, &[]).expect("Could not make region");
    
        cpu_mem.copy_from_ram_slice(&data, &staging);

        unsafe{
            engine.get_device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).unwrap();
            staging.copy_to_region(cmd, &processing);
            let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::MEMORY_WRITE).dst_access_mask(vk::AccessFlags::MEMORY_READ).build();
            engine.get_device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);
            processing.copy_to_region(cmd, &target);
            engine.get_device().end_command_buffer(cmd).unwrap();
            let cmds = [cmd];
            let submit = vk::SubmitInfo::builder().command_buffers(&cmds).build();
            engine.get_device().queue_submit(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER).unwrap().0, &[submit], vk::Fence::null()).unwrap();
            engine.get_device().queue_wait_idle(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER).unwrap().0).unwrap();
        }

        let mut tgt:[u64;100] = [0;100];

        cpu_mem.copy_to_ram_slice(&target, &mut tgt);
        unsafe{
            engine.get_device().destroy_command_pool(pool, None);
        }

        assert!(tgt.last().unwrap() == data.last().unwrap());

        
    }


    #[test]
    fn allocator_test(){
        match pretty_env_logger::try_init() {
            Ok(_) => {},
            Err(_) => {},
        };
        let mut options = vec![];
        get_vulkan_validate(&mut options);
        let (engine, _) = Engine::init(&mut options, None);

        let pool = unsafe{engine.get_device().create_command_pool(&vk::CommandPoolCreateInfo::builder().queue_family_index(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER).unwrap().1).build(), None).expect("Could not create command pool")};
        let cmd = unsafe{engine.get_device().allocate_command_buffers(&vk::CommandBufferAllocateInfo::builder().command_pool(pool).command_buffer_count(1).build()).expect("Could not allocate command buffers")}[0];

        let data:Vec<u64> = (0..100).collect();

        let mem_options = vec![CreateAllocationOptions::MinimumSize(1024*1024*100)];
        let buffer_options = vec![CreateBufferOptions::MinimumSize(1024*1024)];

        let mut allocator = memory::Allocator::new(&engine);
        let cpu_mem_profile = allocator.add_profile(memory::AllocatorProfileType::Allocation(AllocationAllocatorProfile::new(vk::MemoryPropertyFlags::HOST_COHERENT, &mem_options)));
        let gpu_mem_profile = allocator.add_profile(memory::AllocatorProfileType::Allocation(AllocationAllocatorProfile::new(vk::MemoryPropertyFlags::DEVICE_LOCAL, &mem_options)));
        let buffer_profile = allocator.add_profile(memory::AllocatorProfileType::Buffer(BufferAllocatorProfile::new(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, &buffer_options)));
        let cpu_stack = AllocatorProfileStack::TargetBuffer(cpu_mem_profile, buffer_profile);
        let gpu_stack = AllocatorProfileStack::TargetBuffer(gpu_mem_profile, buffer_profile);

        let staging = allocator.get_buffer_region::<u64>( &cpu_stack, data.len(), &AlignmentType::Free, &[]);
        let target = allocator.get_buffer_region::<u64>(&cpu_stack, data.len(), &AlignmentType::Free, &[]);
        let processing = allocator.get_buffer_region::<u64>(&gpu_stack, data.len(), &AlignmentType::Free, &[]);
    
        allocator.copy_from_ram_slice(&data, &staging);

        unsafe{
            engine.get_device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).unwrap();
            staging.copy_to_region(cmd, &processing);
            let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::MEMORY_WRITE).dst_access_mask(vk::AccessFlags::MEMORY_READ).build();
            engine.get_device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);
            processing.copy_to_region(cmd, &target);
            engine.get_device().end_command_buffer(cmd).unwrap();
            let cmds = [cmd];
            let submit = vk::SubmitInfo::builder().command_buffers(&cmds).build();
            engine.get_device().queue_submit(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER).unwrap().0, &[submit], vk::Fence::null()).unwrap();
            engine.get_device().queue_wait_idle(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER).unwrap().0).unwrap();
        }

        let mut tgt:[u64;100] = [0;100];

        allocator.copy_to_ram_slice(&target, &mut tgt);
        allocator.dispose();
        unsafe{
            engine.get_device().destroy_command_pool(pool, None);
        }



        assert!(tgt.last().unwrap() == data.last().unwrap());

    }
    
    #[test]
    fn target(){
        match pretty_env_logger::try_init() {
            Ok(_) => {},
            Err(_) => {},
        };
        let mut options = vec![];
        get_vulkan_validate(&mut options);
        let (engine, _) = Engine::init(&mut options, None);

        let pool = unsafe{engine.get_device().create_command_pool(&vk::CommandPoolCreateInfo::builder().queue_family_index(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().1).build(), None).expect("Could not create command pool")};
        let cmd = unsafe{engine.get_device().allocate_command_buffers(&vk::CommandBufferAllocateInfo::builder().command_pool(pool).command_buffer_count(1).build()).expect("Could not allocate command buffers")}[0];

        let data:Vec<u32> = (0..100).collect();

        let mem_options = vec![CreateAllocationOptions::MinimumSize(1024*1024*100)];
        let buffer_options = vec![CreateBufferOptions::MinimumSize(1024*1024)];

        let mut allocator = memory::Allocator::new(&engine);
        let cpu_mem_profile = allocator.add_profile(memory::AllocatorProfileType::Allocation(AllocationAllocatorProfile::new(vk::MemoryPropertyFlags::HOST_COHERENT, &mem_options)));
        let gpu_mem_profile = allocator.add_profile(memory::AllocatorProfileType::Allocation(AllocationAllocatorProfile::new(vk::MemoryPropertyFlags::DEVICE_LOCAL, &mem_options)));
        let buffer_profile = allocator.add_profile(memory::AllocatorProfileType::Buffer(BufferAllocatorProfile::new(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, &buffer_options)));
        let cpu_stack = AllocatorProfileStack::TargetBuffer(cpu_mem_profile, buffer_profile);
        let gpu_stack = AllocatorProfileStack::TargetBuffer(gpu_mem_profile, buffer_profile);

        let staging = allocator.get_buffer_region::<u32>( &cpu_stack, data.len(), &AlignmentType::Free, &[]);
        let target = allocator.get_buffer_region::<u32>(&cpu_stack, data.len(), &AlignmentType::Free, &[]);
        let processing = allocator.get_buffer_region::<u32>(&gpu_stack, data.len(), &AlignmentType::Free, &[]);
        let binding = processing.get_binding();
        let mut outline = DescriptorSetOutline::new(&engine, &[]);
        let b_index = outline.add_binding(vk::DescriptorType::STORAGE_BUFFER, 1, vk::ShaderStageFlags::COMPUTE);
        let mut stack = DesciptorStack::new(&engine);
        let proccessing_set = stack.add_outline(outline);
        stack.create_sets(&[]);
        let mut set = stack.get_set(proccessing_set);
        set.write(&mut [(b_index, 0, binding)]);
    
        
        let shader = Shader::new(&engine, String::from(r#"
        #version 460
        #extension GL_KHR_vulkan_glsl : enable

        layout(local_size_x = 1) in;

        layout(set = 0, binding = 0) buffer Data {
            uint[] values;
        } data;

        void main(){
            data.values[gl_GlobalInvocationID.x] += 100;
        }"#), shaderc::ShaderKind::Compute, "main", None);

        
        let mut compute = crate::compute::ComputePipeline::new(&engine, &[], &[set.get_layout()], shader.get_stage(vk::ShaderStageFlags::COMPUTE, &std::ffi::CString::new("main").unwrap()));
        
        
        allocator.copy_from_ram_slice(&data, &staging);

        unsafe{
            engine.get_device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).unwrap();
            staging.copy_to_region(cmd, &processing);
            let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::MEMORY_WRITE).dst_access_mask(vk::AccessFlags::SHADER_WRITE).build();
            engine.get_device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::COMPUTE_SHADER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);
            engine.get_device().cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, compute.get_pipeline());
            engine.get_device().cmd_bind_descriptor_sets(cmd, vk::PipelineBindPoint::COMPUTE, compute.get_layout(), 0, &[set.get_set()], &[]);
            engine.get_device().cmd_dispatch(cmd, data.len() as u32, 1, 1);
            let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::SHADER_WRITE).dst_access_mask(vk::AccessFlags::MEMORY_READ).build();
            engine.get_device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::COMPUTE_SHADER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);
            processing.copy_to_region(cmd, &target);
            engine.get_device().end_command_buffer(cmd).unwrap();
            let cmds = [cmd];
            let submit = vk::SubmitInfo::builder().command_buffers(&cmds).build();
            engine.get_device().queue_submit(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().0, &[submit], vk::Fence::null()).unwrap();
            engine.get_device().queue_wait_idle(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().0).unwrap();
        }

        let mut tgt:[u32;100] = [0;100];

        allocator.copy_to_ram_slice(&target, &mut tgt);
        allocator.dispose();
        compute.dispose();
        stack.dispose();
        unsafe{
            engine.get_device().destroy_command_pool(pool, None);
        }



        assert!(*tgt.last().unwrap() == *data.last().unwrap() + 100);
        
    }

}
