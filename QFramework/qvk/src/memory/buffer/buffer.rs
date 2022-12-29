use std::{
    ffi::c_void,
    sync::{Arc, Mutex},
};

use ash::vk;
use log::{debug, info};

use crate::memory::buffer::BufferSource;
use crate::memory::{MemorySource, PartitionSource};
use crate::{
    init::{DeviceSource, InstanceSource},
    memory::{
        partitionsystem::{self, PartitionError},
        Partition, PartitionSystem,
    },
};

use super::{Buffer, BufferFactory};

pub trait BufferSettingsStore {
    fn size(&self) -> vk::DeviceSize;
    fn flags(&self) -> Option<vk::BufferCreateFlags>;
    fn extensions(&self) -> Option<Vec<BufferCreateExtension>>;
    fn usage(&self) -> vk::BufferUsageFlags;
    fn share(&self) -> Option<Vec<u32>>;
}

#[derive(Clone)]
pub enum BufferCreateExtension {}

#[derive(Clone, Copy)]
pub enum BufferAlignmentType {
    // No alignent, like for vertex buffers
    Free,
    // For example, if you have a storage buffer, you will need to align to minStorageBufferOffsetAlignment
    Aligned(u64),
}

#[derive(Debug)]
pub enum BufferCreateError {
    VulkanResult(vk::Result),
    ParitionError(partitionsystem::PartitionError),
}

pub struct SettingsStore {
    pub size: vk::DeviceSize,
    pub flags: Option<vk::BufferCreateFlags>,
    pub extensions: Option<Vec<BufferCreateExtension>>,
    pub usage: vk::BufferUsageFlags,
    pub share: Option<Vec<u32>>,
}

impl<MS: MemorySource + DeviceSource + Clone> BufferFactory<Arc<Buffer<MS, PartitionSystem>>>
    for MS
{
    fn create_buffer(
        &self,
        size: u64,
        usage: vk::BufferUsageFlags,
        flags: Option<vk::BufferCreateFlags>,
        extensions: Option<*const c_void>,
    ) -> Result<Arc<Buffer<MS, PartitionSystem>>, BufferCreateError> {
        // First we need to create the buffer
        let mut info = vk::BufferCreateInfo::builder();
        if let Some(ptr) = extensions {
            info.p_next = ptr;
        }
        if let Some(flags) = flags {
            info = info.flags(flags);
        }
        info = info.size(size);
        info = info.usage(usage);
        info = info.sharing_mode(vk::SharingMode::EXCLUSIVE);
        let indices = self.share();
        if let Some(indices) = &indices {
            info = info.sharing_mode(vk::SharingMode::CONCURRENT);
            info = info.queue_family_indices(&indices);
        }

        let device = self.device();
        let buffer = unsafe { device.create_buffer(&info, None) };

        if let Err(e) = buffer {
            return Err(BufferCreateError::VulkanResult(e));
        }

        let buffer = buffer.unwrap();
        let reqs: vk::MemoryRequirements = unsafe { device.get_buffer_memory_requirements(buffer) };
        let memory_partition = self.partition(reqs.size, Some(reqs.alignment));

        if let Err(e) = memory_partition {
            return Err(BufferCreateError::ParitionError(e));
        }
        let memory_partition = memory_partition.unwrap();
        let offset = memory_partition.offset();
        info!(
            "Created buffer {:?} of size {:?} on memory {:?} at offset {:?}",
            buffer,
            size,
            *self.memory(),
            offset
        );

        // Now that we have a suitable memory partition we need to bind our buffer
        let result = unsafe { device.bind_buffer_memory(buffer, *self.memory(), offset) };
        if let Err(e) = result {
            return Err(BufferCreateError::VulkanResult(e));
        }

        let mut alignment = BufferAlignmentType::Free;
        if usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
            alignment = BufferAlignmentType::Aligned(
                self.physical_device()
                    .properties
                    .limits
                    .min_storage_buffer_offset_alignment,
            );
        } else if usage.contains(vk::BufferUsageFlags::UNIFORM_BUFFER) {
            alignment = BufferAlignmentType::Aligned(
                self.physical_device()
                    .properties
                    .limits
                    .min_uniform_buffer_offset_alignment,
            );
        } else if usage.contains(vk::BufferUsageFlags::UNIFORM_TEXEL_BUFFER) {
            alignment = BufferAlignmentType::Aligned(
                self.physical_device()
                    .properties
                    .limits
                    .min_texel_buffer_offset_alignment,
            );
        }

        Ok(Arc::new(Buffer {
            memory: self.clone(),
            memory_partition,
            partition_sys: Mutex::new(PartitionSystem::new(size)),
            buffer,
            alignment_type: alignment,
            info: info.build(),
        }))
    }
}

impl<M: MemorySource + DeviceSource, P: PartitionSource> BufferSource for Arc<Buffer<M, P>> {
    fn buffer(&self) -> &vk::Buffer {
        &self.buffer
    }

    fn home_partition(&self) -> &Partition {
        &self.memory_partition
    }

    fn partition(
        &self,
        size: u64,
        alignment_type: BufferAlignmentType,
    ) -> Result<Partition, PartitionError> {
        self.partition_sys
            .lock()
            .unwrap()
            .partition(size, |offset| {
                if let BufferAlignmentType::Aligned(a) = alignment_type {
                    return offset % a == 0;
                }
                if let BufferAlignmentType::Aligned(a) = self.alignment_type {
                    return offset % a == 0;
                }
                true
            })
    }

    fn usage(&self) -> vk::BufferUsageFlags {
        self.info.usage
    }
}

impl<M: MemorySource + DeviceSource, P: PartitionSource> Drop for Buffer<M, P> {
    fn drop(&mut self) {
        debug!("Destroyed buffer {:?}", self.buffer);
        unsafe {
            self.memory.device().destroy_buffer(self.buffer, None);
        }
    }
}

impl SettingsStore {
    pub fn new(size: vk::DeviceSize, usage: vk::BufferUsageFlags) -> SettingsStore {
        SettingsStore {
            size,
            flags: None,
            extensions: None,
            usage,
            share: None,
        }
    }

    pub fn set_create_flags(&mut self, flags: vk::BufferCreateFlags) {
        self.flags = Some(flags);
    }

    pub fn add_extension(&mut self, ext: BufferCreateExtension) {
        self.extensions.get_or_insert(vec![]).push(ext);
    }

    pub fn share(&mut self, indecies: &[u32]) {
        self.share = Some(indecies.to_vec());
    }
}

impl BufferSettingsStore for SettingsStore {
    fn size(&self) -> vk::DeviceSize {
        self.size
    }

    fn flags(&self) -> Option<vk::BufferCreateFlags> {
        self.flags
    }

    fn extensions(&self) -> Option<Vec<BufferCreateExtension>> {
        self.extensions.clone()
    }

    fn usage(&self) -> vk::BufferUsageFlags {
        self.usage
    }

    fn share(&self) -> Option<Vec<u32>> {
        self.share.clone()
    }
}

impl<P: PartitionSource, M: MemorySource + DeviceSource + InstanceSource> InstanceSource
    for Arc<Buffer<M, P>>
{
    fn instance(&self) -> &ash::Instance {
        self.memory.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.memory.entry()
    }
}

impl<P: PartitionSource, M: MemorySource + DeviceSource> DeviceSource for Arc<Buffer<M, P>> {
    fn device(&self) -> &ash::Device {
        self.memory.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.memory.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.memory.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.memory.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.memory.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.memory.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.memory.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.memory.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.memory.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.memory.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.memory.host_memory_index()
    }
}

impl<P: PartitionSource, M: MemorySource + DeviceSource> MemorySource for Arc<Buffer<M, P>> {
    fn partition(
        &self,
        size: u64,
        alignment: Option<u64>,
    ) -> Result<Partition, partitionsystem::PartitionError> {
        self.memory.partition(size, alignment)
    }

    fn memory(&self) -> &vk::DeviceMemory {
        self.memory.memory()
    }
}
