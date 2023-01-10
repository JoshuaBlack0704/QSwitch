use std::{mem::size_of, sync::Arc};

use ash::vk;

use crate::{
    command::{
        BindIndexBufferFactory, BindVertexBufferFactory, BufferCopyFactory, CommandBufferFactory,
        CommandBufferSource, Executor, ImageCopyFactory,
    },
    descriptor::{ApplyWriteFactory, DescriptorLayoutBindingFactory},
    init::{DeviceSource, InstanceSource},
    memory::{allocators::BufferSource, image::ImageSource},
};

use super::{BufferSegment, BufferSegmentFactory, BufferSegmentMemOpError, BufferSegmentSource};

impl<B: DeviceSource + BufferSource + Clone> BufferSegmentFactory for B {
    type Segment = Arc<BufferSegment<B>>;

    fn get_segment(&self, size: u64, alignment: Option<u64>) -> Self::Segment {
        let partition = self.get_space(size, alignment);
        let info = [vk::DescriptorBufferInfo::builder()
            .buffer(partition.1 .0)
            .offset(partition.1 .1.offset)
            .range(partition.1 .1.size)
            .build()];

        Arc::new(BufferSegment {
            buffer: self.clone(),
            mem_part: partition.0,
            buf_part: partition.1,
            desc_buffer_info: info,
        })
    }
}

impl<B: DeviceSource + BufferSource> BufferCopyFactory for Arc<BufferSegment<B>> {
    fn size(&self) -> u64 {
        self.buf_part.1.size
    }

    fn offset(&self) -> u64 {
        self.buf_part.1.offset
    }

    fn buffer(&self) -> vk::Buffer {
        self.buf_part.0
    }
}

impl<B: DeviceSource + BufferSource> BufferSegmentSource for Arc<BufferSegment<B>> {
    fn size(&self) -> u64 {
        self.buf_part.1.size
    }

    fn offset(&self) -> u64 {
        self.buf_part.1.offset
    }

    fn copy_from_ram<T>(&self, src: &[T]) -> Result<(), super::BufferSegmentMemOpError> {
        let needed_size = size_of::<T>() * src.len();
        if needed_size > self.buf_part.1.size as usize {
            return Err(BufferSegmentMemOpError::NoSpace);
        }

        let target_offset = self.buf_part.1.offset + self.mem_part.1.offset;
        let target_memory = self.mem_part.0;
        let mapped_range = [vk::MappedMemoryRange::builder()
            .memory(target_memory)
            .offset(0)
            .size(vk::WHOLE_SIZE)
            .build()];

        // debug!(
        //     "Copying {:?} bytes to buffer {:?} at offset {:?} from ram",
        //     needed_size,
        //     *self.buffer.buffer(),
        //     target_offset
        // );
        unsafe {
            let device = self.buffer.device();
            let dst = device
                .map_memory(
                    target_memory,
                    0,
                    vk::WHOLE_SIZE,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();
            let dst = (dst as *mut u8).offset(target_offset as isize) as *mut T;
            let src_ptr = src.as_ptr();
            std::ptr::copy_nonoverlapping(src_ptr, dst, src.len());
            device.flush_mapped_memory_ranges(&mapped_range).unwrap();
            device.unmap_memory(target_memory);
        }
        Ok(())
    }

    fn copy_to_ram<T>(&self, dst: &mut [T]) -> Result<(), super::BufferSegmentMemOpError> {
        let needed_size = self.buf_part.1.size;
        if needed_size as usize > dst.len() * size_of::<T>() {
            return Err(BufferSegmentMemOpError::NoSpace);
        }

        let target_offset = self.buf_part.1.offset + self.mem_part.1.offset;
        let target_memory = self.mem_part.0;
        let mapped_range = [vk::MappedMemoryRange::builder()
            .memory(target_memory)
            .offset(0)
            .size(vk::WHOLE_SIZE)
            .build()];

        // debug!(
        //     "Copying {:?} bytes from buffer {:?} at offset {:?} to ram",
        //     needed_size,
        //     *self.buffer.buffer(),
        //     target_offset
        // );
        unsafe {
            let device = self.buffer.device();
            let src = device
                .map_memory(
                    target_memory,
                    0,
                    vk::WHOLE_SIZE,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();
            let src = (src as *mut u8).offset(target_offset as isize) as *mut T;
            let dst_ptr = dst.as_mut_ptr();
            device
                .invalidate_mapped_memory_ranges(&mapped_range)
                .unwrap();
            std::ptr::copy_nonoverlapping(src, dst_ptr, dst.len());
            device.unmap_memory(target_memory);
        }
        Ok(())
    }

    fn copy_to_segment_internal<BP: BufferCopyFactory + BufferSegmentSource>(
        &self,
        dst: &BP,
    ) -> Result<(), super::BufferSegmentMemOpError> {
        let exe = Executor::new(self, vk::QueueFlags::TRANSFER);

        let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        cmd.buffer_copy(self, dst).unwrap();
        cmd.end().unwrap();

        exe.wait_submit_internal();
        Ok(())
    }

    fn copy_to_image_internal<I: ImageSource + ImageCopyFactory>(
        &self,
        dst: &I,
        buffer_addressing: Option<(u32, u32)>,
    ) -> Result<(), ash::vk::Result> {
        let exe = Executor::new(self, vk::QueueFlags::TRANSFER);

        let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        cmd.buffer_image_copy(self, dst, buffer_addressing).unwrap();
        cmd.end().unwrap();

        exe.wait_submit_internal();
        Ok(())
    }
}

impl<B: BufferSource + DeviceSource> DeviceSource for Arc<BufferSegment<B>> {
    fn device(&self) -> &ash::Device {
        self.buffer.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.buffer.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.buffer.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.buffer.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.buffer.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.buffer.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.buffer.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.buffer.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.buffer.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.buffer.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.buffer.host_memory_index()
    }
}

impl<B: BufferSource + DeviceSource + InstanceSource> InstanceSource for Arc<BufferSegment<B>> {
    fn instance(&self) -> &ash::Instance {
        self.buffer.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.buffer.entry()
    }
}

impl<B: BufferSource + DeviceSource> DescriptorLayoutBindingFactory for Arc<BufferSegment<B>> {
    fn binding(&self) -> vk::DescriptorSetLayoutBinding {
        let binding_type;
        let usage = self.buffer.usage();
        if usage.contains(vk::BufferUsageFlags::STORAGE_BUFFER) {
            binding_type = vk::DescriptorType::STORAGE_BUFFER;
        } else if usage.contains(vk::BufferUsageFlags::UNIFORM_BUFFER) {
            binding_type = vk::DescriptorType::UNIFORM_BUFFER;
        } else {
            unimplemented!();
        }

        vk::DescriptorSetLayoutBinding::builder()
            .descriptor_type(binding_type)
            .descriptor_count(1)
            .build()
    }
}

impl<B: BufferSource + DeviceSource> ApplyWriteFactory for Arc<BufferSegment<B>> {
    fn apply<W: crate::descriptor::WriteSource>(&self, write: &W) {
        let info = vk::WriteDescriptorSet::builder()
            .dst_array_element(0)
            .buffer_info(&self.desc_buffer_info)
            .build();

        write.update(info);
    }
}

impl<B: BufferSource + DeviceSource> BindIndexBufferFactory for Arc<BufferSegment<B>> {
    fn buffer(&self) -> vk::Buffer {
        self.buf_part.0
    }

    fn offset(&self) -> u64 {
        self.buf_part.1.offset
    }

    fn index_type(&self) -> vk::IndexType {
        vk::IndexType::UINT32
    }
}
impl<B: BufferSource + DeviceSource> BindVertexBufferFactory for Arc<BufferSegment<B>> {
    fn buffer(&self) -> vk::Buffer {
        self.buf_part.0
    }

    fn offset(&self) -> u64 {
        self.buf_part.1.offset
    }
}
