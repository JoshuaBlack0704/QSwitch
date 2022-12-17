use ash::vk;

use crate::sync::semaphore::SemaphoreProvider;

use super::SubmitSet;

pub trait SubmitInfoProvider{
    fn info(&self) -> vk::SubmitInfo;
    fn add_cmd(&mut self);
    fn add_wait(&mut self, stage: vk::PipelineStageFlags);
    fn add_signal(&mut self);
}



impl<'a,S:SemaphoreProvider> SubmitSet<'a,S>{
    pub fn new() -> Self {
        Self{
            wait_semaphores: vec![],
            cmds: vec![],
            signal_semaphores: vec![],
        }
    }
}

impl<'a,S:SemaphoreProvider> SubmitInfoProvider for SubmitSet<'a,S>{
    fn info(&self) -> vk::SubmitInfo {
        vk::SubmitInfo
        todo!()
    }

    fn add_cmd(&mut self) {
        
        todo!()
    }

    fn add_wait(&mut self, stage: vk::PipelineStageFlags) {
        todo!()
    }

    fn add_signal(&mut self) {
        todo!()
    }
}
