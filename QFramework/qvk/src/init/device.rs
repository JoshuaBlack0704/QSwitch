use crate::init::{DeviceSource, InstanceSource, PhysicalDeviceData};
use ash::vk::{self, DeviceSize, PhysicalDevice, SurfaceKHR};
use ash_window;
use log::{debug, info};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use std::{cmp::Ordering, io, sync::Arc};
use winit;

use super::{Device, DeviceFactory};

pub trait DeviceSettingsStore {
    fn choose_device(&self) -> bool;
    fn surface_support(&self) -> Option<&winit::window::Window>;
    fn use_features(&self) -> Option<vk::PhysicalDeviceFeatures> {
        None
    }
    fn use_features11(&self) -> Option<vk::PhysicalDeviceVulkan11Features> {
        None
    }
    fn use_features12(&self) -> Option<vk::PhysicalDeviceVulkan12Features> {
        None
    }
    fn use_features13(&self) -> Option<vk::PhysicalDeviceVulkan13Features> {
        None
    }
    fn use_raytracing_features(&self) -> Option<vk::PhysicalDeviceRayTracingPipelineFeaturesKHR> {
        None
    }
    fn use_acc_struct_features(
        &self,
    ) -> Option<vk::PhysicalDeviceAccelerationStructureFeaturesKHR> {
        None
    }
    fn use_device_extensions(&self) -> Option<&[*const i8]>;
}

pub struct Settings<'a, I: InstanceSource + Clone> {
    pub choose_device: bool,
    pub surface_support: Option<&'a winit::window::Window>,
    pub features: Option<vk::PhysicalDeviceFeatures>,
    pub features11: Option<vk::PhysicalDeviceVulkan11Features>,
    pub features12: Option<vk::PhysicalDeviceVulkan12Features>,
    pub features13: Option<vk::PhysicalDeviceVulkan13Features>,
    pub raytracing_features: Option<vk::PhysicalDeviceRayTracingPipelineFeaturesKHR>,
    pub acc_struct_features: Option<vk::PhysicalDeviceAccelerationStructureFeaturesKHR>,
    pub device_extensions: Option<Vec<*const i8>>,
    pub instance: I,
}
impl<'a, I: InstanceSource + Clone> Settings<'a, I> {
    pub fn new(
        choose_device: bool,
        surface_support: Option<&'a winit::window::Window>,
        features: Option<vk::PhysicalDeviceFeatures>,
        features11: Option<vk::PhysicalDeviceVulkan11Features>,
        features12: Option<vk::PhysicalDeviceVulkan12Features>,
        features13: Option<vk::PhysicalDeviceVulkan13Features>,
        raytracing_features: Option<vk::PhysicalDeviceRayTracingPipelineFeaturesKHR>,
        acc_struct_features: Option<vk::PhysicalDeviceAccelerationStructureFeaturesKHR>,
        device_extensions: Option<Vec<*const i8>>,
        instance: I,
    ) -> Settings<'a, I> {
        Settings {
            choose_device,
            surface_support,
            features,
            features11,
            features12,
            features13,
            raytracing_features,
            acc_struct_features,
            device_extensions,
            instance,
        }
    }
    pub fn new_simple(instance_source: I) -> Settings<'a, I> {
        let mut settings = Self::new(
            false,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            instance_source,
        );
        settings.add_extension(ash::extensions::khr::Synchronization2::name().as_ptr());
        let features12 = vk::PhysicalDeviceVulkan12Features::builder()
            .buffer_device_address(true)
            .timeline_semaphore(true)
            .build();
        let features13 = vk::PhysicalDeviceVulkan13Features::builder()
            .synchronization2(true)
            .build();
        settings.features12(features12);
        settings.features13(features13);
        settings
    }

    pub fn add_window(&mut self, window: &'a winit::window::Window) {
        self.surface_support = Some(window);
    }

    pub fn choose_device(&mut self, allow_option: bool) {
        self.choose_device = allow_option;
    }

    pub fn features11(&mut self, features: vk::PhysicalDeviceVulkan11Features) {
        self.features11 = Some(features);
    }
    pub fn features12(&mut self, features: vk::PhysicalDeviceVulkan12Features) {
        self.features12 = Some(features);
    }
    pub fn features13(&mut self, features: vk::PhysicalDeviceVulkan13Features) {
        self.features13 = Some(features);
    }
    pub fn raytracing_features(
        &mut self,
        features: vk::PhysicalDeviceRayTracingPipelineFeaturesKHR,
    ) {
        self.raytracing_features = Some(features);
    }
    pub fn acc_struct_features(
        &mut self,
        features: vk::PhysicalDeviceAccelerationStructureFeaturesKHR,
    ) {
        self.acc_struct_features = Some(features);
    }
    pub fn add_extension(&mut self, name: *const i8) {
        self.device_extensions.get_or_insert(vec![]).push(name);
    }
    fn get_physical_devices(instance: &I) -> Vec<PhysicalDeviceData> {
        // First we pull all of our devices
        let instance = instance.instance();
        let physical_devices = unsafe {
            instance
                .enumerate_physical_devices()
                .expect("Could not get physical devices")
        };
        let mut datas = vec![];

        for device in physical_devices {
            datas.push(PhysicalDeviceData::new(device, instance));
        }

        datas.sort_by(PhysicalDeviceData::more_mem);

        datas
    }

    fn get_queue_infos(
        physical_device: &PhysicalDeviceData,
        priorities: &[f32],
    ) -> Vec<vk::DeviceQueueCreateInfo> {
        let mut qf_cinfos = vec![];

        for (index, fam_props) in physical_device.queue_properties.iter().enumerate() {
            // If this queue is useless we skip it
            if !(fam_props.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                || fam_props.queue_flags.contains(vk::QueueFlags::COMPUTE)
                || fam_props.queue_flags.contains(vk::QueueFlags::TRANSFER))
            {
                continue;
            }

            // If not we make a new info
            debug!("Using queue family {:?}", index);

            let qf_cinfo = vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(index as u32)
                .queue_priorities(priorities)
                .build();

            qf_cinfos.push(qf_cinfo);
        }

        qf_cinfos
    }
}

impl<'a, I: InstanceSource + Clone> DeviceSettingsStore for Settings<'a, I> {
    fn choose_device(&self) -> bool {
        self.choose_device
    }

    fn surface_support(&self) -> Option<&winit::window::Window> {
        self.surface_support
    }

    fn use_device_extensions(&self) -> Option<&[*const i8]> {
        if let Some(ext) = &self.device_extensions {
            return Some(ext);
        }
        None
    }

    fn use_features(&self) -> Option<vk::PhysicalDeviceFeatures> {
        self.features
    }

    fn use_features11(&self) -> Option<vk::PhysicalDeviceVulkan11Features> {
        self.features11
    }

    fn use_features12(&self) -> Option<vk::PhysicalDeviceVulkan12Features> {
        self.features12
    }

    fn use_features13(&self) -> Option<vk::PhysicalDeviceVulkan13Features> {
        self.features13
    }

    fn use_raytracing_features(&self) -> Option<vk::PhysicalDeviceRayTracingPipelineFeaturesKHR> {
        self.raytracing_features
    }

    fn use_acc_struct_features(
        &self,
    ) -> Option<vk::PhysicalDeviceAccelerationStructureFeaturesKHR> {
        self.acc_struct_features
    }
}

type DeviceType<I> = Arc<Device<I>>;
impl<'a, I: InstanceSource + Clone> DeviceFactory<DeviceType<I>> for Settings<'a, I> {
    fn create_device(&self) -> Result<DeviceType<I>, vk::Result> {
        let instance = self.instance();
        let entry = self.entry();
        let surface_loader = ash::extensions::khr::Surface::new(entry, instance);

        let mut surface = None;
        if let Some(window) = self.surface_support() {
            let display = window.raw_display_handle();
            let window = window.raw_window_handle();
            surface = Some(unsafe {
                ash_window::create_surface(entry, instance, display, window, None)
                    .expect("Could not create requested surface")
            });
            info!(
                "Surface support request satisfyied with surface {:?}",
                surface.unwrap()
            )
        }

        // We need to enumerate and sort all physical devices based on memory size
        let physical_devices = Settings::<I>::get_physical_devices(&self.instance);
        let physical_device;
        if self.choose_device() {
            println!("Please choose a device:");
            for (index, dev) in physical_devices.iter().enumerate() {
                println!("    {}: {}\n", index, dev.get_name());
            }
            println!("Please enter the number of the device you wish to use ");
            let io = io::stdin();
            let mut user_input = String::new();
            io.read_line(&mut user_input).unwrap();
            let dev_index: u64 = user_input.trim().parse().expect("Did not understand input");
            physical_device = physical_devices
                .get(dev_index as usize)
                .expect("Not a valid index")
                .clone();
        } else {
            physical_device = physical_devices.get(0).unwrap().clone();
            println!("Using device {}", physical_device.get_name());
        }

        // Now that we have chosen our device we can pull its queues
        let priorities = [1.0; 1];
        let q_cinfos = Settings::<I>::get_queue_infos(&physical_device, &priorities);
        let q_families = q_cinfos
            .iter()
            .map(|i| i.queue_family_index as usize)
            .collect();

        let mut device_builder = vk::DeviceCreateInfo::builder();
        device_builder = device_builder.queue_create_infos(&q_cinfos);

        let mut feature_builder = vk::PhysicalDeviceFeatures2::builder();
        let features: Option<vk::PhysicalDeviceFeatures> = self.use_features();
        let mut features11: Option<vk::PhysicalDeviceVulkan11Features> = self.use_features11();
        let mut features12: Option<vk::PhysicalDeviceVulkan12Features> = self.use_features12();
        let mut features13: Option<vk::PhysicalDeviceVulkan13Features> = self.use_features13();
        let mut raytracing_features: Option<vk::PhysicalDeviceRayTracingPipelineFeaturesKHR> =
            self.use_raytracing_features();
        let mut acc_struct_features: Option<vk::PhysicalDeviceAccelerationStructureFeaturesKHR> =
            self.use_acc_struct_features();

        if let Some(f) = features {
            debug!("Using device features");
            feature_builder = feature_builder.features(f);
        }
        if let Some(f) = &mut features11 {
            debug!("Using device features11");
            feature_builder = feature_builder.push_next(f);
        }
        if let Some(f) = &mut features12 {
            debug!("Using device features12");
            feature_builder = feature_builder.push_next(f);
        }
        if let Some(f) = &mut features13 {
            debug!("Using device features13");
            feature_builder = feature_builder.push_next(f);
        }
        if let Some(f) = &mut raytracing_features {
            debug!("Using device ray tracing features");
            feature_builder = feature_builder.push_next(f);
        }
        if let Some(f) = &mut acc_struct_features {
            debug!("Using device acceleration structure features");
            feature_builder = feature_builder.push_next(f);
        }

        if let Some(ext) = self.use_device_extensions() {
            device_builder = device_builder.enabled_extension_names(ext);
        }

        device_builder = device_builder.push_next(&mut feature_builder);

        let device = unsafe {
            instance.create_device(physical_device.physical_device, &device_builder, None)
        };

        match device {
            Ok(d) => {
                info!("Created logical device {:?}", d.handle());
                Ok(Arc::new(Device {
                    instance: self.instance.clone(),
                    surface,
                    surface_loader,
                    physical_device,
                    device: d,
                    created_queue_families: q_families,
                }))
            }
            Err(e) => Err(e),
        }
    }
}

impl<I: InstanceSource> DeviceSource for Arc<Device<I>> {
    fn device(&self) -> &ash::Device {
        &self.device
    }

    fn surface(&self) -> &Option<SurfaceKHR> {
        &self.surface
    }

    fn physical_device(&self) -> &PhysicalDeviceData {
        &self.physical_device
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        let mut best_score = u32::MAX;
        let mut target_queue = None;
        for family in self.created_queue_families.iter() {
            let props = &self.physical_device.queue_properties[*family];
            if props.queue_flags.contains(target_flags) {
                let mut local_score = 0;
                if props.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    local_score += 1;
                }
                if props.queue_flags.contains(vk::QueueFlags::TRANSFER) {
                    local_score += 1;
                }
                if props.queue_flags.contains(vk::QueueFlags::COMPUTE) {
                    local_score += 1;
                }
                if local_score < best_score {
                    best_score = local_score;
                    let queue = unsafe { self.device.get_device_queue((*family) as u32, 0) };
                    target_queue = Some((queue, (*family) as u32));
                }
            }
        }
        target_queue
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.get_queue(vk::QueueFlags::GRAPHICS)
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.get_queue(vk::QueueFlags::COMPUTE)
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.get_queue(vk::QueueFlags::TRANSFER)
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.get_queue(vk::QueueFlags::GRAPHICS)
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.physical_device.get_memory_index(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.physical_device
            .get_memory_index(vk::MemoryPropertyFlags::DEVICE_LOCAL)
    }

    fn host_memory_index(&self) -> u32 {
        self.physical_device
            .get_memory_index(vk::MemoryPropertyFlags::HOST_VISIBLE)
    }
}

impl<I: InstanceSource> Drop for Device<I> {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
        }
        if let Some(s) = self.surface {
            debug!("Destroyed surface {:?}", s);
            unsafe { self.surface_loader.destroy_surface(s, None) };
        }

        debug!("Destroyed device {:?}", self.device.handle());
        unsafe { self.device.destroy_device(None) };
    }
}

impl PhysicalDeviceData {
    fn new(device: PhysicalDevice, instance: &ash::Instance) -> PhysicalDeviceData {
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

        let queue_props;
        unsafe {
            instance.get_physical_device_properties2(device, &mut properties2);
            instance.get_physical_device_memory_properties2(device, &mut memory_properties);
            queue_props = instance.get_physical_device_queue_family_properties(device);
        }

        PhysicalDeviceData {
            physical_device: device,
            properties: properties2.properties,
            queue_properties: queue_props,
            raytracing_properties: ray_props,
            acc_structure_properties: acc_props,
            mem_props: memory_properties.memory_properties,
            mem_budgets: memory_budgets,
        }
    }

    fn get_name(&self) -> String {
        String::from_utf8(
            self.properties
                .device_name
                .iter()
                .map(|&c| c as u8)
                .collect(),
        )
        .unwrap()
        .replace("\0", "")
    }

    fn more_mem(a: &Self, b: &Self) -> Ordering {
        let mut a_max: DeviceSize = 0;
        let mut b_max: DeviceSize = 0;

        for mem_type in a.mem_props.memory_types.iter() {
            let heap = a.mem_props.memory_heaps[mem_type.heap_index as usize];
            if heap.size > a_max {
                a_max = heap.size
            }
        }
        for mem_type in b.mem_props.memory_types.iter() {
            let heap = b.mem_props.memory_heaps[mem_type.heap_index as usize];
            if heap.size > b_max {
                b_max = heap.size
            }
        }

        if a_max > b_max {
            return Ordering::Less;
        }
        if a_max < b_max {
            return Ordering::Greater;
        }

        Ordering::Equal
    }

    /// Selects the biggest heap that matches properties
    pub fn get_memory_index(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        //First we need to sort by matching properties
        let mut matches: Vec<MemTH> = self
            .mem_props
            .memory_types
            .iter()
            .enumerate()
            .filter(|(_, t)| t.property_flags.contains(properties))
            .map(|(i, t)| MemTH {
                i,
                t: *t,
                h: self.mem_props.memory_heaps[t.heap_index as usize],
            })
            .collect();
        matches.sort_by(MemTH::cmp);

        //Now we select the first one
        let selected = matches.get(0).expect("Could not find suitable memory");
        debug!(
            "Memory type index fetch found memory type {}: properties: {:?} size: {:.2?} Gb",
            selected.i,
            selected.t.property_flags,
            (selected.h.size / 1024 / 1024) as f32 / 1024.0
        );
        selected.i as u32
    }
}
struct MemTH {
    i: usize,
    t: vk::MemoryType,
    h: vk::MemoryHeap,
}
impl MemTH {
    fn cmp(a: &Self, imcumbent: &Self) -> Ordering {
        if a.h.size > imcumbent.h.size {
            return Ordering::Less;
        }
        if a.h.size < imcumbent.h.size {
            return Ordering::Greater;
        }
        Ordering::Equal
    }
}

impl<I: InstanceSource + Clone> InstanceSource for Settings<'_, I> {
    fn instance(&self) -> &ash::Instance {
        self.instance.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.instance.entry()
    }
}

impl<I: InstanceSource + Clone> InstanceSource for Arc<Device<I>> {
    fn instance(&self) -> &ash::Instance {
        self.instance.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.instance.entry()
    }
}
