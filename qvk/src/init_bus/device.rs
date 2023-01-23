use std::{cmp::Ordering, sync::Arc};

use ash::vk;
use log::debug;
use qcom::bus::Bus;

use crate::bus::{QvkBus, QvkBusMessage};

use super::{PhysicalDeviceData, Device, DeviceBuilder, DeviceFeatures, DeviceExtension};

impl<'a> DeviceBuilder<'a>{
    pub fn user_device_select(mut self, select: bool) -> Self {
        self.user_device_select = select;
        self
    }

    pub fn surface_support(mut self, window: &'a winit::window::Window) -> Self{
        self.surface_support = Some(window);
        self
    }

    pub fn features(mut self, features: vk::PhysicalDeviceFeatures) -> Self{
        self.features = features;
        self
    }

     pub fn add_extended_feature(mut self, feature: DeviceFeatures) -> Self{
        self.extended_features.push(feature);
        self
    }

    pub fn add_extension(mut self, extension: DeviceExtension) -> Self{
        self.extensions.push(extension);
        self
    }

    pub fn build(self, bus: Arc<QvkBus>) {
        let instance = bus.get_instance();
        
    }
    
}


impl<'a> Device{
    pub fn builder() -> DeviceBuilder<'a> {
           DeviceBuilder{
            user_device_select: todo!(),
            surface_support: todo!(),
            features: todo!(),
            extended_features: todo!(),
            extensions: todo!(),
        } 
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
impl PhysicalDeviceData {
    fn new(device: vk::PhysicalDevice, instance: &ash::Instance) -> PhysicalDeviceData {
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

        
        let mut a_max: u64 = 0;
        let mut b_max: u64 = 0;
        

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
        
        if a.properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU{
            a_max = 0;
        }
        if b.properties.device_type == vk::PhysicalDeviceType::INTEGRATED_GPU{
            b_max = 0;
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