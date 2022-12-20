use std::sync::{Arc, Mutex, MutexGuard};

use ash::vk;

use crate::{init::DeviceStore, queue::Queue, memory::buffer::{BufferStore, InternalBufferStore}, image::{ImageStore, InternalImageStore}};

use self::{commandpool::CommandPoolSettingsStore, commandset::CommandSetSettingsStore};

pub mod commandpool;
pub trait CommandPoolStore{
    fn cmdpool(&self) -> &vk::CommandPool;
}
pub trait CommandPoolOps{
    fn reset_cmdpool(&self);
}

pub struct CommandPool<D: DeviceStore, S: CommandPoolSettingsStore>{
    device: D,
    settings: S,
    command_pool: vk::CommandPool,
}

pub mod commandset;
pub struct CommandSet<D: DeviceStore, P: CommandPoolStore, S: CommandSetSettingsStore, C:CommandBufferStore>{
    device: D,
    cmdpool: P,
    settings: S,
    cmds: Mutex<Vec<C>>,
}

pub mod commandbuffer;
pub trait CommandBufferFactory<D:DeviceStore,C:CommandBufferStore>{
    fn next_cmd(&self) -> Arc<CommandBuffer<D>>;
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
    command_pool: Arc<CommandPool<D,commandpool::SettingsStore>>,
    command_set: Arc<CommandSet<D, Arc<CommandPool<D,commandpool::SettingsStore>>, commandset::SettingsStore, Arc<CommandBuffer<D>>>>,
    queue: Arc<Queue<D>>,
    
}




