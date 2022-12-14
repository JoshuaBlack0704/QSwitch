use std::sync::{Arc, Mutex};

use ash::vk;

use crate::{device, instance};

use super::{BufferPartition, PartitionSystem, Partition, buffer, partitionsystem::{self, PartitionError}};

pub trait BufferPartitionProvider{
    fn get_partition(&self) -> &Partition;
    fn device_addr(&self) -> vk::DeviceSize;
}


impl<D:device::DeviceProvider, B:buffer::BufferProvider> BufferPartition<D,B,PartitionSystem>{
    pub fn new(device_provider: &Arc<D>, buffer_provider: &Arc<B>, size: u64, custom_alignment: Option<u64>) -> Result<Arc<Self>, PartitionError>{
        
        let p;
        if let Some(a) = custom_alignment{
            p = buffer_provider.partition(size, buffer::BufferAlignmentType::Aligned(a));
        }
        else{
            p = buffer_provider.partition(size, buffer::BufferAlignmentType::Free);
        }

        if let Err(e) = p{
            return Err(e);
        }

        let p = p.unwrap();
        Ok(
            Arc::new(BufferPartition{
            device: device_provider.clone(),
            buffer: buffer_provider.clone(),
            partition_sys: Mutex::new(PartitionSystem::new(p.size)),
            partition: p,
            device_addr: None,
            })
        )
                
    }
}

impl<D:device::DeviceProvider, B:buffer::BufferProvider, P:partitionsystem::PartitionProvider> BufferPartitionProvider for BufferPartition<D,B,P>{
    fn get_partition(&self) -> &Partition {
        &self.partition
    }

    fn device_addr(&self) -> vk::DeviceSize {
        let ext = ash::extensions::khr::BufferDeviceAddress::new(self.device.instance(), self.device.device());
        let info = vk::BufferDeviceAddressInfo::builder()
        .buffer(*self.buffer.buffer());
        let addr = unsafe{ext.get_buffer_device_address(&info)} + self.partition.offset();
        addr
    }
}

