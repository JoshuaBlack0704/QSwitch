use std::sync::{Arc, Mutex, MutexGuard};

use ash::vk;

use crate::{init::DeviceSource, queue::Queue};

pub mod commandpool;
pub trait CommandPoolFactory<C:CommandPoolSource>{
    fn reset_flags(&self) -> Option<vk::CommandPoolResetFlags> {None}
    fn create_command_pool(&self, queue_family_index: u32, create_flags: Option<vk::CommandPoolCreateFlags>) -> Result<C, vk::Result>;
}
pub trait CommandPoolSource{
    fn cmdpool(&self) -> &vk::CommandPool;
}

pub trait CommandPoolOps{
    fn reset_cmdpool(&self);
}
pub struct CommandPool<D: DeviceSource, C:CommandBufferSource>{
    device: D,
    reset_flags: Option<vk::CommandPoolResetFlags>,
    command_pool: vk::CommandPool,
    cmds: Mutex<Vec<C>>,
}

pub mod commandbuffer;
pub trait CommandBufferFactory<C:CommandBufferSource>{
    fn next_cmd(&self, level: vk::CommandBufferLevel) -> C;
    fn reset_cmd(&self, cmd: &C, reset_flags: Option<vk::CommandBufferResetFlags>);
    fn created_cmds(&self) -> Vec<C>;
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
    fn buffer(&self) -> vk::Buffer;
}
pub trait ImageCopyFactory{
    fn extent(&self) -> vk::Extent3D;
    fn subresource(&self) -> vk::ImageSubresourceLayers;
    fn offset(&self) -> vk::Offset3D;
    fn layout(&self) -> MutexGuard<vk::ImageLayout>;
    fn image(&self) -> vk::Image;
}
pub trait ImageTransitionFactory{
    fn image(&self) -> vk::Image;
    fn range(&self) -> vk::ImageSubresourceRange;
    fn old_layout(&self) -> Arc<Mutex<vk::ImageLayout>>;
}
pub trait CommandBufferSource{
    fn cmd(&self) -> vk::CommandBuffer;
    fn begin(&self, info: Option<vk::CommandBufferBeginInfo>) -> Result<(), vk::Result>;
    fn end(&self) -> Result<(), vk::Result>;
    fn barrier(&self, info: vk::DependencyInfo);
    fn bind_pipeline<BP: BindPipelineFactory>(&self, pipeline: &BP);
    fn bind_set<BP:BindPipelineFactory, BS: BindSetFactory>(&self, set: &BS, set_index: u32, pipeline: &BP);
    fn buffer_copy<BP1: BufferCopyFactory, BP2: BufferCopyFactory>(&self, src: &BP1, dst: &BP2) -> Result<(), CommandOpError>;
    fn buffer_image_copy<BS: BufferCopyFactory, IR: ImageCopyFactory>(&self, src: &BS, dst: &IR, buffer_addressing: Option<(u32,u32)>) -> Result<(), CommandOpError>;
    fn image_copy<IR1: ImageCopyFactory, IR2: ImageCopyFactory>(&self, src: &IR1, dst: &IR2) -> Result<(), CommandOpError>;
    fn image_blit<IR1: ImageCopyFactory, IR2: ImageCopyFactory>(&self, src: &IR1, dst: &IR2, scale_filter: vk::Filter) -> Result<(), CommandOpError>;
    fn image_buffer_copy<BS: BufferCopyFactory, IR: ImageCopyFactory>(&self, src: &IR, dst: &BS, buffer_addressing: Option<(u32,u32)>) -> Result<(), CommandOpError>;
    fn dispatch(&self, x: u32, y: u32, z:u32);
    fn transition_img<Img:ImageTransitionFactory>(&self, factory:&Img, new_layout: vk::ImageLayout, src_stage: vk::PipelineStageFlags2, src_access: vk::AccessFlags2, dst_stage: vk::PipelineStageFlags2, dst_access: vk::AccessFlags2);
}
#[derive(Debug)]
pub enum CommandOpError{
    MemOpNoSpace,
    Vulkan(vk::Result)
}
pub struct CommandBuffer<D:DeviceSource>{
    device: D,
    cmd: vk::CommandBuffer,
}

pub mod executor;
pub struct Executor<D:DeviceSource>{
    _device: D,
    command_pool: Arc<CommandPool<D, Arc<CommandBuffer<D>>>>,
    queue: Arc<Queue<D>>,
    
}




