use std::{sync::{Arc, Mutex}, mem::size_of};

use ash::vk::{self, BufferUsageFlags};
use log::{info, debug};

use crate::{memory::{Partition, memory::{MemoryStore, InternalMemoryStore}, PartitionSystem, partitionsystem::{PartitionError, PartitionStore}, buffer::buffer::BufferAlignmentType}, image::{image::{ImageStore, InternalImageStore}, imageresource::ImageSubresourceStore}, init::{instance::{InstanceStore, InternalInstanceStore}, device::{DeviceStore, InternalDeviceStore}}, command::{commandpool, CommandPool, commandset::{self, CommandBufferFactory}, CommandSet, CommandBufferStore}, queue::{SubmitSet, Queue, queue::QueueStore}, descriptor::descriptorlayout::DescriptorLayoutBindingStore};

use super::{buffer::{BufferStore, InternalBufferStore}, BufferSegment};

pub trait BufferSegmentStore{
    fn get_partition(&self) -> &Partition;
    fn device_addr(&self) -> vk::DeviceSize;
    fn copy_from_ram<T>(&self, src: &[T]) -> Result<(), BufferSegmentMemOpError>;
    fn copy_to_ram<T>(&self, dst: &mut [T]) -> Result<(), BufferSegmentMemOpError>;
    fn copy_to_partition<B:BufferStore, BP:BufferSegmentStore + InternalBufferStore<B>,C: CommandBufferStore>(&self, cmd: &Arc<C>, dst: &Arc<BP>) -> Result<(), BufferSegmentMemOpError>;
    fn copy_to_partition_internal<B:BufferStore, BP:BufferSegmentStore + InternalBufferStore<B>>(&self, dst: &Arc<BP>) -> Result<(), BufferSegmentMemOpError>;
    ///Addressing is (bufferRowLength, bufferImageHeight)
    fn copy_to_image<I:ImageStore, IS:ImageSubresourceStore + InternalImageStore<I>,C: CommandBufferStore>(&self, cmd: &Arc<C>, dst: &Arc<IS>, buffer_addressing: Option<(u32, u32)>) -> Result<(), vk::Result>;
    ///Addressing is (bufferRowLength, bufferImageHeight)
    fn copy_to_image_internal<I:ImageStore, IS:ImageSubresourceStore + InternalImageStore<I>>(&self,dst: &Arc<IS>, buffer_addressing: Option<(u32, u32)>) -> Result<(), vk::Result>;
}

#[derive(Clone, Debug)]
pub enum BufferSegmentMemOpError{
    NoSpace,
    VulkanError(vk::Result),
}


impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, M:MemoryStore, B:BufferStore + InternalMemoryStore<M> + InternalDeviceStore<D>> BufferSegment<I,D,M,B,PartitionSystem>{
    pub fn new(buffer_provider: &Arc<B>, size: u64, custom_alignment: Option<u64>) -> Result<Arc<Self>, PartitionError>{
        
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

impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, M:MemoryStore, B:BufferStore + InternalMemoryStore<M> + InternalDeviceStore<D>, P:PartitionStore> BufferSegmentStore for BufferSegment<I,D,M,B,P>{
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

    fn copy_to_partition<Buf:BufferStore, BP:BufferSegmentStore + InternalBufferStore<Buf>, C:CommandBufferStore>(&self, cmd: &Arc<C>, dst: &Arc<BP>) -> Result<(), BufferSegmentMemOpError> {
        if self.partition.size > dst.get_partition().size(){
            return Err(BufferSegmentMemOpError::NoSpace);
        }

        let op = [vk::BufferCopy::builder()
        .src_offset(self.partition.offset)
        .dst_offset(dst.get_partition().offset)
        .size(self.partition.size)
        .build()];

        unsafe{
            let device = self.buffer.device_provider().device();
            device.cmd_copy_buffer(cmd.cmd(), *self.buffer.buffer(), *dst.buffer_provider().buffer(), &op);
        }
        Ok(())
    }

    fn copy_to_partition_internal<Buf:BufferStore, BP:BufferSegmentStore + InternalBufferStore<Buf>>(&self, dst: &Arc<BP>) -> Result<(), BufferSegmentMemOpError> {
        let settings = commandpool::SettingsStore::new(self.buffer_provider().device_provider().transfer_queue().unwrap().1);
        let pool = CommandPool::new(&settings, self.buffer.device_provider()).unwrap();
        let mut settings = commandset::SettingsStore::default(); settings.batch_size = 1;
        let cmd_set = CommandSet::new(&settings, self.buffer.device_provider(), &pool);
        let cmd = cmd_set.next_cmd();
        
        cmd.begin(None).unwrap();
        self.copy_to_partition(&cmd, dst)?;
        cmd.end().unwrap();
        
        let submit = SubmitSet::new(&cmd);
        let submit = [submit];
        let queue = Queue::new(self.buffer.device_provider(), vk::QueueFlags::TRANSFER).unwrap();
        queue.wait_submit(&submit).expect("Could not execute transfer");
        Ok(())
    }

    fn copy_to_image<Img:ImageStore, IS:ImageSubresourceStore + InternalImageStore<Img>, C:CommandBufferStore>(&self, cmd: &Arc<C>, dst: &Arc<IS>, buffer_addressing: Option<(u32, u32)>) -> Result<(), vk::Result> {
        if dst.extent().width == 0{
            return Ok(());
        }
        if dst.extent().height== 0{
            return Ok(());
        }
        if dst.extent().depth == 0{
            return Ok(());
        }
        
        let buffer_offset = self.partition.offset();
        let mut addressing = (0,0);
        if let Some(a) = buffer_addressing{
            addressing = a;
        }

        let subresource = dst.subresource();
        let offset = dst.offset();
        let extent = dst.extent();
        let image = dst.image_provider().image();
        let layout = dst.layout();
        
        let info = [vk::BufferImageCopy::builder()
        .buffer_offset(buffer_offset)
        .buffer_row_length(addressing.0)
        .buffer_image_height(addressing.1)
        .image_subresource(subresource)
        .image_offset(offset)
        .image_extent(extent)
        .build()];

        unsafe{
            let device = self.buffer.device_provider().device();
            debug!("Copying {:?} bytes from buffer {:?} to layer {:?} of image {:?}", self.partition.size, *self.buffer.buffer(), dst.subresource(), *image);
            device.cmd_copy_buffer_to_image(cmd.cmd(), *self.buffer.buffer(), *image, *layout, &info);
        }

        Ok(())
    }

    fn copy_to_image_internal<Img:ImageStore, IS:ImageSubresourceStore + InternalImageStore<Img>>(&self,dst: &Arc<IS>, buffer_addressing: Option<(u32, u32)>) -> Result<(), vk::Result> {
        let settings = commandpool::SettingsStore::new(self.buffer_provider().device_provider().transfer_queue().unwrap().1);
        let pool = CommandPool::new(&settings, self.buffer.device_provider()).unwrap();
        let mut settings = commandset::SettingsStore::default(); settings.batch_size = 1;
        let cmd_set = CommandSet::new(&settings, self.buffer.device_provider(), &pool);
        let cmd = cmd_set.next_cmd();
        
        cmd.begin(None).unwrap();
        self.copy_to_image(&cmd, dst, buffer_addressing)?;
        cmd.end().unwrap();
        
        let submit = SubmitSet::new(&cmd);
        let submit = [submit];
        let queue = Queue::new(self.buffer.device_provider(), vk::QueueFlags::TRANSFER).unwrap();
        queue.wait_submit(&submit).expect("Could not execute transfer");
        Ok(())
    }


}

impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, M:MemoryStore, B:BufferStore + InternalMemoryStore<M> + InternalDeviceStore<D>, P:PartitionStore> InternalBufferStore<B> for BufferSegment<I,D,M,B,P>{
    fn buffer_provider(&self) -> &Arc<B> {
        &self.buffer
    }
}

impl<I:InstanceStore, D:DeviceStore + InternalInstanceStore<I>, M:MemoryStore, B:BufferStore + InternalMemoryStore<M> + InternalDeviceStore<D>> DescriptorLayoutBindingStore for BufferSegment<I,D,M,B,PartitionSystem>{
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
