use std::sync::{Arc, Mutex};

use ash::vk;
use log::{debug, info};

use crate::{
    init::{DeviceSource, InstanceSource},
    memory::allocators::{test_partition, PartitionSystem},
};

use super::{
    BufPart, BufferAllocator, BufferAllocatorFactory, BufferExtensions, BufferSource, MemPart,
    MemorySource,
};

impl<M: DeviceSource + MemorySource + Clone> BufferAllocatorFactory for M {
    type Buffer = Arc<BufferAllocator<M>>;
    fn create_buffer(
        &self,
        min_size: u64,
        usage: vk::BufferUsageFlags,
        flags: Option<vk::BufferCreateFlags>,
        extensions: &[BufferExtensions],
        share: Option<Vec<u32>>,
    ) -> Self::Buffer {
        Arc::new(BufferAllocator {
            mem_alloc: self.clone(),
            min_size,
            usage,
            flags,
            extensions: extensions.to_vec(),
            share,
            buffers: Mutex::new(vec![]),
        })
    }

    fn create_uniform_buffer(
        &self,
        min_size: u64,
        additional_usage: Option<vk::BufferUsageFlags>,
    ) -> Self::Buffer {
        let mut usage = vk::BufferUsageFlags::UNIFORM_BUFFER;
        if let Some(u) = additional_usage {
            usage = usage | u
        }
        self.create_buffer(min_size, usage, None, &[], None)
    }

    fn create_storage_buffer(
        &self,
        min_size: u64,
        additional_usage: Option<vk::BufferUsageFlags>,
    ) -> Self::Buffer {
        let mut usage = vk::BufferUsageFlags::STORAGE_BUFFER;
        if let Some(u) = additional_usage {
            usage = usage | u
        }
        self.create_buffer(min_size, usage, None, &[], None)
    }

    fn create_dev_addr_storage_buffer(
        &self,
        min_size: u64,
        additional_usage: Option<vk::BufferUsageFlags>,
    ) -> Self::Buffer {
        let mut usage =
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS;
        if let Some(u) = additional_usage {
            usage = usage | u
        }
        self.create_buffer(min_size, usage, None, &[], None)
    }

    fn create_vertex_buffer(
        &self,
        min_size: u64,
        additional_usage: Option<vk::BufferUsageFlags>,
    ) -> Self::Buffer {
        let mut usage = vk::BufferUsageFlags::VERTEX_BUFFER;
        if let Some(u) = additional_usage {
            usage = usage | u
        }
        self.create_buffer(min_size, usage, None, &[], None)
    }

    fn create_index_buffer(
        &self,
        min_size: u64,
        additional_usage: Option<vk::BufferUsageFlags>,
    ) -> Self::Buffer {
        let mut usage = vk::BufferUsageFlags::INDEX_BUFFER;
        if let Some(u) = additional_usage {
            usage = usage | u
        }
        self.create_buffer(min_size, usage, None, &[], None)
    }
}

impl<M: DeviceSource + MemorySource> BufferSource for Arc<BufferAllocator<M>> {
    fn get_space(&self, size: u64, mut alignment: Option<u64>) -> (MemPart, BufPart) {
        //First we need to loop through all of the buffers
        //and attempt to find a block of space large enough
        if let None = alignment {
            if self.usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
                alignment = Some(
                    self.physical_device()
                        .properties
                        .limits
                        .min_storage_buffer_offset_alignment,
                );
            } else if self.usage.contains(vk::BufferUsageFlags::UNIFORM_BUFFER) {
                alignment = Some(
                    self.physical_device()
                        .properties
                        .limits
                        .min_uniform_buffer_offset_alignment,
                );
            } else if self
                .usage
                .contains(vk::BufferUsageFlags::UNIFORM_TEXEL_BUFFER)
            {
                alignment = Some(
                    self.physical_device()
                        .properties
                        .limits
                        .min_texel_buffer_offset_alignment,
                );
            }
        }

        let mut buffs = self.buffers.lock().unwrap();
        for (mp, b, ps) in buffs.iter() {
            if let Ok(p) = test_partition(ps, size, alignment) {
                return (mp.clone(), (*b, p));
            }
        }

        //If we made it here, we have no space in any of our buffers
        //and must create a new one
        let new_size;
        if size < self.min_size {
            new_size = self.min_size;
        } else {
            new_size = size * 2;
        }

        let mut info = vk::BufferCreateInfo::builder();
        if let Some(flags) = self.flags {
            info = info.flags(flags);
        }
        info = info
            .size(new_size)
            .usage(self.usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        if let Some(indices) = &self.share {
            info = info
                .sharing_mode(vk::SharingMode::CONCURRENT)
                .queue_family_indices(indices);
        }
        let mut extensions = self.extensions.clone();
        for ext in extensions.iter_mut() {
            info = ext.push(info);
        }

        let device = self.mem_alloc.device();
        let buffer = unsafe { device.create_buffer(&info, None).unwrap() };
        info!("Created buffer {:?}", buffer);
        let mem_reqs = unsafe { device.get_buffer_memory_requirements(buffer) };

        let mp = self
            .mem_alloc
            .get_space(mem_reqs.size, Some(mem_reqs.alignment));

        unsafe {
            device
                .bind_buffer_memory(buffer, mp.0, mp.1.offset)
                .unwrap()
        };

        let p_sys = Mutex::new(PartitionSystem::new(new_size));
        let p = test_partition(&p_sys, size, alignment).unwrap();
        let buf = (mp.clone(), buffer, p_sys);
        buffs.push(buf);
        (mp, (buffer, p))
    }

    fn usage(&self) -> vk::BufferUsageFlags {
        self.usage
    }
}

impl<M: MemorySource + DeviceSource> Drop for BufferAllocator<M> {
    fn drop(&mut self) {
        for b in self.buffers.lock().unwrap().iter() {
            debug!("Destroyed buffer {:?}", b.1);
            unsafe {
                self.mem_alloc.device().destroy_buffer(b.1, None);
            }
        }
    }
}

impl<M: MemorySource + DeviceSource> DeviceSource for Arc<BufferAllocator<M>> {
    fn device(&self) -> &ash::Device {
        self.mem_alloc.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.mem_alloc.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.mem_alloc.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.mem_alloc.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.mem_alloc.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.mem_alloc.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.mem_alloc.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.mem_alloc.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.mem_alloc.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.mem_alloc.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.mem_alloc.host_memory_index()
    }
}

impl<M: MemorySource + DeviceSource + InstanceSource> InstanceSource for Arc<BufferAllocator<M>> {
    fn instance(&self) -> &ash::Instance {
        self.mem_alloc.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.mem_alloc.entry()
    }
}

impl BufferExtensions {
    pub fn push<'a>(
        &mut self,
        _builder: vk::BufferCreateInfoBuilder<'a>,
    ) -> vk::BufferCreateInfoBuilder<'a> {
        todo!()
    }
}
