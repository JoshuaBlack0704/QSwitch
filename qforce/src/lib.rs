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
                            //| vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
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
    use crate::{init::{self,IEngine,PhysicalDevicePropertiesStore, Engine, QueueStore}, IDisposable, descriptor::DescriptorWriteType, command::CommandPool, sync::Fence};

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
        queue_store: QueueStore
    }
    pub struct GeneralMemoryProfiles{
        pub general_device_index: usize,
        pub general_host_index: usize,
        pub storage_buffer_index: usize,
        pub uniform_buffer_index: usize,
        pub device_storage: AllocatorProfileStack,
        pub host_storage: AllocatorProfileStack,
        pub device_uniform: AllocatorProfileStack,
        pub host_uniform: AllocatorProfileStack,
    }
    

    impl GeneralMemoryProfiles{
        pub fn new(allocator: &mut Allocator, min_buffer_size: u64, min_allocation_size: u64) -> GeneralMemoryProfiles {
            let buffer_options = [CreateBufferOptions::MinimumSize(min_buffer_size)];
            let allocation_options = [CreateAllocationOptions::MinimumSize(min_allocation_size)];
            
            let gpu_mem = AllocatorProfileType::Allocation(AllocationAllocatorProfile::new(vk::MemoryPropertyFlags::DEVICE_LOCAL, &allocation_options));
            let cpu_mem = AllocatorProfileType::Allocation(AllocationAllocatorProfile::new(vk::MemoryPropertyFlags::HOST_COHERENT, &allocation_options));
            let storage_buffer = AllocatorProfileType::Buffer(BufferAllocatorProfile::new(vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, &buffer_options));
            let uniform_buffer = AllocatorProfileType::Buffer(BufferAllocatorProfile::new(vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, &buffer_options));
            
            let gpu_mem = allocator.add_profile(gpu_mem);
            let cpu_mem = allocator.add_profile(cpu_mem);
            let storage_buffer = allocator.add_profile(storage_buffer);
            let uniform_buffer = allocator.add_profile(uniform_buffer);
            
            let device_storage = AllocatorProfileStack::TargetBuffer(gpu_mem, storage_buffer);
            let host_storage = AllocatorProfileStack::TargetBuffer(cpu_mem, storage_buffer);
            let device_uniform = AllocatorProfileStack::TargetBuffer(gpu_mem, uniform_buffer);
            let host_uniform = AllocatorProfileStack::TargetBuffer(cpu_mem, uniform_buffer);
            
            GeneralMemoryProfiles{ 
                general_device_index: gpu_mem,
                general_host_index: cpu_mem,
                storage_buffer_index: storage_buffer,
                uniform_buffer_index: uniform_buffer,
                device_storage,
                host_storage,
                device_uniform,
                host_uniform }
        }
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
                queue_store: engine.get_queue_store(),
            }
        }
        pub fn get_buffer_region_from_slice<O: Clone>(&mut self, stage_profile: &AllocatorProfileStack, dst_profile: &AllocatorProfileStack, data: &[O], alignment: &AlignmentType, options: &[CreateBufferRegionOptions]) -> BufferRegion {
            let queue_data = self.queue_store.get_queue(vk::QueueFlags::TRANSFER).unwrap();
            let stage = self.get_buffer_region_from_template(stage_profile, data, &AlignmentType::Free, &[]);
            let dst = self.get_buffer_region_from_template(dst_profile, data, alignment, options);
            
            self.copy_from_ram_slice(data, &stage);
            
            let pool = CommandPool::new_raw(&self.device, vk::CommandPoolCreateInfo::builder()
                .queue_family_index(queue_data.1)
                .build()
            );
            let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder()
                .command_buffer_count(1)
                .build()  
            )[0];
            
            
            unsafe{
                self.device.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                    .build()
                ).expect("Could not begin command buffer");
                stage.copy_to_region(cmd, &dst);
                self.device.end_command_buffer(cmd).expect("Could not end command buffer");
                
                let fence = Fence::new_raw(&self.device, false);
                let cmd = [cmd];
                let submit = [vk::SubmitInfo::builder()
                .command_buffers(&cmd)
                .build()];
                self.device.queue_submit(queue_data.0, &submit, fence.get_fence()).expect("Could not submit queue");
                fence.wait();
            }
            
            dst
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
        pub fn get_buffer_region_from_template<O>(&mut self, profile: &AllocatorProfileStack, slice: &[O], alignment: &AlignmentType, options: &[CreateBufferRegionOptions]) -> BufferRegion {
            self.get_buffer_region::<O>(profile, slice.len(), alignment, options)
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
            if dst.home_block.size == 0{
                debug!("Aborting copy of no data to buffer {:?}", dst.buffer);
                return;
            }
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
                                size, 
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
                    if *a == 1 || self.start_offset == 0 || self.start_offset % *a == 0{
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
                    if *a == 1 || self.allocation_offset == 0 || self.allocation_offset % *a == 0{
                        (self.allocation_offset, self.buffer_offset, self.size)
                    }
                    else {
                        let allocation_offset = ((self.allocation_offset/ *a + 1) * *a);
                        let forward_delta = allocation_offset - self.allocation_offset;
                        let buffer_offset = self.buffer_offset + forward_delta;

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
                    if *a == 1 || self.buffer_offset == 0 || self.buffer_offset & *a == 0{
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
                    else if self.c_info.usage.contains(vk::BufferUsageFlags::UNIFORM_BUFFER)
                    {
                        AlignmentType::User(self.properties.pd_properties.limits.min_uniform_buffer_offset_alignment)
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
            if self.home_block.size == 0{
                debug!("Aborted copy to region of no size from buffer {:?}", self.buffer);
                return;
            }
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
        pub fn get_device_address(&self) -> u64 {
            let address_info = vk::BufferDeviceAddressInfo::builder()
            .buffer(self.buffer);
            let address = unsafe{self.device.get_buffer_device_address(&address_info)} + self.home_block.buffer_offset;
            address
        }
        pub fn get_buffer(&self) -> vk::Buffer {
            self.buffer
        }
        pub fn get_buffer_offset(&self) -> u64 {
            self.home_block.buffer_offset
        }
        pub fn get_allocation_offset(&self) -> u64 {
            self.home_block.allocation_offset
        }
        pub fn get_size(&self) -> u64 {
            self.home_block.size 
        }
        pub fn get_write(&self) -> DescriptorWriteType {
            DescriptorWriteType::Buffer(
            [vk::DescriptorBufferInfo::builder()
            .buffer(self.buffer)
            .offset(self.home_block.buffer_offset)
            .range(self.home_block.size)
            .build()])
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
        pub fn get_view(&self) -> vk::ImageView {
            self.view
        }
        pub fn get_write(&self, sampler: Option<vk::Sampler>) -> DescriptorWriteType {
            let mut write_builder = vk::DescriptorImageInfo::builder()
            .image_layout(self.layout)
            .image_view(self.view);
            match sampler {
                Some(s) => write_builder = write_builder.sampler(s),
                None => {},
            }
            DescriptorWriteType::Image([write_builder.build()])
        }
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
            .subresource_range(self.c_info.subresource_range)
            .src_access_mask(src_access)
            .dst_access_mask(dst_access)
            .build();
            let old_layout = self.layout;
            self.layout = new_layout;
            (transition, old_layout)
        }   
        pub fn internal_transition<T:IEngine>(&mut self, engine:&T, new_layout: vk::ImageLayout) -> vk::ImageLayout {
            let queue_data = engine.get_queue_store().get_queue(vk::QueueFlags::GRAPHICS).unwrap();
            let pool = CommandPool::new(engine, vk::CommandPoolCreateInfo::builder().queue_family_index(queue_data.1).build());
            let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];
            
            unsafe{
                let (transition, old_layout)= self.transition(vk::AccessFlags::NONE, vk::AccessFlags::NONE, new_layout);
                let transition = [transition];
                self.device.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).expect("Could not begin command buffer");
                self.device.cmd_pipeline_barrier(cmd, vk::PipelineStageFlags::ALL_COMMANDS, vk::PipelineStageFlags::ALL_COMMANDS, vk::DependencyFlags::empty(), &[], &[], &transition);
                self.device.end_command_buffer(cmd).expect("Could not end command buffer");
                
                let cmd = [cmd];
                let submit = [vk::SubmitInfo::builder()
                .command_buffers(&cmd)
                .build()];
                let fence = Fence::new(engine, false);
                self.device.queue_submit(queue_data.0, &submit, fence.get_fence());
                fence.wait();
                old_layout
            }
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

    pub struct DescriptorStack{
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


    impl DescriptorStack {
        pub fn new<T:IEngine>(engine: &T) -> DescriptorStack {
            DescriptorStack{ device: engine.get_device(),
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
                        let mut write = vk::WriteDescriptorSet::builder()
                        .dst_set(self.set)
                        .descriptor_type(d_type)
                        .dst_binding(*binding as u32)
                        .dst_array_element(*start_array_index as u32)
                        .push_next(a)
                        .build();
                        write.descriptor_count = 1;
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
    impl IDisposable for DescriptorStack{
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
    impl Drop for DescriptorStack{
        fn drop(&mut self) {
            self.dispose();
    }
    }

}
#[allow(dead_code, unused)]
pub mod command{
    use ash::vk;
    use log::debug;

    use crate::{init::IEngine, IDisposable};

    pub struct CommandPool{
        device: ash::Device,
        command_pool: ash::vk::CommandPool,
        c_info: vk::CommandPoolCreateInfo,
        disposed: bool,
    }
    impl CommandPool{
        pub fn new<T: IEngine>(engine: &T, c_info: ash::vk::CommandPoolCreateInfo) -> CommandPool {
    
            unsafe {
                let command_pool = engine.get_device().create_command_pool(&c_info, None).unwrap();
                CommandPool{
                    device: engine.get_device(),
                    command_pool,
                    c_info,
                    disposed: false,
                }
            }
    
        }
        pub fn new_raw(device: &ash::Device, c_info: ash::vk::CommandPoolCreateInfo) -> CommandPool {
            unsafe {
                let command_pool = device.create_command_pool(&c_info, None).unwrap();
                CommandPool{
                    device: device.clone(),
                    command_pool,
                    c_info,
                    disposed: false,
                }
            }
            
        }
        pub fn get_command_buffers(&self, mut a_info: vk::CommandBufferAllocateInfo) -> Vec<vk::CommandBuffer> {
            a_info.command_pool = self.command_pool;
            unsafe {
                self.device.allocate_command_buffers(&a_info).unwrap()
            }
        }
        pub fn reset(&self){
            unsafe {
                self.device.reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty()).unwrap();
            }
        }
    }
    impl IDisposable for CommandPool{
        fn dispose(&mut self) {
            if !self.disposed{
                self.disposed = true;
                debug!("Destroying command pool {:?}", self.command_pool);
                unsafe{
                    self.device.destroy_command_pool(self.command_pool, None);
                }
            }
    }
    }
    impl Drop for CommandPool {
        fn drop(&mut self) {
            self.dispose();
        }
    }
    
}
#[allow(dead_code, unused)]
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
        pub fn new_raw(device: &ash::Device, start_signaled: bool) -> Fence {
            let fence;
            let c_info;
            if start_signaled{
                c_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED).build();
            }
            else {
                c_info = vk::FenceCreateInfo::builder().build();
            }

            unsafe{
                fence = device.create_fence(&c_info, None).expect("Could not create fence");
            }
            debug!("Created fence {:?}", fence);
            Fence{ device: device.clone(), fence, disposed: false }
            
        }
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
                debug!("Waiting on fence {:?}", self.fence);
                let fence = [self.fence];
                self.device.wait_for_fences(&fence, true, u64::max_value()).expect("Could not wait on fence");
            }
        }
        pub fn wait_reset(&self){
            self.wait();
            unsafe{
                let fence = [self.fence];
                self.device.reset_fences(&fence).expect("Could not reset fence");
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
    
    pub struct SyncStageOutline{
        
    }
    
    pub struct SyncSystem{
        device: ash::Device,
            
    }
}
#[allow(dead_code,unused)]
pub mod ray_tracing{
    use std::mem::size_of;

    use ash::vk;
    use log::debug;

    use crate::{memory::{BufferRegion, Allocator, AllocatorProfileStack, AlignmentType, CreateBufferOptions, AllocatorProfileType, BufferAllocatorProfile, AllocatorResourceType, AllocationAllocatorProfile, CreateAllocationOptions, self}, command::CommandPool, init::{IEngine, PhysicalDevicePropertiesStore}, sync::Fence, IDisposable, descriptor::DescriptorWriteType};

    pub struct ShaderTable{
       pub ray_gen: Vec<vk::PipelineShaderStageCreateInfo>,
       pub hit_groups: Vec<(vk::RayTracingShaderGroupTypeKHR, (Option<vk::PipelineShaderStageCreateInfo>, Option<vk::PipelineShaderStageCreateInfo>, Option<vk::PipelineShaderStageCreateInfo>))>,
       pub misses: Vec<vk::PipelineShaderStageCreateInfo>,
        
    }
    pub struct RayTacingPipeline{
        device: ash::Device,
        ray_loader: ash::extensions::khr::RayTracingPipeline,
        pipeline_layout: vk::PipelineLayout,
        pipeline: vk::Pipeline,
        ray_gen_region: BufferRegion,
        hit_groups_region: BufferRegion,
        misses_region: BufferRegion,
        pub sbt_addresses: (vk::StridedDeviceAddressRegionKHR, vk::StridedDeviceAddressRegionKHR, vk::StridedDeviceAddressRegionKHR),
        disposed: bool,
    }
    #[derive(Clone)]
    pub struct RayTracingMemoryProfiles{
        properties: PhysicalDevicePropertiesStore,
        staging: AllocatorProfileStack, 
        vertex: AllocatorProfileStack, 
        index: AllocatorProfileStack, 
        acc_struct: AllocatorProfileStack, 
        instance_data: AllocatorProfileStack,
        scratch: AllocatorProfileStack, 
        shader_table: AllocatorProfileStack,
    }
    pub struct TriangleObjectGeometry{
        vertex_buffer: BufferRegion,
        index_buffer: BufferRegion,
        geometry_info: vk::AccelerationStructureGeometryKHR,
        primative_count: u32,
        shader_data: ObjectShaderData,
    }
    #[derive(Clone)]
    pub struct ObjectShaderData{
        vertex_address: u64,
        index_address: u64,
    }
    pub struct Blas{
        device: ash::Device,
        acc_loader: ash::extensions::khr::AccelerationStructure,
        profiles: RayTracingMemoryProfiles,
        blas_region: BufferRegion,
        blas: vk::AccelerationStructureKHR,
        device_address: vk::AccelerationStructureReferenceKHR,
        disposed: bool,
    }
    pub struct BlasObjectOutline{
        geometry: vk::AccelerationStructureGeometryKHR,
        primative_count: u32,
        max_primative_count: u32
    }
    pub struct Tlas{
        device: ash::Device,
        acc_loader: ash::extensions::khr::AccelerationStructure,
        profiles: RayTracingMemoryProfiles,
        tlas: [vk::AccelerationStructureKHR;1],
        disposed: bool
    }
    pub struct TlasInstanceOutline{
       pub instance_data: vk::DeviceOrHostAddressConstKHR,
       pub instance_count: u32,
       pub instance_count_overkill: u32,
       pub array_of_pointers: bool,
    }
    
    impl RayTacingPipeline{
        pub fn new<T:IEngine>(engine: &T, sbt: &ShaderTable, profiles: &RayTracingMemoryProfiles, allocator: &mut Allocator, set_layouts: &[vk::DescriptorSetLayout], push_constant_ranges: &[vk::PushConstantRange]) -> RayTacingPipeline {
            let device = engine.get_device();
            let ray_loader = ash::extensions::khr::RayTracingPipeline::new(&engine.get_instance(), &device);
            let properties = engine.get_property_store();
            
            let mut stages = vec![];
            let mut groups = vec![];
            
            for ray_gen in sbt.ray_gen.iter(){
                debug!("Adding ray gen shader at sbt index {:?}", stages.len());
                let group = vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(stages.len() as u32)
                .closest_hit_shader(u32::MAX)
                .any_hit_shader(u32::MAX)
                .intersection_shader(u32::MAX)
                .build();
                stages.push(*ray_gen);
                groups.push(group);
            }
            for miss in sbt.misses.iter(){
                debug!("Adding miss shader at sbt index {:?}", stages.len());
                let group = vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(vk::RayTracingShaderGroupTypeKHR::GENERAL)
                .general_shader(stages.len() as u32)
                .closest_hit_shader(u32::MAX)
                .any_hit_shader(u32::MAX)
                .intersection_shader(u32::MAX)
                .build();
                stages.push(*miss);
                groups.push(group);
            }
            for (geo_hit_type, (closest_hit, any_hit, intersection)) in sbt.hit_groups.iter(){
                let mut group_builder = vk::RayTracingShaderGroupCreateInfoKHR::builder()
                .ty(*geo_hit_type);
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
            .max_pipeline_ray_recursion_depth(1)
            .layout(layout)
            .build()];
            let pipeline = unsafe{ray_loader.create_ray_tracing_pipelines(
                vk::DeferredOperationKHR::null(), 
                vk::PipelineCache::null(), 
                &c_info, 
                None).expect("Could not create ray tracing pipeline")[0]};
            debug!("Built ray tracing pipeline {:?}", layout);
            
            let handle_data;

            let shaders = unsafe {
                handle_data = ray_loader.get_ray_tracing_shader_group_handles(pipeline, 0, groups.len() as u32, groups.len() * properties.pd_raytracing_properties.shader_group_handle_size as usize).expect("Could not get shader handles")
            };

            let handle_size = properties.pd_raytracing_properties.shader_group_handle_size;
            let mut aligned_handle_size = handle_size;
            let shader_alignment = properties.pd_raytracing_properties.shader_group_handle_alignment;
            let shader_base_alignment = properties.pd_raytracing_properties.shader_group_base_alignment;
            
            while aligned_handle_size % shader_alignment != 0 {
                aligned_handle_size += 1;
            }
            
            let aligned_handle_size_diff = aligned_handle_size - handle_size;
            let mut ray_gen_handles = vec![];
            let mut miss_handles = vec![];
            let mut hit_handles = vec![];
            for stage_index in 0..stages.len(){
                let handle_cursor = stage_index * handle_size as usize;
                let handle = &handle_data[handle_cursor..handle_cursor+handle_size as usize];
                if stage_index < sbt.ray_gen.len(){
                    ray_gen_handles.extend_from_slice(handle);
                    if aligned_handle_size_diff > 0{
                        ray_gen_handles.resize_with(ray_gen_handles.len() + aligned_handle_size_diff as usize, || 0);
                    }
                    
                }   
                else if stage_index < sbt.ray_gen.len() + sbt.misses.len(){
                    miss_handles.extend_from_slice(handle);
                    if aligned_handle_size_diff > 0{
                        miss_handles.resize_with(miss_handles.len() + aligned_handle_size_diff as usize, || 0);
                    }
                    
                }   
                else{
                    hit_handles.extend_from_slice(handle);
                    if aligned_handle_size_diff > 0{
                        hit_handles.resize_with(hit_handles.len() + aligned_handle_size_diff as usize, || 0);
                    }
                    
                }          
            }

            let ray_gen_region = allocator.get_buffer_region_from_template(&profiles.shader_table, &ray_gen_handles, &AlignmentType::Allocation(shader_base_alignment as u64), &[]);
            let miss_region = allocator.get_buffer_region_from_template(&profiles.shader_table, &miss_handles, &AlignmentType::Allocation(shader_base_alignment as u64), &[]);
            let hit_region = allocator.get_buffer_region_from_template(&profiles.shader_table, &hit_handles, &AlignmentType::Allocation(shader_base_alignment as u64), &[]);
            
            let ray_gen_stage = allocator.get_buffer_region_from_template(&profiles.staging, &ray_gen_handles, &AlignmentType::Free, &[]);
            let miss_stage = allocator.get_buffer_region_from_template(&profiles.staging, &miss_handles, &AlignmentType::Free, &[]);
            let hit_stage = allocator.get_buffer_region_from_template(&profiles.staging, &hit_handles, &AlignmentType::Free, &[]);
            
            allocator.copy_from_ram_slice(&ray_gen_handles, &ray_gen_stage);
            allocator.copy_from_ram_slice(&miss_handles, &miss_stage);
            allocator.copy_from_ram_slice(&hit_handles, &hit_stage);
            
            let pool = CommandPool::new(engine, vk::CommandPoolCreateInfo::builder().queue_family_index(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().1).build());
            let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];
            
            
            let fence = Fence::new(engine, false);
            unsafe{
                device.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).expect("Could not begin command buffer");
                ray_gen_stage.copy_to_region(cmd, &ray_gen_region);
                miss_stage.copy_to_region(cmd, &miss_region);
                hit_stage.copy_to_region(cmd, &hit_region);
                device.end_command_buffer(cmd).expect("Could not end command buffer");
                
                
                let cmds = [cmd];
                let submit = [vk::SubmitInfo::builder()
                .command_buffers(&cmds)
                .build()];
                
                device.queue_submit(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().0, &submit, fence.get_fence()).expect("Could not submit queue");
                fence.wait_reset();
            }           
            
            let ray_gen_address = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(ray_gen_region.get_device_address())
            .size(ray_gen_region.get_size())
            .stride(aligned_handle_size as u64)
            .build();
            let miss_address = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(miss_region.get_device_address())
            .size(miss_region.get_size())
            .stride(aligned_handle_size as u64)
            .build();
            let hit_address = vk::StridedDeviceAddressRegionKHR::builder()
            .device_address(hit_region.get_device_address())
            .size(hit_region.get_size())
            .stride(aligned_handle_size as u64)
            .build();
            RayTacingPipeline{ device,
                ray_loader,
                pipeline_layout: layout,
                pipeline,
                ray_gen_region,
                hit_groups_region: hit_region,
                misses_region: miss_region,
                sbt_addresses: (ray_gen_address, miss_address, hit_address),
                disposed: false, }
        }
        pub fn get_pipeline(&self) -> vk::Pipeline {
            self.pipeline
        }
        pub fn get_pipeline_layout(&self) -> vk::PipelineLayout {
            self.pipeline_layout
        }
    }
    impl IDisposable for RayTacingPipeline{
        fn dispose(&mut self) {
        if !self.disposed{
                self.disposed = true;
                debug!("Destroying pipeline layout {:?}", self.pipeline_layout);
                debug!("Destroying pipeline {:?}", self.pipeline);
                unsafe{
                    self.device.destroy_pipeline_layout(self.pipeline_layout, None);
                    self.device.destroy_pipeline(self.pipeline, None);
                }
            }
    }
    }
    impl Drop for RayTacingPipeline{
        fn drop(&mut self) {
        self.dispose();
    }
    }
    impl Tlas{
        pub fn new<T: IEngine>(engine: &T, profiles: &RayTracingMemoryProfiles, allocator: &mut Allocator, instance_outlines: &[TlasInstanceOutline]) -> Tlas {
            let device = engine.get_device();
            let acc_loader = ash::extensions::khr::AccelerationStructure::new(&engine.get_instance(), &device);
            
            let geometries:Vec<vk::AccelerationStructureGeometryKHR> = instance_outlines.iter().map(|outline| {
                let instance_data = vk::AccelerationStructureGeometryInstancesDataKHR::builder()
                .array_of_pointers(outline.array_of_pointers)
                .data(outline.instance_data)
                .build();
                let mut geo_union = vk::AccelerationStructureGeometryDataKHR::default();
                geo_union.instances = instance_data;
                vk::AccelerationStructureGeometryKHR::builder()
                .geometry_type(vk::GeometryTypeKHR::INSTANCES)
                .geometry(geo_union)
                .build()
            }).collect();
            let primative_counts:Vec<u32> = instance_outlines.iter().map(|outline| {
                outline.instance_count})
            .collect();
            let max_primative_counts:Vec<u32> = instance_outlines.iter().map(|outline| {
                outline.instance_count * outline.instance_count_overkill
            }).collect();
            
            let mut build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_BUILD)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(&geometries);
            
            let build_type = vk::AccelerationStructureBuildTypeKHR::DEVICE;
            let sizing = unsafe{acc_loader.get_acceleration_structure_build_sizes(build_type, &build_info, &max_primative_counts)};
            
            let tlas_region = allocator.get_buffer_region::<u8>(&profiles.acc_struct, sizing.acceleration_structure_size as usize, &AlignmentType::Allocation(256), &[]);
            let scratch_region = allocator.get_buffer_region::<u8>(&profiles.scratch, sizing.build_scratch_size as usize, &AlignmentType::Allocation(profiles.properties.pd_acc_structure_properties.min_acceleration_structure_scratch_offset_alignment as u64), &[]);
            let scratch_device_address = vk::DeviceOrHostAddressKHR{device_address: scratch_region.get_device_address()};
            
            let acc_c_info = vk::AccelerationStructureCreateInfoKHR::builder()
            .buffer(tlas_region.get_buffer())
            .offset(tlas_region.get_buffer_offset())
            .size(tlas_region.get_size())
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL);
            let acc_struct = unsafe{acc_loader.create_acceleration_structure(&acc_c_info, None).expect("Could not create acceleration structre")};
            debug!("Created new top level acceleration structure {:?}", acc_struct);
            
           
            let build_info =[ build_info
            .dst_acceleration_structure(acc_struct)
            .scratch_data(scratch_device_address)
            .build()];
            let build_ranges:Vec<vk::AccelerationStructureBuildRangeInfoKHR> = geometries.iter().enumerate().map(|(index, info)| {
                vk::AccelerationStructureBuildRangeInfoKHR::builder()
                .first_vertex(0)
                .primitive_count(primative_counts[index])
                .primitive_offset(0)
                .build()
            }).collect();
            let build_ranges = [build_ranges.as_slice()];
            let pool = CommandPool::new(engine, vk::CommandPoolCreateInfo::builder().queue_family_index(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().1).build());
            let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];
            
            
            let fence = Fence::new(engine, false);
            unsafe{
                device.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).expect("Could not begin command buffer");
                acc_loader.cmd_build_acceleration_structures(cmd, &build_info, &build_ranges);
                device.end_command_buffer(cmd).expect("Could not end command buffer");
                
                
                let cmds = [cmd];
                let submit = [vk::SubmitInfo::builder()
                .command_buffers(&cmds)
                .build()];
                
                device.queue_submit(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().0, &submit, fence.get_fence()).expect("Could not submit queue");
                fence.wait_reset();
            }           
            
            Tlas{
                device,
                acc_loader,
                profiles: profiles.clone(),
                tlas: [acc_struct],
                disposed: false, }
        }
        pub fn get_write(&self) -> DescriptorWriteType {
            DescriptorWriteType::AccelerationStructure(None, (
                            vk::WriteDescriptorSetAccelerationStructureKHR::builder()
            .acceleration_structures(&self.tlas)
            .build()

            ))
            
        }
        pub fn prepare_instance_memory<T:IEngine>(engine: &T, profiles: &RayTracingMemoryProfiles, allocator: &mut Allocator, count: usize, default: Option<&[vk::AccelerationStructureInstanceKHR]>) -> BufferRegion {
            let instance_buffer: BufferRegion;
            match default{
                Some(d) => {
                    instance_buffer = allocator.get_buffer_region_from_template(&profiles.instance_data, d, &AlignmentType::Free, &[]);
                    let staging_buffer = allocator.get_buffer_region_from_template(&profiles.staging, d, &AlignmentType::Free, &[]);
                    allocator.copy_from_ram_slice(d, &staging_buffer);                    
                    let pool = CommandPool::new(engine, vk::CommandPoolCreateInfo::builder().queue_family_index(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().1).build());
                    let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];
            
            
                    unsafe{
                        engine.get_device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).expect("Could not begin command buffer");
                        staging_buffer.copy_to_region(cmd, &instance_buffer);
                        engine.get_device().end_command_buffer(cmd).expect("Could not end command buffer");
                
                        let fence = Fence::new(engine, false);
                
                        let cmds = [cmd];
                        let submit = [vk::SubmitInfo::builder()
                        .command_buffers(&cmds)
                        .build()];
                
                        engine.get_device().queue_submit(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().0, &submit, fence.get_fence()).expect("Could not submit queue");
                        fence.wait_reset();
                    }           
                },
                None => {
                    instance_buffer = allocator.get_buffer_region::<vk::AccelerationStructureInstanceKHR>(&profiles.instance_data, count, &AlignmentType::Free, &[]);
                },
            }
            instance_buffer
        }
        pub fn get_tlas(&self) -> vk::AccelerationStructureKHR {
            self.tlas[0]
        }
    }
    impl IDisposable for Tlas{
        fn dispose(&mut self) {
        if !self.disposed{
                self.disposed = true;
                debug!("Destroying top level acceleration structure {:?}", self.tlas);
                unsafe{
                    self.acc_loader.destroy_acceleration_structure(self.tlas[0], None);
                }
            }
    }
    }
    impl Drop for Tlas{
        fn drop(&mut self) {
        self.dispose();
    }
    }
    impl Blas{
        pub fn new<T: IEngine>(engine: &T, profiles: &RayTracingMemoryProfiles, allocator: &mut Allocator, objects: &[BlasObjectOutline]) -> Blas {
            let device = engine.get_device();
            let acc_loader = ash::extensions::khr::AccelerationStructure::new(&engine.get_instance(), &device);
            let geometries:Vec<vk::AccelerationStructureGeometryKHR> = objects.iter().map(|outline| outline.geometry).collect();
            let primative_counts:Vec<u32> = objects.iter().map(|outline| outline.primative_count).collect();
            let max_primative_counts:Vec<u32> = objects.iter().map(|outline| outline.max_primative_count).collect();
            
            let mut build_info = vk::AccelerationStructureBuildGeometryInfoKHR::builder()
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE)
            .geometries(&geometries);
            
            let build_type = vk::AccelerationStructureBuildTypeKHR::DEVICE;
            let sizing = unsafe{acc_loader.get_acceleration_structure_build_sizes(build_type, &build_info, &max_primative_counts)};
            
            let blas_region = allocator.get_buffer_region::<u8>(&profiles.acc_struct, sizing.acceleration_structure_size as usize, &AlignmentType::User(256), &[]);
            let scratch_region = allocator.get_buffer_region::<u8>(&profiles.scratch, sizing.build_scratch_size as usize, &AlignmentType::Allocation(profiles.properties.pd_acc_structure_properties.min_acceleration_structure_scratch_offset_alignment as u64), &[]);
            let scratch_device_address = vk::DeviceOrHostAddressKHR{device_address: scratch_region.get_device_address()};
            
            let acc_c_info = vk::AccelerationStructureCreateInfoKHR::builder()
            .buffer(blas_region.get_buffer())
            .offset(blas_region.get_buffer_offset())
            .size(blas_region.get_size())
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL);
            let acc_struct = unsafe{acc_loader.create_acceleration_structure(&acc_c_info, None).expect("Could not create acceleration structre")};
            debug!("Created new bottom level acceleration structure {:?}", acc_struct);
            
           
            let build_info =[ build_info
            .dst_acceleration_structure(acc_struct)
            .scratch_data(scratch_device_address)
            .build()];
            let build_ranges:Vec<vk::AccelerationStructureBuildRangeInfoKHR> = geometries.iter().enumerate().map(|(index, info)| {
                vk::AccelerationStructureBuildRangeInfoKHR::builder()
                .first_vertex(0)
                .primitive_count(primative_counts[index])
                .primitive_offset(0)
                .build()
            }).collect();
            let build_ranges = [build_ranges.as_slice()];
            let pool = CommandPool::new(engine, vk::CommandPoolCreateInfo::builder().queue_family_index(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().1).build());
            let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];
            
            
            unsafe{
                device.begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).expect("Could not begin command buffer");
                acc_loader.cmd_build_acceleration_structures(cmd, &build_info, &build_ranges);
                device.end_command_buffer(cmd).expect("Could not end command buffer");
                
                let fence = Fence::new(engine, false);
                
                let cmds = [cmd];
                let submit = [vk::SubmitInfo::builder()
                .command_buffers(&cmds)
                .build()];
                
                device.queue_submit(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().0, &submit, fence.get_fence()).expect("Could not submit queue");
                fence.wait();
            }           
            
            let acc_struct_address_info = vk::AccelerationStructureDeviceAddressInfoKHR::builder()
            .acceleration_structure(acc_struct);
            let acc_struct_address = unsafe{acc_loader.get_acceleration_structure_device_address(&acc_struct_address_info)};
            Blas{ device,
                acc_loader,
                profiles: profiles.clone(),
                blas_region,
                blas: acc_struct,
                device_address: vk::AccelerationStructureReferenceKHR {device_handle: acc_struct_address},
                disposed: false, }
            
        }
        pub fn get_blas_ref(&self) -> vk::AccelerationStructureReferenceKHR {
            self.device_address
        }
    }
    impl IDisposable for Blas{
        fn dispose(&mut self) {
        if !self.disposed{
                self.disposed = true;
                debug!("Destroying blas {:?}", self.blas);
                unsafe{
                    self.acc_loader.destroy_acceleration_structure(self.blas, None);
                }
            }
        }
    }
    impl Drop for Blas{
        fn drop(&mut self) {
            self.dispose();
    }
    }
    impl TriangleObjectGeometry{
        pub fn new<T: IEngine, V: Clone>(engine: &T, profiles: &RayTracingMemoryProfiles, allocator: &mut Allocator, vertex_data: &[V], vertex_format: vk::Format, index_data: &[u32]) -> TriangleObjectGeometry {
            let vb_stage = allocator.get_buffer_region_from_template(&profiles.staging, &vertex_data, &AlignmentType::Free, &[]);
            let ib_stage = allocator.get_buffer_region_from_template(&profiles.staging, &index_data, &AlignmentType::Free, &[]);
            allocator.copy_from_ram_slice(&vertex_data, &vb_stage);
            allocator.copy_from_ram_slice(&index_data, &ib_stage);
            
            let vertex_buffer = allocator.get_buffer_region_from_template(&profiles.vertex, &vertex_data, &AlignmentType::Free, &[]);
            let vertex_buffer_address = vk::DeviceOrHostAddressConstKHR{device_address: vertex_buffer.get_device_address()};
            let index_buffer = allocator.get_buffer_region_from_template(&profiles.index, &index_data, &AlignmentType::Free, &[]);
            let index_device_address = vk::DeviceOrHostAddressConstKHR{device_address: index_buffer.get_device_address()};
            
            let triangle_info = vk::AccelerationStructureGeometryTrianglesDataKHR::builder()
            .vertex_format(vertex_format)
            .vertex_data(vertex_buffer_address)
            .vertex_stride(size_of::<V>() as u64)
            .max_vertex(vertex_data.len() as u32)
            .index_type(vk::IndexType::UINT32)
            .index_data(index_device_address);
            let mut geo_union = vk::AccelerationStructureGeometryDataKHR::default();
            geo_union.triangles = triangle_info.build();
            
            let geo_info = vk::AccelerationStructureGeometryKHR::builder()
            .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
            .geometry(geo_union)
            .flags(vk::GeometryFlagsKHR::OPAQUE)
            .build();
            
            let pool = CommandPool::new(engine, vk::CommandPoolCreateInfo::builder().queue_family_index(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().1).build());
            let cmd = pool.get_command_buffers(vk::CommandBufferAllocateInfo::builder().command_buffer_count(1).build())[0];
            
            
            unsafe{
                engine.get_device().begin_command_buffer(cmd, &vk::CommandBufferBeginInfo::builder().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT).build()).expect("Could not begin command buffer");
                vb_stage.copy_to_region(cmd, &vertex_buffer);
                ib_stage.copy_to_region(cmd, &index_buffer);
                engine.get_device().end_command_buffer(cmd).expect("Could not end command buffer");
                
                let fence = Fence::new(engine, false);
                
                let cmds = [cmd];
                let submit = [vk::SubmitInfo::builder()
                .command_buffers(&cmds)
                .build()];
                
                engine.get_device().queue_submit(engine.get_queue_store().get_queue(vk::QueueFlags::TRANSFER | vk::QueueFlags::COMPUTE).unwrap().0, &submit, fence.get_fence()).expect("Could not submit queue");
                fence.wait_reset();
            }           
            
            let shader_data = ObjectShaderData{ 
                vertex_address: vertex_buffer.get_device_address(),
                index_address: index_buffer.get_device_address() };
            TriangleObjectGeometry{ 
                vertex_buffer,
                index_buffer,
                geometry_info: geo_info,
                primative_count: (index_data.len()/3) as u32,
                shader_data, }
        }
        pub fn get_shader_data(&self) -> ObjectShaderData {
            self.shader_data.clone()
        }
        pub fn get_blas_outline(&self, primiative_overkill: u32) -> BlasObjectOutline {
            BlasObjectOutline{ geometry: self.geometry_info,
                primative_count: self.primative_count,
                max_primative_count: self.primative_count * primiative_overkill }
        }
    }
    impl ObjectShaderData{
        pub fn new(vertex_address: u64, index_address: u64) -> ObjectShaderData {
            ObjectShaderData{ vertex_address, index_address }
        }
    }
    impl RayTracingMemoryProfiles{
        pub fn new<T: IEngine>(engine: &T, allocator: &mut Allocator) -> RayTracingMemoryProfiles {
            let stage_mem_options = [CreateAllocationOptions::MinimumSize(1024*1024*20)];
            let gpu_mem_options = [CreateAllocationOptions::MemoryAllocateFlags(vk::MemoryAllocateFlagsInfo::builder().flags(vk::MemoryAllocateFlags::DEVICE_ADDRESS).build()), CreateAllocationOptions::MinimumSize(1024*1024*100)];
            let buffer_options = [CreateBufferOptions::SizeOverkillFactor(3), CreateBufferOptions::MinimumSize(1024*1024*10)];
            let stage_mem_profile = AllocatorProfileType::Allocation(AllocationAllocatorProfile::new(vk::MemoryPropertyFlags::HOST_COHERENT, &stage_mem_options));
            let gpu_mem_profile = AllocatorProfileType::Allocation(AllocationAllocatorProfile::new(vk::MemoryPropertyFlags::DEVICE_LOCAL, &gpu_mem_options));
            let staging_profile = AllocatorProfileType::Buffer(BufferAllocatorProfile::new(vk::BufferUsageFlags::TRANSFER_SRC, &buffer_options));
            let vertex_profile = AllocatorProfileType::Buffer(BufferAllocatorProfile::new(
                vk::BufferUsageFlags::TRANSFER_SRC
                     | vk::BufferUsageFlags::TRANSFER_DST
                     | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                     | vk::BufferUsageFlags::VERTEX_BUFFER
                     | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                 &buffer_options));
            let index_profile = AllocatorProfileType::Buffer(BufferAllocatorProfile::new(
                vk::BufferUsageFlags::TRANSFER_SRC
                     | vk::BufferUsageFlags::TRANSFER_DST
                     | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                     | vk::BufferUsageFlags::INDEX_BUFFER
                     | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                 &buffer_options));
            let blas_profile = AllocatorProfileType::Buffer(BufferAllocatorProfile::new(
                vk::BufferUsageFlags::TRANSFER_SRC
                     | vk::BufferUsageFlags::TRANSFER_DST
                     | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR
                     | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                 &buffer_options));
            let scratch_profile = AllocatorProfileType::Buffer(BufferAllocatorProfile::new(
                vk::BufferUsageFlags::STORAGE_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::TRANSFER_SRC,
                &buffer_options));
            let instance_profile = AllocatorProfileType::Buffer(BufferAllocatorProfile::new(
                vk::BufferUsageFlags::STORAGE_BUFFER
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
                &buffer_options));
            let shader_table_profile = AllocatorProfileType::Buffer(BufferAllocatorProfile::new(
                vk::BufferUsageFlags::SHADER_BINDING_TABLE_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::TRANSFER_SRC,
                &buffer_options));
            
            let cpu_mem_index = allocator.add_profile(stage_mem_profile);
            let gpu_mem_index = allocator.add_profile(gpu_mem_profile);
            let staging_index = allocator.add_profile(staging_profile);
            let vertex_index = allocator.add_profile(vertex_profile);
            let index_index = allocator.add_profile(index_profile);
            let blas_index = allocator.add_profile(blas_profile);
            let scratch_index = allocator.add_profile(scratch_profile);
            let instance_index = allocator.add_profile(instance_profile);
            let shader_table_index = allocator.add_profile(shader_table_profile);
            
            let staging_profile = AllocatorProfileStack::TargetBuffer(cpu_mem_index, staging_index);
            let vertex_profile = AllocatorProfileStack::TargetBuffer(gpu_mem_index, vertex_index);
            let index_profile = AllocatorProfileStack::TargetBuffer(gpu_mem_index, index_index);
            let acc_struct_profile = AllocatorProfileStack::TargetBuffer(gpu_mem_index, blas_index);
            let scratch_profile = AllocatorProfileStack::TargetBuffer(gpu_mem_index, scratch_index);
            let instance_data_profile = AllocatorProfileStack::TargetBuffer(gpu_mem_index, instance_index);
            let shader_table_profile = AllocatorProfileStack::TargetBuffer(gpu_mem_index, shader_table_index);
            
            RayTracingMemoryProfiles{ properties: engine.get_property_store(),
                staging: staging_profile,
                vertex: vertex_profile,
                index: index_profile,
                acc_struct: acc_struct_profile,
                instance_data: instance_data_profile,
                scratch: scratch_profile,
                shader_table: shader_table_profile }
        }
        
    }
}
#[allow(dead_code, unused)]
pub mod shader{
    use std::ffi::CStr;

    use ash::vk;
    use log::debug;

    use crate::{init::IEngine, IDisposable};

    pub struct Shader{
        device: ash::Device,
        source: String,
        module: vk::ShaderModule,       
        disposed: bool
    }
    impl Shader{
        pub fn new<T: IEngine>(engine: &T, source: String, kind: shaderc::ShaderKind, ep_name: &str, options: Option<&shaderc::CompileOptions>) -> Shader{
            let module: vk::ShaderModule;
            let compiler = shaderc::Compiler::new().unwrap();
            let byte_source = compiler.compile_into_spirv(source.as_str(), kind, "shader.glsl", ep_name, options).unwrap();
            //debug!("Compiled shader {} to binary {:?}", source, byte_source.as_binary());
            unsafe{
                let c_info = vk::ShaderModuleCreateInfo::builder().code(byte_source.as_binary()).build();
                module = engine.get_device().create_shader_module(&c_info, None).unwrap();
            }
            Shader { device: engine.get_device(), source, module, disposed: false }
        }
        pub fn get_stage(&self, stage: vk::ShaderStageFlags, ep: &CStr) -> vk::PipelineShaderStageCreateInfo{
            vk::PipelineShaderStageCreateInfo::builder()
            .stage(stage)
            .module(self.module)
            .name(ep)
            .build()
        }
    }
    impl IDisposable for Shader{
        fn dispose(&mut self) {
        if !self.disposed{
                self.disposed = true;
                debug!("Destroying shader module {:?}", self.module);
                unsafe{
                    self.device.destroy_shader_module(self.module, None);
                }
            }
    }
    }
    impl Drop for Shader{
        fn drop(&mut self) {
            self.dispose();
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
    use std::ffi::{c_void, CString};

    use ash::vk::{self, Packed24_8};

    use crate::{init::{self, Engine, IEngine, EngineInitOptions}, memory::{self, AlignmentType, AllocationAllocatorProfile, BufferAllocatorProfile, AllocatorProfileStack, CreateAllocationOptions, CreateBufferOptions, Allocator}, IDisposable, descriptor::{DescriptorSetOutline, DescriptorStack}, shader::{Shader}, ray_tracing::{TriangleObjectGeometry, RayTracingMemoryProfiles, Blas, Tlas, TlasInstanceOutline, ShaderTable, RayTacingPipeline} };

    #[cfg(debug_assertions)]
    fn get_vulkan_validate(options: &mut Vec<init::EngineInitOptions>){
        println!("Validation Layers Active");
        let validation_features = [
                    vk::ValidationFeatureEnableEXT::BEST_PRACTICES,
                    vk::ValidationFeatureEnableEXT::GPU_ASSISTED,
                    vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION,
                ];
        options.push(init::EngineInitOptions::UseValidation(Some(validation_features.to_vec()), None));
        options.push(EngineInitOptions::UseDebugUtils);
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
    fn discriptor_test(){
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
        let mut stack = DescriptorStack::new(&engine);
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
    
    
    #[repr(C)]
    #[derive(Clone)]
    pub struct Vertex{
        pos: [f32; 3],
    }    
    #[test]
    fn ray_tracing_test(){
        match pretty_env_logger::try_init() {
            Ok(_) => {},
            Err(_) => {},
        };
        
        let features12 = vk::PhysicalDeviceVulkan12Features::builder()
        .buffer_device_address(true)
        .timeline_semaphore(true);
        let acc_features = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder()
        .acceleration_structure(true);
        let ray_tracing_features = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder()
        .ray_tracing_pipeline(true);
        let acc_extension = ash::extensions::khr::AccelerationStructure::name().as_ptr();
        let ray_tracing = ash::extensions::khr::RayTracingPipeline::name().as_ptr();
        let def_host = ash::extensions::khr::DeferredHostOperations::name().as_ptr();
        
        let mut options = vec![
            EngineInitOptions::DeviceFeatures12(features12.build()),
            EngineInitOptions::DeviceFeaturesAccelerationStructure(acc_features.build()),
            EngineInitOptions::DeviceFeaturesRayTracing(ray_tracing_features.build()),
            EngineInitOptions::DeviceExtensions(vec![acc_extension, def_host, ray_tracing]),
        ];
        get_vulkan_validate(&mut options);
        let (engine, _) = Engine::init(&mut options, None);
        let mut allocator = Allocator::new(&engine);
        let ray_tracing_profiles = RayTracingMemoryProfiles::new(&engine, &mut allocator);
        let v_data = [
            Vertex{pos: [ 0.0, 1.0, 0.0]}, //top
            Vertex{pos: [ -1.0, -1.0,0.5]},  //left
            Vertex{pos: [1.0,-1.0,0.5]}, //right
            Vertex{pos: [0.0, -1.0, -0.5]}, //front  
        ];
        let i_data = [
            3, 2, 0, //fr
            1, 0, 2, //back
            1, 3, 0, //fl
            1,2,3 ]; //bottom
    
        let mut options = shaderc::CompileOptions::new().unwrap();
        options.set_target_spirv(shaderc::SpirvVersion::V1_6);
        let ray_gen = Shader::new(&engine, String::from(r#"
        #version 460
        #extension GL_EXT_ray_tracing : require
        #extension GL_KHR_vulkan_glsl : enable

        layout(binding = 0, set = 0) uniform accelerationStructureEXT topLevelAS;
        layout(binding = 1, set = 0, rgba32f) uniform image2D image;

        struct hitPayload
        {
            bool hit;
            vec3 hitValue;
        };

        layout(location = 0) rayPayloadEXT hitPayload prd;

        void main() 
            {
                const vec2 pixelCenter = vec2(gl_LaunchIDEXT.xy) + vec2(0.5);
                const vec2 inUV = pixelCenter/vec2(gl_LaunchSizeEXT.xy);
                vec2 d = inUV * 2.0 - 1.0;
                vec4 origin    = vec4(0, 0, -1, 1);
                vec4 target    = vec4(d.x, -d.y, 0, 1);
                vec4 direction = vec4(normalize(target.xyz - origin.xyz), 0);
                uint  rayFlags = gl_RayFlagsOpaqueEXT;
                float tMin     = 0.001;
                float tMax     = 100000.0;
                traceRayEXT(topLevelAS, // acceleration structure
                    rayFlags,       // rayFlags
                    0xFF,           // cullMask
                    0,              // sbtRecordOffset
                    0,              // sbtRecordStride
                    0,              // missIndex
                    origin.xyz,     // ray origin
                    tMin,           // ray min range
                    direction.xyz,  // ray direction
                    tMax,           // ray max range
                    0               // payload (location = 0)
            );
                if (d.x > 0 && prd.hit){
                    prd.hitValue = prd.hitValue * 0.5;
                }
                imageStore(image, ivec2(gl_LaunchIDEXT.xy), vec4(prd.hitValue,1.0));
            }

        "#), shaderc::ShaderKind::RayGeneration, "main", Some(&options));
        let closest_hit = Shader::new(&engine, String::from(r#"
        #version 460
        #extension GL_EXT_ray_tracing : require
        #extension GL_EXT_nonuniform_qualifier : enable

        struct hitPayload
        {
            bool hit;
            vec3 hitValue;
        };

        layout(location = 0) rayPayloadInEXT hitPayload hitdata;
        hitAttributeEXT vec3 attribs;

        void main()
        {
            hitdata.hit = true;
            hitdata.hitValue = vec3(0.2, 0.5, 0.5);
        }"#), shaderc::ShaderKind::ClosestHit, "main", Some(&options));
        let miss = Shader::new(&engine, String::from(r#"
        #version 460
        #extension GL_EXT_ray_tracing : require

        struct hitPayload
        {
            bool hit;
            vec3 hitValue;
        };

        layout(location = 0) rayPayloadInEXT hitPayload hitdata;

        void main()
        {
            hitdata.hit = false;
            hitdata.hitValue = vec3(0.0, 0.1, 0.3);
        }"#), shaderc::ShaderKind::Miss, "main", Some(&options));
        let main = CString::new("main").unwrap();
        let sbt = ShaderTable{ 
            ray_gen: vec![ray_gen.get_stage(vk::ShaderStageFlags::RAYGEN_KHR, &main)],
            hit_groups: vec![(vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP, (Some(closest_hit.get_stage(vk::ShaderStageFlags::CLOSEST_HIT_KHR, &main)), None, None))],
            misses: vec![] };
        let ray_pipeline = RayTacingPipeline::new(&engine, &sbt, &ray_tracing_profiles, &mut allocator, &[], &[]);
        
        let object_data = TriangleObjectGeometry::new(&engine, &ray_tracing_profiles, &mut allocator, &v_data, vk::Format::R32G32B32_SFLOAT, &i_data);
        let blas_outlines = [object_data.get_blas_outline(1)];
        let blas = Blas::new(&engine, &ray_tracing_profiles, &mut allocator, &blas_outlines);
        let transform = vk::TransformMatrixKHR{ matrix: 
            [1.0,0.0,0.0,0.0,
             0.0,1.0,0.0,0.0,
             0.0,0.0,1.0,1.0] };
        let default_instance =[ vk::AccelerationStructureInstanceKHR{ 
            transform, 
            instance_custom_index_and_mask: Packed24_8::new(0, 0xff), 
            instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(0, 0x00000002 as u8), 
            acceleration_structure_reference: blas.get_blas_ref()}];
        let instance_buffer = Tlas::prepare_instance_memory(&engine, &ray_tracing_profiles, &mut allocator, 100, Some(&default_instance));
        let instance_data =[ TlasInstanceOutline{ 
            instance_data: vk::DeviceOrHostAddressConstKHR{device_address: instance_buffer.get_device_address()},
            instance_count: 100,
            instance_count_overkill: 1,
            array_of_pointers: false }];
        let _tlas = Tlas::new(&engine, &ray_tracing_profiles, &mut allocator, &instance_data);
        
        println!("Won't lose device {:?}", engine.get_device().handle());
    }

}
