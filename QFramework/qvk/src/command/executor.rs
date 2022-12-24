use std::sync::Arc;

use ash::vk;

use crate::{init::DeviceSource, queue::{Queue, QueueOps, SubmitSet, SubmitInfoStore}};

use super::{Executor, commandpool, CommandPool, commandset, CommandSet, CommandBufferFactory, CommandBuffer, CommandBufferStore, CommandPoolOps};

impl<D:DeviceSource + Clone> Executor<D>{
    pub fn new(device_provider: &D, queue_flags: vk::QueueFlags) -> Arc<Executor<D>> {
        let index = device_provider.get_queue(queue_flags).unwrap().1;
        let settings = commandpool::SettingsStore::new(index);
        let pool = CommandPool::new(&settings, device_provider).unwrap();
        let mut settings = commandset::SettingsStore::default();
        settings.batch_size = 1;
        let bset = CommandSet::new(&settings, device_provider, &pool);

        let queue = Queue::new(device_provider, queue_flags).unwrap();

        Arc::new(
            Self{
                _device: device_provider.clone(),
                command_pool: pool,
                command_set: bset,
                queue,
            }
        )
    }
    pub fn wait_submit_internal(&self){
        let cmds = self.command_set.created_cmds();
        let mut submit = SubmitSet::new(&cmds[0]);
        if cmds.len() > 1{
            for cmd in cmds[1..].iter(){
                submit.add_cmd(cmd);
            }
        }
        let submit = [submit];
        self.queue.wait_submit(&submit).unwrap();
    }
}

impl<D:DeviceSource> CommandPoolOps for Executor<D>{
    fn reset_cmdpool(&self) {
        self.command_pool.reset_cmdpool();
    }
}

impl<D:DeviceSource + Clone> QueueOps for Executor<D>{
    fn submit<C:CommandBufferStore + Clone, S:crate::queue::SubmitInfoStore<C>, F:crate::sync::FenceSource>(&self, submits: &[S], fence: Option<&F>) -> Result<(), vk::Result> {
        self.queue.submit(submits, fence)
    }

    fn wait_submit<C:CommandBufferStore + Clone, S:crate::queue::SubmitInfoStore<C>>(&self, submits: &[S]) -> Result<(), vk::Result> {
        self.queue.wait_submit(submits)
    }

    fn wait_idle(&self) {
        self.queue.wait_idle();
    }
}

impl<D:DeviceSource + Clone> CommandBufferFactory<D,Arc<CommandBuffer<D>>> for Executor<D>{
    fn next_cmd(&self) -> Arc<CommandBuffer<D>> {
        self.command_set.next_cmd()
    }

    fn reset_cmd(&self, cmd: &Arc<CommandBuffer<D>>) {
        self.command_set.reset_cmd(cmd)
    }
}