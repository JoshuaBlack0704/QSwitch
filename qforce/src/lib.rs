

#[allow(dead_code)]
pub mod enums{
    use ash;
    use ash::vk;
    use flume;
    use crate::core;

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
    pub enum DescriptorMessage{
        WriteInfoUpdate(core::DescriptorBindingReceipt, DescriptorInfoType),
    }
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
    pub trait IDescriptorEntryPoint {
        fn add_binding(&mut self, descriptor_type: vk::DescriptorType, stage: vk::ShaderStageFlags, info: enums::DescriptorInfoType, subscriber: flume::Sender<enums::DescriptorMessage>) -> (core::DescriptorBindingReceipt, flume::Sender<enums::DescriptorMessage>);
    }

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
    use ash::vk::DescriptorSetLayout;
    use shaderc;
    use ash;
    use ash::{vk, Entry};
    use std::{string::String, ffi::CStr, os::raw::c_char};
    use std::borrow::{Cow, Borrow, BorrowMut};
    use crate::traits::{self, IEngineData};
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
        channels: (flume::Sender<enums::MemoryMessage>, flume::Receiver<enums::MemoryMessage>),
        sectors: Vec<enums::MemorySector>,
        //Alloc Info, Cursor, Allocation Handle
        allocations: Vec<(vk::MemoryAllocateInfo, vk::DeviceSize, vk::DeviceMemory)>,
        sector_count: usize,
        physical_device_properties: vk::PhysicalDeviceProperties,
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
                let properties = engine.instance().get_physical_device_properties(engine.physical_device());
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
                
                Memory{ 
                    device, 
                    type_index: selected_type as u32, 
                    channels,
                    sectors: vec![],
                    allocations: vec![],
                    sector_count: 0,
                    physical_device_properties: properties,
                 }
            }


        }
        pub fn get_buffer(&mut self, mut c_info: vk::BufferCreateInfo) -> Buffer {
            let buffer: vk::Buffer;
            let channel = flume::unbounded();
            let mem_reqs: vk::MemoryRequirements;
            c_info.usage = c_info.usage | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST;
            unsafe {
                buffer = self.device.create_buffer(&c_info, None).unwrap();
                mem_reqs = self.device.get_buffer_memory_requirements(buffer);
            }
            debug!("New buffer memory reqs: {:?}", mem_reqs);
            let index = self.sectorize(&mem_reqs);
            let target_sector = self.sectors[index].borrow_mut();
            match target_sector {
                enums::MemorySector::Empty(allocation, index, offset, reqs) => {
                    unsafe{
                        self.device.bind_buffer_memory(buffer, self.allocations[*allocation].2, *offset).unwrap();
                    }
                    *target_sector = enums::MemorySector::Buffer(*allocation, *index, buffer, c_info, *offset, *reqs, channel.0.clone());
                    
                },
                _ => unimplemented!()
            }
            Buffer { device: self.device.clone(), channel, sector: target_sector.clone(), descriptor_channel: flume::unbounded(), descriptor_blocks: vec![], limits: self.physical_device_properties.limits }

        }
        pub fn get_image(&mut self, mut c_info: vk::ImageCreateInfo) -> Image{
            let image: vk::Image;
            let mem_reqs: vk::MemoryRequirements;
            let channel = flume::unbounded();
            c_info.usage = c_info.usage | vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST;
            unsafe {
                image = self.device.create_image(&c_info, None).unwrap();
                mem_reqs = self.device.get_image_memory_requirements(image);
            }
            debug!("New image memory reqs: {:?}", mem_reqs);
            let index = self.sectorize(&mem_reqs);
            let target_sector = self.sectors[index].borrow_mut();
            match target_sector {
                enums::MemorySector::Empty(allocation, index, offset, reqs) => {
                    unsafe {
                        self.device.bind_image_memory(image, self.allocations[*allocation].2, *offset).unwrap();
                    }
                    *target_sector = enums::MemorySector::Image(*allocation, *index, image, c_info, *offset, vk::ImageSubresourceLayers::builder().aspect_mask(vk::ImageAspectFlags::COLOR).mip_level(0).base_array_layer(0).layer_count(1).build(), *reqs, channel.0.clone());
                },
                _ => unimplemented!()
            }
            Image{ device: self.device.clone(), image, channel, sector: target_sector.clone() }
        }
        fn sectorize(&mut self, mem_reqs: &vk::MemoryRequirements) -> usize{
            debug!("Finding sector of size {}", mem_reqs.size);

            //We try to find a pre-existing sector to take over
            for (index,sector) in self.sectors.iter_mut().enumerate(){
                let extracted_data: enums::MemorySector;
                let extracted_channel: flume::Sender<enums::MemoryMessage>;
                match sector {
                    enums::MemorySector::Buffer(ai, si,_, _, o, reqs, c) => {extracted_data = enums::MemorySector::Empty(*ai, *si, *o, *reqs);extracted_channel = c.clone();},
                    enums::MemorySector::Image(ai, si, _, _, o, _, reqs, c) => {extracted_data = enums::MemorySector::Empty(*ai, *si,  *o, *reqs);extracted_channel = c.clone();},
                    _ => unimplemented!(),
                }
                match extracted_data {
                    enums::MemorySector::Empty(ai, si, o, r) => {
                        if extracted_channel.is_disconnected() && r.size >= mem_reqs.size && r.alignment == mem_reqs.alignment {
                            debug!("Using sector {} of size {}", si, r.size);
                            *sector = extracted_data;
                            debug!("Found pre-exsisting sector {} allocated on {} at {} with requirements {:?}", si, ai, o, r);
                            return index;
                        }
                    },
                    _ => unimplemented!()
                }
                
            }

            //Now we try to find an unused chunk of memory to take over
            for (index, (alloc_info, cursor, _)) in self.allocations.iter_mut().enumerate(){
                let offset = (*cursor / mem_reqs.alignment + 1) * mem_reqs.alignment;
                let remaining_size = alloc_info.allocation_size - offset;

                if remaining_size >= mem_reqs.size{
                    self.sectors.push(enums::MemorySector::Empty(index, self.sector_count, offset, mem_reqs.clone()));
                    self.sector_count += 1;
                    *cursor += mem_reqs.size;
                    debug!("Created sector {} on allocation {} at {} with requirements {:?}", self.sectors.len()-1, index, offset, mem_reqs);
                    return self.sectors.len() - 1;
                }
            }

            //If we get here we need to create a new allocation. Size is either 1MB or 2x the size of the request
            //1 MB
            let allocation_size:u64;
            if 1024*1024 <= mem_reqs.size{
                allocation_size = mem_reqs.size + 1024*1024;
            }
            else{
                allocation_size = 1024*1024;
            }
            let aloc_info = vk::MemoryAllocateInfo::builder().allocation_size(allocation_size).memory_type_index(self.type_index).build();
            let allocation: vk::DeviceMemory;
            unsafe{
                allocation = self.device.allocate_memory(&aloc_info, None).unwrap();
            }
            self.sectors.push(enums::MemorySector::Empty(self.allocations.len(), self.sector_count, 0, mem_reqs.clone()));
            self.sector_count += 1;
            self.allocations.push((aloc_info, mem_reqs.size, allocation));
            debug!("Created allocation {} with size {} and sector {}", self.allocations.len()-1, aloc_info.allocation_size, self.sectors.len()-1);
            self.sectors.len() -1


        }
        pub fn consolidate(&mut self, cmd: &vk::CommandBuffer){
            //We first process all messages in the message stream and apply the updates to the sectors
            for message in self.channels.1.try_iter(){
                match message {
                    enums::MemoryMessage::ImageInfoUpdate(si, c_info, layers) => {
                        match self.sectors.iter_mut().find(|sector| {
                            let found = match sector {
                                enums::MemorySector::Buffer(_, _si, _, _, _, _, _) => *_si == si,
                                enums::MemorySector::Image(_, _si, _, _, _, _, _, _) => *_si == si,
                                _ => unimplemented!()
                            };
                            found
                        }).unwrap() {
                            enums::MemorySector::Image(_, _, _, c, _, s, _, _) => {
                                 *s = layers;
                                 *c = c_info;
                                 debug!("Updates found for iamge sector {}", si);
                            },
                            _ => unimplemented!()
                        }
                    },
                    _ => todo!()
                }
            }
            //We must first build a new sector layout
            let mut cursor: vk::DeviceSize = 0;
            let mut sectors = Vec::with_capacity(self.sectors.len());
            for sector in self.sectors.iter(){
                match sector {
                    enums::MemorySector::Buffer(_, si, b, c, _, r, f) => {
                        if !f.is_disconnected() {
                            cursor = ((cursor / r.alignment) + 1) * r.alignment;
                            sectors.push(enums::MemorySector::Buffer(0, *si, *b, *c, cursor, *r, f.clone()));
                            cursor += r.size;
                            debug!("Sector {} accepted. Cursor is at {}", si, cursor);
                        }
                        else {
                            debug!("Sector {} disconnected", si);
                        }
                        
                    },
                    enums::MemorySector::Image(_, si, i, c, _, s, r, f) => {
                        if !f.is_disconnected(){
                            cursor = ((cursor / r.alignment) + 1) * r.alignment;
                            sectors.push(enums::MemorySector::Image(0, *si, *i, *c, cursor, *s, *r, f.clone()));
                            cursor += r.size;
                            debug!("Sector {} accepted. Cursor is at {}", si, cursor);
                        }
                        else {
                            debug!("Sector {} disconnected", si);
                        }
                    },
                    _ => unimplemented!()
                }
            }
            //Then we create an allocation to hold all sectors
            let allocation: vk::DeviceMemory;
            let aloc_info = vk::MemoryAllocateInfo::builder().allocation_size(cursor * 2).memory_type_index(self.type_index).build();
            unsafe{
                allocation = self.device.allocate_memory(&aloc_info, None).unwrap();
                debug!("New allocation of size {} created", aloc_info.allocation_size);
            }
            //Then we need to record the transfer operations
            //During this phase we also create the new buffer and image objects
            let mut old_data = Vec::with_capacity(sectors.len());
            for sector in sectors.iter_mut(){
                match sector {
                    enums::MemorySector::Buffer(_, si, b, c, o, _, _) => {
                        let target_buffer: vk::Buffer;
                        let copy = vk::BufferCopy::builder().dst_offset(0).src_offset(0).size(c.size).build();
                        unsafe {
                            target_buffer = self.device.create_buffer(c, None).unwrap();
                            self.device.bind_buffer_memory(target_buffer, allocation, *o).unwrap();
                            self.device.cmd_copy_buffer(*cmd, *b, target_buffer, &vec![copy]);
                        }
                        debug!("Recorded copy from buffer {:?} to new buffer {:?} for sector {}", *b, target_buffer, si);
                        old_data.push((Some(*b), None, None));
                        *b = target_buffer;
                    },
                    enums::MemorySector::Image(_, si, i, c, o, s, _, _) => {
                        let target_image: vk::Image;
                        let copy = vk::ImageCopy::builder().src_subresource(*s).dst_subresource(*s).src_offset(vk::Offset3D::builder().build()).dst_offset(vk::Offset3D::builder().build()).extent(c.extent).build();
                        unsafe{
                            target_image = self.device.create_image(c, None).unwrap();
                            self.device.bind_image_memory(target_image, allocation, *o).unwrap();
                            self.device.cmd_copy_image(*cmd, *i, c.initial_layout, target_image, c.initial_layout, &vec![copy]);
                        }
                        debug!("Recorded copy from image {:?} to new image {:?} for sector {}", *i, target_image, si);
                        old_data.push((None, Some(*i), None));
                        *i = target_image;
                    },
                    _ => unimplemented!()
                }
            }
            for old_allocation in self.allocations.iter(){
                old_data.push((None, None, Some(old_allocation.2)));
            }
            self.allocations = vec![(aloc_info, cursor, allocation)];

            //Now we send updates through channels
            for sector in self.sectors.iter(){
                match sector {
                    enums::MemorySector::Buffer(_, _, _, _, _, _, f) => {
                        f.send(enums::MemoryMessage::BindingUpdate(sector.clone())).unwrap();
                    },
                    enums::MemorySector::Image(_, _, _, _, _, _, _, f) => {
                        f.send(enums::MemoryMessage::BindingUpdate(sector.clone())).unwrap();
                    },
                    _ => unimplemented!()
                }
            }

        }
        pub fn copy_from_ram(&self, src: *const u8, byte_count: usize, target_sector: &enums::MemorySector, dst_offset: isize){
            let target_allocation: vk::DeviceMemory;
            let target_offset: isize;
            let target_index: usize;
            match target_sector {
                enums::MemorySector::Buffer(ai, si, _, _, o, _, _) => {
                    target_allocation = self.allocations[*ai].2;
                    target_offset = *o as isize + dst_offset;
                    target_index = *si;
                },
                enums::MemorySector::Image(ai, si, _, _, o, _, _, _) => {
                    target_allocation = self.allocations[*ai].2;
                    target_offset = *o as isize + dst_offset;
                    target_index = *si;
                },
                _ => unimplemented!()
            }



            let mapped_range = vk::MappedMemoryRange::builder()
                .memory(target_allocation)
                .offset(0)
                .size(vk::WHOLE_SIZE)
                .build();
    
            unsafe {
                let dst = (self.device.map_memory(target_allocation, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()).unwrap() as *mut u8).offset(target_offset);
                debug!("Copying {} bytes from {:?} to sector {} on allocation {:?} at {}", byte_count, src, target_index, target_allocation, target_offset);
                std::ptr::copy_nonoverlapping(src, dst, byte_count);
                self.device.flush_mapped_memory_ranges(&vec![mapped_range]).unwrap();
                self.device.unmap_memory(target_allocation);
            }
        }
        pub fn copy_to_ram(&self, dst: *mut u8, byte_count: usize, src_sector: &enums::MemorySector, _src_offset: isize){
            let src_allocation: vk::DeviceMemory;
            let src_offset: isize;
            let src_index: usize;
            match src_sector {
                enums::MemorySector::Buffer(ai, si, _, _, o, _, _) => {
                    src_allocation = self.allocations[*ai].2;
                    src_offset = *o as isize + _src_offset;
                    src_index = *si;
                },
                enums::MemorySector::Image(ai, si, _, _, o, _, _, _) => {
                    src_allocation = self.allocations[*ai].2;
                    src_offset = *o as isize + _src_offset;
                    src_index = *si;
                },
                _ => unimplemented!()
            }
    
            let mapped_range = vk::MappedMemoryRange::builder()
            .memory(src_allocation)
            .offset(0)
            .size(vk::WHOLE_SIZE)
            .build();
    
            unsafe {
                let src = (self.device.map_memory(src_allocation, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()).unwrap() as *const u8).offset(src_offset);
                self.device.invalidate_mapped_memory_ranges(&vec![mapped_range]).unwrap();
                debug!("Copying {} bytes to {:?} from sector {} on allocation {:?} at {}", byte_count, dst, src_index, src_allocation, _src_offset);
                std::ptr::copy_nonoverlapping(src, dst, byte_count);
                self.device.unmap_memory(src_allocation);
            }
    
        }
    }
    impl Drop for Memory{
        fn drop(&mut self) {
        debug!("Dropping Memory");
        unsafe{
            for (_,_,mem) in self.allocations.iter(){
                self.device.free_memory(*mem, None);
            }
        }
    }
    }
    pub struct Image{
        device: ash::Device,
        image: vk::Image,
        channel: (flume::Sender<enums::MemoryMessage>, flume::Receiver<enums::MemoryMessage>),
        sector: enums::MemorySector
    }
    impl Image{
    }
    pub struct Buffer{
        device: ash::Device,
        channel: (flume::Sender<enums::MemoryMessage>, flume::Receiver<enums::MemoryMessage>),
        sector: enums::MemorySector,
        descriptor_channel: (flume::Sender<enums::DescriptorMessage>, flume::Receiver<enums::DescriptorMessage>),
        descriptor_blocks: Vec<(vk::DeviceSize, vk::DeviceSize, vk::ShaderStageFlags, flume::Sender<enums::DescriptorMessage>)>,
        limits: vk::PhysicalDeviceLimits,
    }
    impl Buffer{
        pub fn get_sector(&mut self) -> &enums::MemorySector{
            for message in self.channel.1.try_iter(){
                match message {
                    enums::MemoryMessage::BindingUpdate(s) => {
                        match s {
                            enums::MemorySector::Buffer(_, _, _, _, _, _, _) => {
                                self.sector = s;
                                debug!("Binding update processed")
                            },
                            _ => unimplemented!()
                        }
                    },
                    _ => unimplemented!()
                }
            }

            &self.sector
         }
        pub fn get_buffer(&mut self) -> vk::Buffer{
            let buffer: vk::Buffer;
            match self.get_sector() {
                enums::MemorySector::Buffer(_, _, b, _, _, _, _) => {buffer = *b},
                _ => unimplemented!()
            }
            debug!("Buffer {:?} returned", buffer);
            buffer
         }
        pub fn transfer_from_buffer(&mut self, cmd: vk::CommandBuffer, src: &mut Buffer, src_offset: vk::DeviceSize, size: vk::DeviceSize, dst_offset: vk::DeviceSize){
            let copy = vk::BufferCopy::builder().src_offset(src_offset).dst_offset(dst_offset).size(size).build();
            unsafe{
                let self_buffer = self.get_buffer();
                self.device.cmd_copy_buffer(cmd, src.get_buffer(), self_buffer, &vec![copy]);
            }
         }
        pub fn transfer_to_buffer(&mut self, cmd: vk::CommandBuffer, dst: &mut Buffer, dst_offset: vk::DeviceSize, size: vk::DeviceSize, src_offset: vk::DeviceSize){
            let copy = vk::BufferCopy::builder().src_offset(src_offset).dst_offset(dst_offset).size(size).build();
            unsafe{
                let self_buffer = self.get_buffer();
                self.device.cmd_copy_buffer(cmd, self_buffer, dst.get_buffer(), &vec![copy]);
            }
         }
        pub fn add_descriptor_block<T: traits::IDescriptorEntryPoint>(&mut self, offset: vk::DeviceSize, range: vk::DeviceSize, stages: vk::ShaderStageFlags, descriptor_system: &mut T){
            
            match self.get_sector() {
                enums::MemorySector::Buffer(_, _, _, c, _, _, _) => {
                    if c.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER){
                        let alignment = self.limits.min_storage_buffer_offset_alignment;
                        let descriptor_storage_buffer_alignment_check = offset % alignment == 0;
                        debug!("Descriptor alignment requirement {}\n   Requriement met {}", alignment, descriptor_storage_buffer_alignment_check);
                        assert!(descriptor_storage_buffer_alignment_check);
                    }
                    else{
                        todo!()
                    }
                },
                _ => unimplemented!()
            }


            let (_, _) = descriptor_system
            .add_binding(
                self.get_descriptor_type(), 
                stages, 
            enums::DescriptorInfoType::Buffer(vk::DescriptorBufferInfo::builder().buffer(self.get_buffer()).offset(offset).range(range).build()),
            self.descriptor_channel.0.clone());
        }
        fn get_descriptor_type(&mut self) -> vk::DescriptorType{
            let d_type: vk::DescriptorType;
            match self.get_sector() {
                enums::MemorySector::Buffer(_, _, _, c, _, _, _) => {
                    let usage = c.usage;
                    if usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER){
                        d_type = vk::DescriptorType::STORAGE_BUFFER;
                    }
                    else {
                        unimplemented!();
                    }
                },
                _ => unimplemented!()
            }
            d_type
         }
         
    }
    impl Drop for Buffer{
        fn drop(&mut self) {
        match self.sector{
            enums::MemorySector::Buffer(_, _, b, _, _, _, _) => unsafe {debug!("Destroying Buffer {:?}", b); self.device.destroy_buffer(b, None);},
            _ => unimplemented!()
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
    
    #[derive(Clone)]
    pub struct DescriptorBindingReceipt{
        set_index: usize,
        binding_index: usize,
    }
    #[derive(Clone)]
    struct DescriptorSet{
        pub set: Option<vk::DescriptorSet>,
        pub layout: Option<vk::DescriptorSetLayout>,
        pub  bindings: Vec<(vk::DescriptorType, vk::ShaderStageFlags, enums::DescriptorInfoType, flume::Sender<enums::DescriptorMessage>)>
    }
    pub struct DescriptorSystem{
        device: ash::Device,
        pool: Option<vk::DescriptorPool>,
        sets: Vec<DescriptorSet>,
        active_set: usize,
        channel: (flume::Sender<enums::DescriptorMessage>, flume::Receiver<enums::DescriptorMessage>)
    }
    impl DescriptorSystem{
        pub fn new<T: IEngineData>(engine: &T) -> DescriptorSystem{
            DescriptorSystem { device: engine.device(), pool: None, sets: vec![], active_set: 0, channel: flume::unbounded() }
        }
        pub fn set_active_set(&mut self, index: usize){
            self.active_set = index;
        }
        pub fn create_new_set(&mut self) -> usize{
            let set = DescriptorSet{ set: None, layout: None, bindings: vec![] };
            self.sets.push(set);
            self.sets.len()-1
        }
        pub fn update(&mut self){

            match self.pool{
                Some(_) => {},
                None => {
                    debug!("Updating pool");
                    //First we need to produce all of the set layouts while keeping track of the desc types
                    let mut sizes: Vec<vk::DescriptorPoolSize> = vec![];
                    let mut layouts: Vec<vk::DescriptorSetLayout> = vec![];
                    for (index, set_data) in self.sets.iter_mut().enumerate(){
                        debug!("Analyzing set {}", index);
                        set_data.set = None;
                        let mut bindings: Vec<vk::DescriptorSetLayoutBinding> = vec![];
                        for (b_index, (descriptor_type, stages, _, f)) in set_data.bindings.iter().enumerate(){
                            //Here we add the size out of the disconnect check because we must ensure the pool can carry old layouts
                            match sizes.iter_mut().find(|size| size.ty == *descriptor_type) {
                                Some(s) => {
                                    s.descriptor_count += 1;
                                    debug!("Type {:?} count set to {}", s.ty, s.descriptor_count);
                                },
                                None => {
                                    sizes.push(vk::DescriptorPoolSize::builder().ty(*descriptor_type).descriptor_count(1).build());
                                    debug!("New type added to sizes {:?}", *descriptor_type);
                                },
                            }
                            if !f.is_disconnected(){
                                //Here we gen a new binding
                                let binding = vk::DescriptorSetLayoutBinding::builder()
                                .binding(bindings.len() as u32)
                                .descriptor_type(*descriptor_type)
                                .descriptor_count(1)
                                .stage_flags(*stages)
                                .build();
                                debug!("Binding slot {} of set {} is active generated binding {:?}", b_index, index, binding);
                                bindings.push(binding);
                            }
                        }


                        match set_data.layout {
                            Some(l) => {
                                layouts.push(l)
                            },
                            None => {
                                let c_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(bindings.as_slice()).build();
                                unsafe{
                                    set_data.layout = Some(self.device.create_descriptor_set_layout(&c_info, None).unwrap());
                                }
                                debug!("Created set layout {:?} for set {}", set_data.layout, index);
                                layouts.push(set_data.layout.unwrap());
                            },
                        }
                    }
                    //Then we need to create the pool
                    let c_info = vk::DescriptorPoolCreateInfo::builder()
                    .max_sets(self.sets.len() as u32)
                    .pool_sizes(sizes.as_slice())
                    .build();
                    unsafe{
                        self.pool = Some(self.device.create_descriptor_pool(&c_info, None).unwrap());
                        debug!("Generated new pool {:?}", self.pool);
                    }
                    //Then we need to allocate the sets
                    let a_info = vk::DescriptorSetAllocateInfo::builder().descriptor_pool(self.pool.unwrap()).set_layouts(layouts.as_slice()).build();
                    unsafe{
                        for (index, set) in self.device.allocate_descriptor_sets(&a_info).unwrap(). iter().enumerate(){
                            self.sets[index].set = Some(*set);
                        }
                    }
                    debug!("Allocated {} sets", a_info.descriptor_set_count);
                    //Then we need to write all sets
                    self.rewrite_sets();
                },
            }


            //We need to read the write update messages and apply the data changes and write into the sets
            let mut writes_needed = vec![];
            for message in self.channel.1.try_iter(){
                match message {
                    enums::DescriptorMessage::WriteInfoUpdate(r, t) => {
                        let target_set = self.sets.get_mut(r.set_index).unwrap();
                        let (_,_,i,_) = target_set.bindings.get_mut(r.binding_index).unwrap();
                        *i = t;
                        writes_needed.push(target_set.clone());
                        debug!("Write update read for binding {} on set {}", r.binding_index, r.set_index);
                    },
                }
            }
            self.write_sets(writes_needed.as_slice());
        }
        pub fn get_set_layout(&mut self, set_index: usize) -> DescriptorSetLayout {
            self.update();
            let target_set = self.sets.get_mut(set_index).unwrap();
            let layout = match target_set.layout {
                Some(l) => l,
                None => panic!(),
            };
            layout
        }
        pub fn get_set(&mut self, set_index: usize) -> vk::DescriptorSet{
            self.update();
            let target_set = self.sets.get_mut(set_index).unwrap();
            let set = match target_set.set {
                Some(s) => s,
                None => panic!(),
            };
            set
        }
        #[doc = "Writes all sets"]
        fn rewrite_sets(&mut self){
            let mut b_infos = vec![];
            let mut i_infos = vec![];
            let mut writes: Vec<vk::WriteDescriptorSet> = vec![];
                    for (index, set_data) in self.sets.iter().enumerate(){
                        let mut binding_count = 0;
                        writes.append(
                            &mut set_data.bindings.iter()
                            .enumerate()
                            .filter(|(_, (_,_,_,f))| !f.is_disconnected())
                            .map(|(b_index, (t,_,info,_))| {
                                
                                let write = match info {
                                    enums::DescriptorInfoType::Image(i) => {
                                        i_infos.push(vec![*i]);
                                        let write = vk::WriteDescriptorSet::builder()
                                            .dst_set(set_data.set.unwrap())
                                            .dst_binding(binding_count as  u32)
                                            .descriptor_type(*t)
                                            .image_info(i_infos.last().unwrap().as_slice())
                                            .build();
                                        debug!("Generted write {:?} on set {} for binding {}, target is image view {:?}", write, index, b_index, i.image_view);
                                        binding_count += 1;
                                        write

                                    },
                                    enums::DescriptorInfoType::Buffer(b) => {
                                         b_infos.push(vec![*b]);
                                         let write = vk::WriteDescriptorSet::builder()
                                            .dst_set(set_data.set.unwrap())
                                            .dst_binding(binding_count as  u32)
                                            .descriptor_type(*t)
                                            .buffer_info(b_infos.last().unwrap().as_slice())
                                            .build();
                                        debug!("Generted write {:?} on set {} for binding {}, target is buffer {:?}", write, index, b_index, b.buffer);
                                        binding_count += 1;
                                        write
                                        },
                                        
                                };
                                write
                            })
                            .collect()
                        );
                    }
                    unsafe{
                        self.device.update_descriptor_sets(writes.as_slice(), &[]);
                        debug!("Made {} Descriptor Set Writes", writes.len());
                    }
        }
        fn write_sets(&mut self, sets: &[DescriptorSet]){
            let mut b_infos = vec![];
            let mut i_infos = vec![];
            let mut writes: Vec<vk::WriteDescriptorSet> = vec![];
                    for (index, set_data) in sets.iter().enumerate(){
                        let mut binding_count = 0;
                        writes.append(
                            &mut set_data.bindings.iter()
                            .enumerate()
                            .filter(|(_, (_,_,_,f))| !f.is_disconnected())
                            .map(|(b_index, (t,_,info,_))| {
                                
                                let write = match info {
                                    enums::DescriptorInfoType::Image(i) => {
                                        i_infos.push(vec![*i]);
                                        let write = vk::WriteDescriptorSet::builder()
                                            .dst_set(set_data.set.unwrap())
                                            .dst_binding(binding_count as  u32)
                                            .descriptor_type(*t)
                                            .image_info(i_infos.last().unwrap().as_slice())
                                            .build();
                                        debug!("Generted write {:?} on set {} for binding {}, target is image view {:?}", write, index, b_index, i.image_view);
                                        binding_count += 1;
                                        write

                                    },
                                    enums::DescriptorInfoType::Buffer(b) => {
                                         b_infos.push(vec![*b]);
                                         let write = vk::WriteDescriptorSet::builder()
                                            .dst_set(set_data.set.unwrap())
                                            .dst_binding(binding_count as  u32)
                                            .descriptor_type(*t)
                                            .buffer_info(b_infos.last().unwrap().as_slice())
                                            .build();
                                        debug!("Generted write {:?} on set {} for binding {}, target is buffer {:?}", write, index, b_index, b.buffer);
                                        binding_count += 1;
                                        write
                                        },
                                        
                                };
                                write
                            })
                            .collect()
                        );
                    }
            if writes.len() > 0{
                unsafe{
                    self.device.update_descriptor_sets(writes.as_slice(), &[]);
                    debug!("Made {} Descriptor Set Writes", writes.len());
                }
            }
        }
    }
    impl traits::IDescriptorEntryPoint for DescriptorSystem {
        fn add_binding(&mut self, descriptor_type: vk::DescriptorType, stage: vk::ShaderStageFlags, info: enums::DescriptorInfoType, subscriber: flume::Sender<enums::DescriptorMessage>) -> (self::DescriptorBindingReceipt, flume::Sender<enums::DescriptorMessage>) {
            let target_set = self.sets.get_mut(self.active_set).unwrap();
            let mut binding_receipt = DescriptorBindingReceipt{ set_index: self.active_set, binding_index: target_set.bindings.len() };
            let mut empty_slot: Option<usize> = None;
            for (index, (_,_,_,f)) in target_set.bindings.iter().enumerate(){
                if f.is_disconnected(){
                    empty_slot = Some(index);
                    break;
                }
            }
            match empty_slot {
                Some(i) => {
                    debug!("Binding slot {} on set {} used for new binding", i, self.active_set);
                    binding_receipt.binding_index = i;
                    target_set.bindings[i] = (descriptor_type, stage, info, subscriber);
                },
                None => {
                    debug!("Adding new binding slot for set {}", self.active_set);
                    target_set.bindings.push((descriptor_type, stage, info, subscriber));
                },
            }
            match target_set.layout {
                Some(l) => {
                    unsafe{
                        debug!("Destroying Descriptor Set Layout {:?}", l);
                        self.device.destroy_descriptor_set_layout(l, None);
                        target_set.layout = None;
                    }
                },
                None => {},
            }
            match self.pool {
                Some(p) => {
                    unsafe{
                        debug!("Destroying Descriptor Pool {:?}", p);
                        self.device.destroy_descriptor_pool(p, None);
                        self.pool = None;
                    }
                },
                None => {},
            }
            (binding_receipt, self.channel.0.clone())
        }
    }
    impl Drop for DescriptorSystem{
        fn drop(&mut self) {
            match self.pool {
                Some(p) => unsafe {debug!("Destroying Descriptor Pool {:?}", p); self.device.destroy_descriptor_pool(p, None);},
                None => {},
            }
            for layout in self.sets.iter().map(|set| set.layout){
            match layout {
                Some(l) => unsafe {debug!("Destroying Descriptor Set Layout {:?}", l) ;self.device.destroy_descriptor_set_layout(l, None);},
                None => {},
            } 
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


    struct AcceleratedObject<V: traits::IVulkanVertex>{
        vertices: Vec<V>,
        indecies: Vec<u32>,
        //Offset, size
        vertex_buffer_data: (vk::DeviceSize, vk::DeviceSize),
        //Offset, size
        index_buffer_data: (vk::DeviceSize, vk::DeviceSize),
        blas_buffer_data: (vk::DeviceSize, vk::DeviceSize),
        blas: vk::AccelerationStructureKHR,
        shader_group: (Option<vk::ShaderModule>, Option<vk::ShaderModule>, Option<vk::ShaderModule>)
    }

    pub struct ObjectStore<C: traits::ICommandPool, V: traits::IVulkanVertex>{
        device: ash::Device,
        cmd: (vk::CommandBuffer, C),
        vertex_buffer: Buffer,
        index_buffer: Buffer,
        blas_buffer: Buffer,
        memory: Memory,
        objects: Vec<AcceleratedObject<V>>,
    }
    impl<C: traits::ICommandPool,V: traits::IVulkanVertex> ObjectStore<C,V>{
        pub fn new<T: IEngineData>(engine: &T){

        }
    }

}
