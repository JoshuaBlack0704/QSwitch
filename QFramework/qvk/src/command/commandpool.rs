use std::sync::{Arc, Mutex};

use ash::vk;
use log::{debug, info};
use crate::command::CommandPoolSource;

use crate::init::{DeviceSource, DeviceSupplier};
use super::{CommandPool, CommandPoolOps, CommandPoolFactory, CommandBufferFactory, CommandBuffer, CommandBufferSource};


impl<D:DeviceSource + Clone> CommandBufferFactory<Arc<CommandBuffer<D>>> for Arc<CommandPool<D, Arc<CommandBuffer<D>>>>{
    fn next_cmd(&self, level: vk::CommandBufferLevel) -> Arc<CommandBuffer<D>> {
        // First we need to see if there are any cmds available
        let mut cmds = self.cmds.lock().unwrap();

        //All we do is loop through cmds and see if we have a free cmd
        for cmd in cmds.iter(){
            if Arc::strong_count(cmd) == 1{
                //If we do we return it
                return cmd.clone();
            }
        }
      
        // If not we need to make a new batch
        let mut alloc_builder = vk::CommandBufferAllocateInfo::builder();
        alloc_builder = alloc_builder.command_pool(self.command_pool);
        alloc_builder = alloc_builder.command_buffer_count(1);
        alloc_builder = alloc_builder.level(level);
        let new_cmds = unsafe{self.device.device().allocate_command_buffers(&alloc_builder).expect("Could not allocate command buffers")};
        // Now the book keeping and queueing
        for cmd in new_cmds{
            info!("Created command buffer {:?}", cmd);
            cmds.push(CommandBuffer::new(&self.device, cmd));
        }
        // Now we get a newly queued element
        cmds.last().unwrap().clone()
    }

    fn reset_cmd(&self, cmd: &Arc<CommandBuffer<D>>, reset_flags: Option<vk::CommandBufferResetFlags>) {
        if let Some(f) = reset_flags{
            unsafe{self.device.device().reset_command_buffer(cmd.cmd(), f)}.expect("Failed to reset command buffer");
        }
        else{
            unsafe{self.device.device().reset_command_buffer(cmd.cmd(), vk::CommandBufferResetFlags::empty())}.expect("Failed to reset command buffer");
        }
    }

    fn created_cmds(&self) -> Vec<Arc<CommandBuffer<D>>> {
        self.cmds.lock().unwrap().clone()
    }
}

impl<D:DeviceSource + Clone, DS:DeviceSupplier<D>> CommandPoolFactory<Arc<CommandPool<D, Arc<CommandBuffer<D>>>>> for DS{
    fn create_command_pool(&self, queue_family_index: u32, create_flags: Option<vk::CommandPoolCreateFlags>) -> Result<Arc<CommandPool<D, Arc<CommandBuffer<D>>>>, vk::Result> {
        let device_provider = self.device_provider();
        let mut cmdpool_cinfo = vk::CommandPoolCreateInfo::builder();
        cmdpool_cinfo = cmdpool_cinfo.queue_family_index(queue_family_index);
        if let Some(flags) = create_flags{
            cmdpool_cinfo = cmdpool_cinfo.flags(flags);
        }
        
        let command_pool = unsafe{device_provider.device().create_command_pool(&cmdpool_cinfo, None)};
        
        match command_pool{
            Ok(pool) => {
                info!("Created command pool {:?}", pool);
                return Ok(
                    Arc::new(
                        CommandPool{ 
                            device: device_provider.clone(), 
                            command_pool: pool,
                            reset_flags: self.reset_flags(),
                            cmds: Mutex::new(vec![]),
                        }
                    )
                    
                );
            },
            Err(res) => {
                return Err(res);
            },
        }
    }
}


impl<D:DeviceSource, C:CommandBufferSource> CommandPoolOps for Arc<CommandPool<D,C>>{
    fn reset_cmdpool(&self) {
        match self.reset_flags{
            Some(f) => {
                unsafe{self.device.device().reset_command_pool(self.command_pool, f)}.expect("Could not reset command pool");
            },
            None => {
                unsafe{self.device.device().reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty())}.expect("Could not reset command pool");
            },
        }
    }
}

impl <D: DeviceSource, C:CommandBufferSource> CommandPoolSource for Arc<CommandPool<D,C>>{
    fn cmdpool(&self) -> &vk::CommandPool {
        &self.command_pool
    }
}

impl<D: DeviceSource, C:CommandBufferSource> Drop for CommandPool<D,C>{
    fn drop(&mut self) {
        debug!("Destroyed command pool {:?}", self.command_pool);
        unsafe{
            self.device.device().destroy_command_pool(self.command_pool, None);
        }
    }
}

impl<D: DeviceSource, C:CommandBufferSource> DeviceSupplier<D> for CommandPool<D,C>{
    fn device_provider(&self) -> &D {
        &self.device
    }
}
