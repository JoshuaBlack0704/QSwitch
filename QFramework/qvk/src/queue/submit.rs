use std::sync::Arc;

use ash::vk;

use crate::sync::semaphore::SemaphoreProvider;

use super::SubmitSet;

pub trait SubmitInfoProvider{
    fn info(&self) -> vk::SubmitInfo2;
    fn add_cmd(&mut self, cmd: Arc<vk::CommandBuffer>);
    fn add_wait<S:SemaphoreProvider>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2);
    fn add_signal<S:SemaphoreProvider>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2);
}



impl SubmitSet{
    pub fn new() -> Self {
        Self{
            wait_semaphores: vec![],
            cmds: vec![],
            signal_semaphores: vec![],
            live_cmds: vec![],
        }
    }
}

impl SubmitInfoProvider for SubmitSet{
    fn info(&self) -> vk::SubmitInfo2 {
      
        vk::SubmitInfo2::builder()
        .wait_semaphore_infos(&self.wait_semaphores)
        .command_buffer_infos(&self.live_cmds)
        .signal_semaphore_infos(&self.signal_semaphores)
        .build()
    }

    fn add_cmd(&mut self, cmd: Arc<vk::CommandBuffer>) {
        let info = vk::CommandBufferSubmitInfo::builder().device_mask(0).command_buffer(*cmd);
        self.cmds.push(cmd);
        self.live_cmds.push(info.build());
    }

    fn add_wait<S:SemaphoreProvider>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2) {
        let info = semaphore_provider.submit_info(stage);
        self.wait_semaphores.push(info);
    }

    fn add_signal<S:SemaphoreProvider>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2) {
        let info = semaphore_provider.submit_info(stage);
        self.signal_semaphores.push(info);
    }


}
