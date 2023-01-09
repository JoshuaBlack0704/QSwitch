use std::sync::Arc;

use crate::init::DeviceSource;

use super::{BufferSegmentSource, BufferSegment, BufferSource, BufferSegmentFactory};

impl<B:DeviceSource + BufferSource + Clone> BufferSegmentFactory for B{
    type Segment = Arc<BufferSegment<B>>;

    fn get_segment(&self, size: u64, alignment: Option<u64>) -> Self::Segment {
        let partition = self.get_space(size,alignment);
        Arc::new(
            BufferSegment{
                buffer: self.clone(),
                mem_part: partition.0,
                buf_part: partition.1,
            }
        )
    }
}

impl<B:DeviceSource + BufferSource> BufferSegmentSource for Arc<BufferSegment<B>>{
    
}
