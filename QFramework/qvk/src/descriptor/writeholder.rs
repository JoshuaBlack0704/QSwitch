use std::sync::{Arc, Mutex};

use ash::vk;

use super::WriteHolder;

impl WriteHolder{
    pub fn new(write: vk::WriteDescriptorSet) -> Arc<WriteHolder> {
        Arc::new(
            Self{
                write: Mutex::new(write),
            }
        )
    }
}