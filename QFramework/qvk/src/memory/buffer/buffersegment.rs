use std::{mem::size_of, sync::{Arc, Mutex}};

use ash::vk::{self, BufferUsageFlags};
use log::{debug, info};

use crate::{command::{CommandBufferStore, commandpool, CommandPool, commandset, CommandSet, BufferCopyFactory, ImageCopyFactory}, init::{DeviceStore, InstanceStore, InternalInstanceStore, InternalDeviceStore}, memory::{buffer::buffer::BufferAlignmentType, Partition, PartitionSystem, partitionsystem::PartitionError}, queue::{Queue, SubmitSet, QueueOps}};
use crate::command::CommandBufferFactory;
use crate::descriptor::DescriptorLayoutBindingFactory;
use crate::image::{ImageStore, ImageSubresourceStore, InternalImageStore};
use crate::memory::{InternalMemoryStore, MemoryStore, PartitionStore};
use crate::memory::buffer::{BufferSegmentStore, BufferStore, InternalBufferStore};

use super::BufferSegment;

#[derive(Clone, Debug)]
pub enum BufferSegmentMemOpError{
    NoSpace,
    VulkanError(vk::Result),
}


impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, M:MemoryStore, B:BufferStore + InternalMemoryStore<M> + InternalDeviceStore<D> + Clone> BufferSegment<I,D,M,B,PartitionSystem>{
    pub fn new(buffer_provider: &B, size: u64, custom_alignment: Option<u64>) -> Result<Arc<Self>, PartitionError>{
        
        let p;
        if let Some(a) = custom_alignment{
            p = buffer_provider.partition(size, BufferAlignmentType::Aligned(a));
        }
        else{
            p = buffer_provider.partition(size, BufferAlignmentType::Free);
        }

        if let Err(e) = p{
            return Err(e);
        }


        let p = p.unwrap();
        info!("Partitioned buffer {:?} at offset {:?} for {:?} bytes", *buffer_provider.buffer(), p.offset, p.size);
        Ok(
            Arc::new(BufferSegment{
            buffer: buffer_provider.clone(),
            _partition_sys: Mutex::new(PartitionSystem::new(p.size)),
            partition: p,
            _device_addr: None,
            _instance: std::marker::PhantomData,
            _memory: std::marker::PhantomData,
            _device: std::marker::PhantomData,
            })
        )
                
    }
}

impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I> + Clone, M:MemoryStore, B:BufferStore + InternalMemoryStore<M> + InternalDeviceStore<D>, P:PartitionStore> BufferSegmentStore for Arc<BufferSegment<I,D,M,B,P>>{
    fn get_partition(&self) -> &Partition {
        &self.partition
    }

    fn device_addr(&self) -> vk::DeviceSize {
        let device = self.buffer.device_provider();
        let ext = ash::extensions::khr::BufferDeviceAddress::new(device.instance_provider().instance(), device.device());
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
        let target_memory = self.buffer.memory_provider().memory();
        let mapped_range = [vk::MappedMemoryRange::builder()
        .memory(*target_memory)
        .offset(0)
        .size(vk::WHOLE_SIZE)
        .build()];
        
        debug!("Copying {:?} bytes to buffer {:?} at offset {:?} from ram", needed_size, *self.buffer.buffer(), target_offset);
        unsafe{
            let device = self.buffer.device_provider().device();
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
        let target_memory = self.buffer.memory_provider().memory();
        let mapped_range = [vk::MappedMemoryRange::builder()
        .memory(*target_memory)
        .offset(0)
        .size(vk::WHOLE_SIZE)
        .build()];
        
        debug!("Copying {:?} bytes from buffer {:?} at offset {:?} to ram", needed_size, *self.buffer.buffer(), target_offset);
        unsafe{
            let device = self.buffer.device_provider().device();
            let src = device.map_memory(*target_memory, 0, vk::WHOLE_SIZE, vk::MemoryMapFlags::empty()).unwrap();
            let src = (src as *mut u8).offset(target_offset as isize) as *mut T;
            let dst_ptr = dst.as_mut_ptr();
            device.invalidate_mapped_memory_ranges(&mapped_range).unwrap();
            std::ptr::copy_nonoverlapping(src, dst_ptr, dst.len());
            device.unmap_memory(*target_memory);            
        }
        Ok(())
    }

    fn copy_to_segment_internal<Buf:BufferStore, BP:BufferCopyFactory + InternalBufferStore<Buf>>(&self, dst: &BP) -> Result<(), BufferSegmentMemOpError> {
        let settings = commandpool::SettingsStore::new(self.buffer_provider().device_provider().transfer_queue().unwrap().1);
        let pool = CommandPool::new(&settings, self.buffer.device_provider()).unwrap();
        let mut settings = commandset::SettingsStore::default(); settings.batch_size = 1;
        let cmd_set = CommandSet::new(&settings, self.buffer.device_provider(), &pool);
        let cmd = cmd_set.next_cmd();
        
        cmd.begin(None).unwrap();
        cmd.buffer_copy(self, dst);
        cmd.end().unwrap();
        
        let submit = SubmitSet::new(&cmd);
        let submit = [submit];
        let queue = Queue::new(self.buffer.device_provider(), vk::QueueFlags::TRANSFER).unwrap();
        queue.wait_submit(&submit).expect("Could not execute transfer");
        Ok(())
    }

    fn copy_to_image_internal<Img:ImageStore, IS: InternalImageStore<Img> + ImageCopyFactory>(&self,dst: &IS, buffer_addressing: Option<(u32, u32)>) -> Result<(), vk::Result> {
        let settings = commandpool::SettingsStore::new(self.buffer_provider().device_provider().transfer_queue().unwrap().1);
        let pool = CommandPool::new(&settings, self.buffer.device_provider()).unwrap();
        let mut settings = commandset::SettingsStore::default(); settings.batch_size = 1;
        let cmd_set = CommandSet::new(&settings, self.buffer.device_provider(), &pool);
        let cmd = cmd_set.next_cmd();
        
        cmd.begin(None).unwrap();
        cmd.buffer_image_copy(self, dst, buffer_addressing);
        // self.copy_to_image(&cmd, dst, buffer_addressing)?;
        cmd.end().unwrap();
        
        let submit = SubmitSet::new(&cmd);
        let submit = [submit];
        let queue = Queue::new(self.buffer.device_provider(), vk::QueueFlags::TRANSFER).unwrap();
        queue.wait_submit(&submit).expect("Could not execute transfer");
        Ok(())
    }


}

impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, M:MemoryStore, B:BufferStore + InternalMemoryStore<M> + InternalDeviceStore<D>, P:PartitionStore> BufferCopyFactory for Arc<BufferSegment<I,D,M,B,P>>{
    fn size(&self) -> u64 {
        self.partition.size
    }

    fn offset(&self) -> u64 {
        self.partition.offset
    }
}
    

impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, M:MemoryStore, B:BufferStore + InternalMemoryStore<M> + InternalDeviceStore<D>, P:PartitionStore> InternalBufferStore<B> for Arc<BufferSegment<I,D,M,B,P>>{
    fn buffer_provider(&self) -> &B {
        &self.buffer
    }
}

impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, M:MemoryStore, B:BufferStore + InternalMemoryStore<M> + InternalDeviceStore<D>> DescriptorLayoutBindingFactory for Arc<BufferSegment<I,D,M,B,PartitionSystem>>{
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
