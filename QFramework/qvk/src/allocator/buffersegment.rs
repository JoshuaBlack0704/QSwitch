use std::sync::Arc;

use crate::init::DeviceSource;

use super::{BufferSegmentSource, BufferSegment, MemPart, BufPart, BufferAllocator};

impl<D:DeviceSource> BufferSegmentSource for Arc<BufferSegment<D>>{
    
}

impl<D:DeviceSource> BufferSegment<D>{
    pub fn new(buf: Arc<BufferAllocator<D>>, mem_part: MemPart, buf_part: BufPart) -> Arc<BufferSegment<D>> {
        Arc::new(
            Self{
                buffer: buf.clone(),
                mem_part,
                buf_part,
            }
        )
    }
}