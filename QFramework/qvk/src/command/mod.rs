use std::sync::{Arc, Mutex, MutexGuard};

use ash::vk;
use log::info;

use crate::{init::{DeviceStore, DeviceSupplier}, queue::Queue, memory::buffer::{BufferStore, InternalBufferStore}, image::{ImageStore, InternalImageStore}};

pub mod commandpool;
pub trait CommandPoolFactory<C:CommandPoolStore>{
    fn create_commandpool(&self, q_family_index: u32, create_flags: Option<vk::CommandPoolCreateFlags>, reset_flags: Option<vk::CommandPoolResetFlags>) -> Result<C, vk::Result>;
}
impl<D:DeviceStore + Clone> CommandPoolFactory<Arc<CommandPool<D>>> for D{
    fn create_commandpool(&self, q_family_index: u32, create_flags: Option<vk::CommandPoolCreateFlags>, reset_flags: Option<vk::CommandPoolResetFlags>) -> Result<Arc<CommandPool<D>>, vk::Result> {
        
        let mut cmdpool_cinfo = vk::CommandPoolCreateInfo::builder();
        cmdpool_cinfo = cmdpool_cinfo.queue_family_index(q_family_index);
        if let Some(flags) = create_flags{
            cmdpool_cinfo = cmdpool_cinfo.flags(flags);
        }
        
        let command_pool = unsafe{self.device().create_command_pool(&cmdpool_cinfo, None)};
        
        match command_pool{
            Ok(pool) => {
                info!("Created command pool {:?}", pool);
                return Ok(Arc::new(
                    CommandPool{ 
                        device: self.clone(), 
                        command_pool: pool,
                        _q_family_index: q_family_index,
                        reset_flags,
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
pub trait CommandPoolStore{
    fn cmdpool(&self) -> &vk::CommandPool;
}
pub trait CommandPoolOps{
    fn reset_cmdpool(&self);
}
pub struct CommandPool<D: DeviceStore>{
    device: D,
    _q_family_index: u32,
    reset_flags: Option<vk::CommandPoolResetFlags>,
    command_pool: vk::CommandPool,
}

pub mod commandset;
pub trait CommandSetFactory<Cmd:CommandBufferStore, C:CommandBufferFactory<Cmd>>{
    fn create_command_set(&self, level: vk::CommandBufferLevel, reset_flags: Option<vk::CommandBufferResetFlags>) -> C;
}
impl<D:DeviceStore + Clone, P:CommandPoolStore + Clone + DeviceSupplier<D>> CommandSetFactory<Arc<CommandBuffer<D>>, Arc<CommandSet<D,P,Arc<CommandBuffer<D>>>>> for P{
    fn create_command_set(&self, level: vk::CommandBufferLevel, reset_flags: Option<vk::CommandBufferResetFlags>) -> Arc<CommandSet<D,P,Arc<CommandBuffer<D>>>> {
        Arc::new(
            CommandSet{
                device: self.device_provider().clone(),
                cmdpool: self.clone(),
                level,
                reset_flags,
                cmds: Mutex::new(vec![]),
            }
        )
    }
}
pub struct CommandSet<D: DeviceStore, P: CommandPoolStore, C:CommandBufferStore>{
    device: D,
    cmdpool: P,
    level: vk::CommandBufferLevel,
    reset_flags: Option<vk::CommandBufferResetFlags>,
    cmds: Mutex<Vec<C>>,
}

pub mod commandbuffer;
pub trait CommandBufferFactory<C:CommandBufferStore>{
    fn next_cmd(&self) -> C;
    fn reset_cmd(&self, cmd: &C);
}
pub trait BindPipelineFactory{
    fn layout(&self) -> vk::PipelineLayout;
    fn bind_point(&self) -> vk::PipelineBindPoint;
    fn pipeline(&self) -> vk::Pipeline;
}
pub trait BindSetFactory{
    fn set(&self) -> vk::DescriptorSet;
    fn dynamic_offsets(&self) -> Option<Vec<u32>>;
}
pub trait BufferCopyFactory{
    fn size(&self) -> u64;
    fn offset(&self) -> u64;
}
pub trait ImageCopyFactory{
    fn extent(&self) -> vk::Extent3D;
    fn subresource(&self) -> vk::ImageSubresourceLayers;
    fn offset(&self) -> vk::Offset3D;
    fn layout(&self) -> MutexGuard<vk::ImageLayout>;
}
pub trait CommandBufferStore{
    fn cmd(&self) -> vk::CommandBuffer;
    fn begin(&self, info: Option<vk::CommandBufferBeginInfo>) -> Result<(), vk::Result>;
    fn end(&self) -> Result<(), vk::Result>;
    fn barrier(&self, info: vk::DependencyInfo);
    fn bind_pipeline<BP: BindPipelineFactory>(&self, pipeline: &BP);
    fn bind_set<BP:BindPipelineFactory, BS: BindSetFactory>(&self, set: &BS, set_index: u32, pipeline: &BP);
    fn buffer_copy<B1:BufferStore, B2:BufferStore, BP1: BufferCopyFactory + InternalBufferStore<B1>, BP2: BufferCopyFactory + InternalBufferStore<B2>>(&self, src: &BP1, dst: &BP2) -> Result<(), CommandOpError>;
    fn buffer_image_copy<B:BufferStore, BS: BufferCopyFactory + InternalBufferStore<B>, I:ImageStore, IR: ImageCopyFactory + InternalImageStore<I>>(&self, src: &BS, dst: &IR, buffer_addressing: Option<(u32,u32)>) -> Result<(), CommandOpError>;
    fn image_copy<I1: ImageStore, I2: ImageStore, IR1: ImageCopyFactory + InternalImageStore<I1>, IR2: ImageCopyFactory + InternalImageStore<I2>>(&self, src: &IR1, dst: &IR2) -> Result<(), CommandOpError>;
    fn image_blit<I1: ImageStore, I2: ImageStore, IR1: ImageCopyFactory + InternalImageStore<I1>, IR2: ImageCopyFactory + InternalImageStore<I2>>(&self, src: &IR1, dst: &IR2, scale_filter: vk::Filter) -> Result<(), CommandOpError>;
    fn image_buffer_copy<B:BufferStore, BS: BufferCopyFactory + InternalBufferStore<B>, I:ImageStore, IR: ImageCopyFactory + InternalImageStore<I>>(&self, src: &IR, dst: &BS, buffer_addressing: Option<(u32,u32)>) -> Result<(), CommandOpError>;
    fn dispatch(&self, x: u32, y: u32, z:u32);
}
#[derive(Debug)]
pub enum CommandOpError{
    MemOpNoSpace,
    Vulkan(vk::Result)
}
pub struct CommandBuffer<D:DeviceStore>{
    device: D,
    cmd: vk::CommandBuffer,
}

pub mod executor;
pub struct Executor<D:DeviceStore>{
    _device: D,
    command_pool: Arc<CommandPool<D>>,
    command_set: Arc<CommandSet<D, Arc<CommandPool<D>>,  Arc<CommandBuffer<D>>>>,
    queue: Arc<Queue<D>>,
    
}




