use std::{mem::size_of, sync::Arc};

use ash::vk::{self, BufferUsageFlags};
use log::{debug, info};

use crate::{command::{CommandBufferSource,  BufferCopyFactory, ImageCopyFactory, Executor}, init::{DeviceSource, InstanceSource}, memory::{buffer::buffer::BufferAlignmentType, Partition, partitionsystem::PartitionError}, descriptor::{WriteSource, ApplyWriteFactory}};
use crate::command::CommandBufferFactory;
use crate::descriptor::DescriptorLayoutBindingFactory;
use crate::image::ImageSource;
use crate::memory::MemorySource;
use crate::memory::buffer::{BufferSegmentSource, BufferSource};

use super::{BufferSegment, BufferSegmentFactory};

#[derive(Clone, Debug)]
pub enum BufferSegmentMemOpError{
    NoSpace,
    VulkanError(vk::Result),
}

impl<B:BufferSource + MemorySource + DeviceSource + InstanceSource + Clone> BufferSegmentFactory<Arc<BufferSegment<B>>> for B{
    fn create_segment(&self, size: u64, alignment: Option<u64>) -> Result<Arc<BufferSegment<B>>, PartitionError> {
        let p;
        if let Some(a) = alignment{
            p = BufferSource::partition(self, size, BufferAlignmentType::Aligned(a));
        }
        else{
            p = BufferSource::partition(self, size, BufferAlignmentType::Free);
        }

        if let Err(e) = p{
            return Err(e);
        }


        let p = p.unwrap();

        let b_info = vk::DescriptorBufferInfo::builder()
        .buffer(*self.buffer())
        .offset(p.offset)
        .range(p.size)
        .build();
        info!("Partitioned buffer {:?} at offset {:?} for {:?} bytes", *self.buffer(), p.offset, p.size);
        Ok(
            Arc::new(BufferSegment{
            buffer: self.clone(),
            partition: p,
            desc_buffer_info: [b_info],
            _device_addr: None,
            })
        )
    }
}

impl<B:BufferSource + MemorySource + DeviceSource + InstanceSource + Clone> BufferSegmentSource for Arc<BufferSegment<B>>{
    fn get_partition(&self) -> &Partition {
        &self.partition
    }

    fn device_addr(&self) -> vk::DeviceSize {
        let device = self;
        let ext = ash::extensions::khr::BufferDeviceAddress::new(device.instance(), device.device());
        let info = vk::BufferDeviceAddressInfo::builder()
        .buffer(*self.buffer.buffer());
        let addr = unsafe{ext.get_buffer_device_address(&info)} + self.partition.offset();
        addr
    }

    fn copy_from_ram<T>(&self, src: &[T]) -> Result<(), BufferSegmentMemOpError> {
        let needed_size = size_of::<T>() * src.len();
        if needed_size > self.partition.size as usize{
            return Err(BufferSegmentMemOpError::NoSpace);
        }

        let target_offset = self.partition.offset + self.buffer.home_partition().offset;
        let target_memory = self.memory();
        let mapped_range = [vk::MappedMemoryRange::builder()
        .memory(*target_memory)
        .offset(0)
        .size(vk::WHOLE_SIZE)
        .build()];
        
        debug!("Copying {:?} bytes to buffer {:?} at offset {:?} from ram", needed_size, *self.buffer.buffer(), target_offset);
        unsafe{
            let device = self.device();
            let dst = device.map_memory(*target_memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()).unwrap();
            let dst = (dst as *mut u8).offset(target_offset as isize) as *mut T;
            let src_ptr = src.as_ptr();
            std::ptr::copy_nonoverlapping(src_ptr, dst, src.len());
            device.flush_mapped_memory_ranges(&mapped_range).unwrap();
            device.unmap_memory(*target_memory);            
        }
        Ok(())
    }

    fn copy_to_ram<T>(&self, dst: &mut [T]) -> Result<(), BufferSegmentMemOpError> {
        let needed_size = self.partition.size;
        if needed_size as usize > dst.len() * size_of::<T>(){
            return Err(BufferSegmentMemOpError::NoSpace)
        }

        let target_offset = self.partition.offset + self.buffer.home_partition().offset;
        let target_memory = self.memory();
        let mapped_range = [vk::MappedMemoryRange::builder()
        .memory(*target_memory)
        .offset(0)
        .size(vk::WHOLE_SIZE)
        .build()];
        
        debug!("Copying {:?} bytes from buffer {:?} at offset {:?} to ram", needed_size, *self.buffer.buffer(), target_offset);
        unsafe{
            let device = self.device();
            let src = device.map_memory(*target_memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()).unwrap();
            let src = (src as *mut u8).offset(target_offset as isize) as *mut T;
            let dst_ptr = dst.as_mut_ptr();
            device.invalidate_mapped_memory_ranges(&mapped_range).unwrap();
            std::ptr::copy_nonoverlapping(src, dst_ptr, dst.len());
            device.unmap_memory(*target_memory);            
        }
        Ok(())
    }

    fn copy_to_segment_internal< BP:BufferCopyFactory + BufferSource>(&self, dst: &BP) -> Result<(), BufferSegmentMemOpError> {
        let exe = Executor::new(self, vk::QueueFlags::TRANSFER);
        
        let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        cmd.buffer_copy(self, dst).unwrap();
        cmd.end().unwrap();
        
        exe.wait_submit_internal();
        Ok(())
    }

    fn copy_to_image_internal<IS: ImageSource + ImageCopyFactory>(&self, dst: &IS, buffer_addressing: Option<(u32, u32)>) -> Result<(), vk::Result> {
        let exe = Executor::new(self, vk::QueueFlags::TRANSFER);
        
        let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
        cmd.begin(None).unwrap();
        cmd.buffer_image_copy(self, dst, buffer_addressing).unwrap();
        // self.copy_to_image(&cmd, dst, buffer_addressing)?;
        cmd.end().unwrap();
        
        exe.wait_submit_internal();
        Ok(())
    }


}

impl<B:BufferSource + MemorySource + DeviceSource> BufferCopyFactory for Arc<BufferSegment<B>>{
    fn size(&self) -> u64 {
        self.partition.size
    }

    fn offset(&self) -> u64 {
        self.partition.offset
    }

    fn buffer(&self) -> vk::Buffer {
        *BufferSource::buffer(self)
    }
}
    

impl<B:BufferSource + MemorySource + DeviceSource> BufferSource for Arc<BufferSegment<B>>{
    fn buffer(&self) -> &vk::Buffer {
        self.buffer.buffer()
    }

    fn home_partition(&self) -> &Partition {
        self.buffer.home_partition()
    }

    fn partition(&self, size: u64, alignment_type: BufferAlignmentType) -> Result<Partition, PartitionError>  {
        BufferSource::partition(&self.buffer, size, alignment_type)
    }

    fn usage(&self) -> vk::BufferUsageFlags {
        self.buffer.usage()
    }
}

impl<B:BufferSource + MemorySource + DeviceSource> DescriptorLayoutBindingFactory for Arc<BufferSegment<B>>{
    fn binding(&self) -> vk::DescriptorSetLayoutBinding {
        let binding_type ;
        let usage = self.buffer.usage();
        if usage.contains(BufferUsageFlags::STORAGE_BUFFER){
            binding_type = vk::DescriptorType::STORAGE_BUFFER;
        }
        else if usage.contains(BufferUsageFlags::UNIFORM_BUFFER){
            binding_type = vk::DescriptorType::UNIFORM_BUFFER;
        }
        else{
            unimplemented!();
        }

        vk::DescriptorSetLayoutBinding::builder()
        .descriptor_type(binding_type)
        .descriptor_count(1)
        .build()
    }
}

impl<B:BufferSource + MemorySource + DeviceSource> ApplyWriteFactory for Arc<BufferSegment<B>>{
    fn apply<W:WriteSource>(&self, write: &W) {

        let info = vk::WriteDescriptorSet::builder()
        .dst_array_element(0)
        .buffer_info(&self.desc_buffer_info)
        .build();

        write.update(info);
    }
}

impl<B:BufferSource + MemorySource + DeviceSource + InstanceSource> InstanceSource for Arc<BufferSegment<B>>{
    
    fn instance(&self) -> &ash::Instance {
        self.buffer.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.buffer.entry()
    }
}

impl<B:BufferSource + MemorySource + DeviceSource> MemorySource for Arc<BufferSegment<B>>{
    fn partition(&self, size: u64, alignment: Option<u64>) -> Result<Partition, crate::memory::partitionsystem::PartitionError> {
        MemorySource::partition(&self.buffer, size, alignment)
    }

    fn memory(&self) -> &vk::DeviceMemory {
        self.buffer.memory()
    }
}

impl<B:BufferSource + MemorySource + DeviceSource> DeviceSource for Arc<BufferSegment<B>>{
    
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
