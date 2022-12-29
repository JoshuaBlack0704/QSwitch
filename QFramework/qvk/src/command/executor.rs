use std::sync::Arc;

use ash::vk;

use crate::{
    init::DeviceSource,
    queue::{QueueFactory, QueueOps, SubmitInfoSource, SubmitSet},
};

use super::{
    CommandBuffer, CommandBufferFactory, CommandBufferSource, CommandPoolFactory, CommandPoolOps,
    Executor,
};

impl<D: DeviceSource + Clone> Executor<D> {
    pub fn new(device_provider: &D, queue_flags: vk::QueueFlags) -> Arc<Executor<D>> {
        let index = device_provider.get_queue(queue_flags).unwrap().1;
        let pool = device_provider.create_command_pool(index, None).unwrap();

        let queue = device_provider.create_queue(queue_flags).unwrap();

        Arc::new(Self {
            _device: device_provider.clone(),
            command_pool: pool,
            queue,
        })
    }
    pub fn wait_submit_internal(&self) {
        let cmds = self.command_pool.created_cmds();
        let mut submit = SubmitSet::new(&cmds[0]);
        if cmds.len() > 1 {
            for cmd in cmds[1..].iter() {
                submit.add_cmd(cmd);
            }
        }
        let submit = [submit];
        self.queue.wait_submit(&submit).unwrap();
    }
}

impl<D: DeviceSource> CommandPoolOps for Executor<D> {
    fn reset_cmdpool(&self) {
        self.command_pool.reset_cmdpool();
    }
}

impl<D: DeviceSource + Clone> QueueOps for Executor<D> {
    fn submit<
        C: CommandBufferSource + Clone,
        S: crate::queue::SubmitInfoSource<C>,
        F: crate::sync::FenceSource,
    >(
        &self,
        submits: &[S],
        fence: Option<&F>,
    ) -> Result<(), vk::Result> {
        self.queue.submit(submits, fence)
    }

    fn wait_submit<C: CommandBufferSource + Clone, S: crate::queue::SubmitInfoSource<C>>(
        &self,
        submits: &[S],
    ) -> Result<(), vk::Result> {
        self.queue.wait_submit(submits)
    }

    fn wait_idle(&self) {
        self.queue.wait_idle();
    }
}

impl<D: DeviceSource + Clone> CommandBufferFactory<Arc<CommandBuffer<D>>> for Executor<D> {
    fn next_cmd(&self, level: vk::CommandBufferLevel) -> Arc<CommandBuffer<D>> {
        self.command_pool.next_cmd(level)
    }

    fn reset_cmd(&self, cmd: &Arc<CommandBuffer<D>>, flags: Option<vk::CommandBufferResetFlags>) {
        self.command_pool.reset_cmd(cmd, flags)
    }

    fn created_cmds(&self) -> Vec<Arc<CommandBuffer<D>>> {
        self.command_pool.created_cmds()
    }
}
