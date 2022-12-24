use std::sync::{Arc, Mutex};

use ash::vk;

use super::{WriteHolder, WriteSource};

impl WriteHolder{
    pub fn new(ty: vk::DescriptorType, dst_binding: u32, write: vk::WriteDescriptorSet) -> Arc<WriteHolder> {
        Arc::new(
            Self{
                write: Mutex::new(write),
                needs_update: Mutex::new(true),
                ty,
                dst_binding,
            }
        )
    }
}

impl WriteSource for Arc<WriteHolder>{
    fn update(&self, mut write: vk::WriteDescriptorSet) {
        let mut lock = self.write.lock().unwrap();
        let mut signal = self.needs_update.lock().unwrap();
        write.dst_binding = self.dst_binding;
        write.descriptor_type = self.ty;

        *lock = write;
        *signal = true;
    }

    fn needs_write(&self) -> bool {
        *self.needs_update.lock().unwrap()
    }

    fn get_write(&self) -> vk::WriteDescriptorSet {
        let mut signal = self.needs_update.lock().unwrap();
        let write = self.write.lock().unwrap();
        *signal = false;
        *write
    }
}