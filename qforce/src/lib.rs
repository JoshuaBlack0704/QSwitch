

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

#[allow(dead_code)]
pub mod core{
    use log::debug;
    use shaderc;
    use ash;
    use ash::{vk, Entry};
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
                    ash::extensions::khr::AccelerationStructure::name().as_ptr(),
                    ash::extensions::khr::DeferredHostOperations::name().as_ptr(),
                    ash::extensions::khr::RayTracingPipeline::name().as_ptr(),
                    ash::extensions::khr::BufferDeviceAddress::name().as_ptr(),
                    #[cfg(any(target_os = "macos", target_os = "ios"))]
                        KhrPortabilitySubsetFn::name().as_ptr(),
                ];
                let mut features13 = vk::PhysicalDeviceVulkan13Features::builder().dynamic_rendering(true).build();
                let mut features12 = vk::PhysicalDeviceVulkan12Features::builder().timeline_semaphore(true).buffer_device_address(true).build();
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
            Buffer(vk::DescriptorBufferInfo),
            Image(vk::DescriptorImageInfo),
        }

        #[doc = "Safe clonable structure that provides helper functions and data needed to resolve different requirements"]
        #[doc = "such as min offset alignments and such"]
        #[derive(Clone)]
        pub struct AllocationDataStore{
            instance: ash::Instance,
            physical_device: vk::PhysicalDevice,
            device: ash::Device,
            props: vk::PhysicalDeviceProperties,
            mem_props: vk::PhysicalDeviceMemoryProperties,
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
                    let mem_props = instance.get_physical_device_memory_properties(physical_device);
                    AllocationDataStore { 
                        instance, 
                        physical_device, 
                        device, 
                        props, 
                        mem_props, 
                        destroy_allocation: None, 
                        destroy_buffer: None, 
                        destroy_image: None }
                }
            }
            pub fn get_type(&self, properties: vk::MemoryPropertyFlags) -> u32{
                let mut selected_type: usize = 0;
                    //Selecting the corrent memory type
                    for type_index in 0..self.mem_props.memory_types.len(){
                        let mem_type = &self.mem_props.memory_types[type_index];
                        let heap = &self.mem_props.memory_heaps[mem_type.heap_index as usize];
                        if mem_type.property_flags & properties != vk::MemoryPropertyFlags::empty() {
                            debug!("Found compatible memory");
                            debug!("Type index: {}, Type property: {:?}, Type heap: {}", type_index, self.mem_props.memory_types[type_index].property_flags, self.mem_props.memory_types[type_index].heap_index);
                            if self.mem_props.memory_types[selected_type].property_flags & properties != vk::MemoryPropertyFlags::empty() {
                                if heap.size > self.mem_props.memory_heaps[self.mem_props.memory_types[selected_type].heap_index as usize].size && type_index != selected_type{
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
                
                write = DescriptorWriteType::Buffer(vk::DescriptorBufferInfo::builder().buffer(self.buffer).offset(self.buffer_offset).range(self.size).build());

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
                let mut b_infos:Vec<[vk::DescriptorBufferInfo;1]> = Vec::with_capacity(outlines.len()*2);
                let mut i_infos:Vec<[vk::DescriptorImageInfo;1]> = Vec::with_capacity(outlines.len()*2);

                for (index,set) in allocated_sets.iter().enumerate(){
                    let outline = outlines.get(index).unwrap().clone();
                    let layout = layouts.get(index).unwrap().clone();

                    for binding in outline.bindings.iter(){
                        match binding.1 {
                            DescriptorWriteType::Buffer(b) => {
                                b_infos.push([b]);
                                let write = vk::WriteDescriptorSet::builder()
                                .dst_set(*set)
                                .dst_array_element(0)
                                .dst_binding(binding.0.binding)
                                .descriptor_type(binding.0.descriptor_type)
                                .buffer_info(b_infos.last().unwrap())
                                .build();
                                debug!("Generated descriptor set write {:?}", write);
                                writes.push(write);
                            },
                            DescriptorWriteType::Image(i) => {
                                i_infos.push([i]);
                                let write = vk::WriteDescriptorSet::builder()
                                .dst_set(*set)
                                .dst_array_element(0)
                                .dst_binding(binding.0.binding)
                                .descriptor_type(binding.0.descriptor_type)
                                .image_info(i_infos.last().unwrap())
                                .build();
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
        
        #[doc = "Must survive as long as the create blas store command buffer is executing"]
        pub struct BlasStoreCreateRecipt{
            cpu_mem: Allocation,
            v_copy: Buffer,
            i_copy: Buffer,
            s_buffer: Buffer,
        }
        pub struct BlasStore{
            allocator: AllocationDataStore,
            gpu_mem: Allocation,
            acc_loader: ash::extensions::khr::AccelerationStructure,
            vertex_buffer: Buffer,
            index_buffer: Buffer,
            blas_buffer: Buffer,
            acceleration_structures: Vec<vk::AccelerationStructureKHR>,
        }

        enum BlasGeometeryType {
            Triangles(vk::GeometryFlagsKHR, vk::BuildAccelerationStructureFlagsKHR, *const c_void, *const c_void, *const c_void)
        }
        pub struct BlasOutline<V>{
            vertex_data: Vec<V>,
            vertex_format: vk::Format,
            index_data: Vec<u32>,
            transform: Option<vk::DeviceOrHostAddressConstKHR>,
            geo_type: BlasGeometeryType,
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
                    .buffer(blas_region.buffer)
                    .offset(blas_region.buffer_offset)
                    .size(blas_region.size)
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
        impl BlasStore{
            pub fn new<T: IEngineData, V: Clone>(engine: &T, cmd: vk::CommandBuffer, outlines: &[BlasOutline<V>]) -> (BlasStore, BlasStoreCreateRecipt){
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
                let mut cpu_mem = allocator.allocate(allocator.get_type(vk::MemoryPropertyFlags::HOST_COHERENT), copy_size + 1024*1024, 0 as *const c_void);

                let mut vertex_copy = cpu_mem.get_buffer(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC, vertex_size + 1024, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
                let mut index_copy = cpu_mem.get_buffer(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC, index_size + 1024, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);

                let vertex_copy_regions = vertex_copy.get_regions(&requested_vertex_data_regions, None);
                let index_copy_regions = index_copy.get_regions(&requested_index_data_regions, None);

                let mut vertex_buffer = gpu_mem.get_buffer(vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR | vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS, vertex_size, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
                let mut index_buffer = gpu_mem.get_buffer(vk::BufferUsageFlags::INDEX_BUFFER | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR | vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS, index_size, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
                let mut blas_buffer = gpu_mem.get_buffer(vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS, blas_size, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
                let mut scratch_buffer = gpu_mem.get_buffer(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS, scratch_size, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);

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
                
                
                
                let recipt = BlasStoreCreateRecipt{ cpu_mem, v_copy: vertex_copy, i_copy: index_copy, s_buffer: scratch_buffer };
                (BlasStore{ 
                    allocator, 
                    gpu_mem, 
                    vertex_buffer, 
                    index_buffer, 
                    blas_buffer,
                    acceleration_structures: acc_structs,
                    acc_loader, }, recipt)

            }
            pub fn new_immediate<T: IEngineData, V: Clone>(engine: &T, outlines: &[BlasOutline<V>]){
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
                unsafe{
                    engine.device().queue_submit(engine.queue_data().graphics.0, &submit, vk::Fence::null()).unwrap();
                    engine.device().queue_wait_idle(engine.queue_data().graphics.0).unwrap();
                }
                
            }
    
        }
        impl Drop for BlasStore{
            fn drop(&mut self) {
                unsafe{
                    for acc in self.acceleration_structures.iter(){
                        debug!("Destroying acceleration structure {:?}", *acc);
                        self.acc_loader.destroy_acceleration_structure(*acc, None);
                    }
                }
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

#[cfg(test)]
mod tests{
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
    
    use crate::{core::{self, memory}, traits::IEngineData};
    use ash::{self, vk};
    use std::ffi::c_void;
    use log::{self, debug};



    #[test]
    fn memory_round_trip_and_compute(){
        pretty_env_logger::init();
        let engine = core::WindowlessEngine::init(get_vulkan_validate());
        let allocator = memory::AllocationDataStore::new(&engine);
        let mut data:Vec<u32> = (0..100).collect();
        let check: Vec<u32> = data.iter().map(|v| v + 100).collect();
        let mut cpu_mem = allocator.allocate_typed::<u32>(allocator.get_type(vk::MemoryPropertyFlags::HOST_COHERENT), data.len()*3, 0 as *const c_void);
        let mut gpu_mem = allocator.allocate_typed::<u32>(allocator.get_type(vk::MemoryPropertyFlags::DEVICE_LOCAL), data.len(), 0 as *const c_void);
        let mut b1 = cpu_mem.get_buffer_typed::<u32>(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, data.len() * 2 + 10, None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
        let mut b2 = gpu_mem.get_buffer_typed::<u32>(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, data.len(), None, vk::BufferCreateFlags::empty(), 0 as *const c_void);
        let shader = core::Shader::new(&engine, String::from(r#"
        #version 460
        #extension GL_KHR_vulkan_glsl : enable

        layout(local_size_x = 1) in;

        layout(set = 0, binding = 0) buffer Data {
            uint[] values;
        } data;

        void main(){
            data.values[gl_GlobalInvocationID.x] += 100;
        }"#), shaderc::ShaderKind::Compute, "main", None);

        let descriptor_store = memory::DescriptorDataStore::new(&engine);
        let start_region = b1.get_region_typed::<u32>(data.len(), None);
        cpu_mem.copy_from_ram_typed(data.as_ptr(), data.len(), &start_region);
        let gpu_region = b2.get_region_typed::<u32>(data.len(), None);
        let end_region = b1.get_region_typed::<u32>(data.len(), None);
        let mut outline = core::memory::DescriptorSetOutline::new(vk::DescriptorSetLayoutCreateFlags::empty(), 0 as *const c_void, 0 as *const c_void);
        outline.add_binding(gpu_region.get_binding(vk::ShaderStageFlags::COMPUTE));
        let descriptor_stack = descriptor_store.get_descriptor_stack(&[outline], vk::DescriptorPoolCreateFlags::empty(), 0 as *const c_void, 0 as *const c_void);

        let compute = core::ComputePipeline::new(&engine, &[], &[descriptor_stack.get_set_layout(0)], shader.get_stage(vk::ShaderStageFlags::COMPUTE, &std::ffi::CString::new("main").unwrap()));

        let pool = core::CommandPool::new(&engine, vk::CommandPoolCreateInfo::builder().build());
        let cmd = crate::traits::ICommandPool::get_command_buffers(&pool, vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];

        unsafe{
            let cmds = vec![cmd];
            engine.device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().build()).unwrap();
            start_region.copy_to_region(cmd, &gpu_region);
            let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::NONE).dst_access_mask(vk::AccessFlags::MEMORY_READ).build();
            engine.device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);
            engine.device().cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, compute.get_pipeline());
            engine.device().cmd_bind_descriptor_sets(cmd, vk::PipelineBindPoint::COMPUTE, compute.get_layout(), 0, &vec![descriptor_stack.get_set(0)], &[]);
            engine.device().cmd_dispatch(cmd, data.len() as u32, 1, 1);
            engine.device().cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::TRANSFER,  vk::PipelineStageFlags::TRANSFER, vk::DependencyFlags::empty(), &[mem_barrier], &[], &[]);
            gpu_region.copy_to_region(cmd, &end_region);
            engine.device().end_command_buffer(cmd).unwrap();
            let submit = vk::SubmitInfo::builder().command_buffers(&cmds).build();
            engine.device().queue_submit(engine.queue_data().graphics.0, &[submit], vk::Fence::null()).unwrap();
            engine.device().queue_wait_idle(engine.queue_data().graphics.0).unwrap();
        }
        



        data = vec![100;data.len()];
        cpu_mem.copy_to_ram_typed(&end_region, data.len(), data.as_mut_ptr());
        debug!("{}", data.last().unwrap());
        assert!(check == data);
    }
    
}
