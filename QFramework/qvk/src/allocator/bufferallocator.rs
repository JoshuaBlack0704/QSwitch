use std::sync::{Arc, Mutex};

use ash::vk;

use crate::init::{DeviceSource, InstanceSource};

use super::{BufferAllocator, MemoryAllocator, BufferExtensions, BufferSegmentFactory, BufferSegment, BufferSegmentSource};

impl<D:DeviceSource + Clone> BufferAllocator<D>{
    pub fn new(
    device_source: &D, 
    mem: Arc<MemoryAllocator<D>>, 
    min_size: u64, 
    usage: vk::BufferUsageFlags, 
    flags: Option<vk::BufferCreateFlags>, 
    share: Option<Vec<u32>>, 
    extensions: &[BufferExtensions]) -> Arc<BufferAllocator<D>> {
        Arc::new(
            Self{
                device: device_source.clone(),
                min_size,
                usage,
                flags,
                extensions: extensions.to_vec(),
                share,
                mem,
                buffers: Mutex::new(vec![]),
            }
        )
    }
}

impl<D:DeviceSource + Clone + InstanceSource> BufferSegmentFactory for Arc<BufferAllocator<D>>{
    type Segment = Arc<BufferSegment<D>>;

    fn get_segment(&self, size: u64, alignment: Option<u64>) -> Self::Segment {
        let buffers = self.buffers.lock().unwrap();
        for (mem, b, p) in buffers.iter(){
            if let Of(p) = super::test_partition(&p, size, alignment){
               return BufferSegment::new(self.clone(),) 
            }
        }
        

        todo!()
    }
}