use std::sync::{Arc, Mutex};

use ash::vk;

use crate::device;

use super::{BufferPartition, buffer::{self, BufferAlignmentType}, partitionsystem, PartitionSystem, Partition};

pub trait BufferPartitioner{
    type Provider: BufferPartitionProvider;
    /// This get a partition aligned to whatever alignment the provider chooses internally
    fn partition(&self, size: u64) -> Arc<Self::Provider>;
    /// This gets a partition with a provided alignment
    fn partition_aligned(&self, size: u64, alignment: BufferAlignmentType) -> Arc<Self::Provider>;
    
}

pub trait BufferPartitionProvider{
    fn alignment_type(&self) -> BufferAlignmentType;
    fn device_addr(&self) -> vk::DeviceSize;
}


impl<D:device::DeviceProvider, B:buffer::BufferProvider> BufferPartition<D,B,PartitionSystem>{
    pub(crate) fn new(device_provider: &Arc<D>, buffer_provider: &Arc<B>, partition: Partition) -> Arc<Self>{
        
        
        Arc::new(BufferPartition{
            device: device_provider.clone(),
            buffer: buffer_provider.clone(),
            partition_sys: Mutex::new(PartitionSystem::new(partition.size)),
            partition,
            device_addr: None,
        })
    }
}
impl<D:device::DeviceProvider, B:buffer::BufferProvider, P:partitionsystem::PartitionProvider> BufferPartitionProvider for BufferPartition<D,B,P>{
    
}

