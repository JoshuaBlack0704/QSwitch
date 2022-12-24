use std::sync::Arc;

use ash::vk;

use crate::command::CommandBufferStore;
use crate::queue::SubmitInfoStore;
use crate::sync::SemaphoreSource;

use super::SubmitSet;


impl<C:CommandBufferStore + Clone> SubmitSet<C>{
    pub fn new(first_cmd: &C) -> Self {
        let info = vk::CommandBufferSubmitInfo::builder().device_mask(0).command_buffer(first_cmd.cmd());
        Self{
            wait_semaphores: vec![],
            cmds: vec![first_cmd.clone()],
            signal_semaphores: vec![],
            live_cmds: vec![info.build()],
        }
    }
}

impl<C:CommandBufferStore + Clone> SubmitInfoStore<C> for SubmitSet<C>{
    fn info(&self) -> vk::SubmitInfo2 {
      
        vk::SubmitInfo2::builder()
        .wait_semaphore_infos(&self.wait_semaphores)
        .command_buffer_infos(&self.live_cmds)
        .signal_semaphore_infos(&self.signal_semaphores)
        .build()
    }

    fn add_cmd(&mut self, cmd: &C) {
        let info = vk::CommandBufferSubmitInfo::builder().device_mask(0).command_buffer(cmd.cmd());
        self.cmds.push(cmd.clone());
        self.live_cmds.push(info.build());
    }

    fn add_wait<S:SemaphoreSource>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2) {
        let info = semaphore_provider.submit_info(stage);
        self.wait_semaphores.push(info);
    }

    fn add_signal<S:SemaphoreSource>(&mut self, semaphore_provider: &Arc<S>, stage: vk::PipelineStageFlags2) {
        let info = semaphore_provider.submit_info(stage);
        self.signal_semaphores.push(info);
    }


}
