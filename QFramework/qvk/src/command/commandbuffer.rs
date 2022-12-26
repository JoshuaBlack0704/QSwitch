use std::sync::Arc;

use ash::vk;
use log::debug;

use crate::{init::DeviceSource, memory::buffer::BufferSupplier};

use super::{CommandBuffer, CommandBufferSource, BindPipelineFactory, BindSetFactory, CommandOpError};

 
impl<D:DeviceSource + Clone> CommandBuffer<D>{
    pub fn new(device_store: &D, cmd: vk::CommandBuffer) -> Arc<CommandBuffer<D>> {
        Arc::new(
            Self{
                device: device_store.clone(),
                cmd,
            }
        )
    }
}

impl<D:DeviceSource> CommandBufferSource for Arc<CommandBuffer<D>>{
    fn cmd(&self) -> vk::CommandBuffer {
        self.cmd
    }

    fn begin(&self, info: Option<vk::CommandBufferBeginInfo>) -> Result<(), vk::Result> {
        unsafe{
            let mut begin = vk::CommandBufferBeginInfo::default();
            if let Some(i) = info{
                begin = i;
            }
            self.device.device().begin_command_buffer(self.cmd, &begin)
        }
    }

    fn end(&self) -> Result<(), vk::Result> {
        unsafe{
            self.device.device().end_command_buffer(self.cmd)
        }
    }

    fn barrier(&self, info: vk::DependencyInfo) {
        unsafe{
            self.device.device().cmd_pipeline_barrier2(self.cmd, &info);
        }
    }

    fn bind_pipeline<BP: BindPipelineFactory>(&self, pipeline: &BP) {
        unsafe{
            self.device.device().cmd_bind_pipeline(self.cmd, pipeline.bind_point(), pipeline.pipeline());
        }
    }

    fn bind_set<BP:BindPipelineFactory, BS: BindSetFactory>(&self, set: &BS, set_index: u32, pipeline: &BP) {
        unsafe{
            if let Some(o) = set.dynamic_offsets(){
                let sets = [set.set()];
                self.device.device().cmd_bind_descriptor_sets(self.cmd, pipeline.bind_point(), pipeline.layout(), set_index, &sets, &o);
            }
            else{
                let sets = [set.set()];
                self.device.device().cmd_bind_descriptor_sets(self.cmd, pipeline.bind_point(), pipeline.layout(), set_index, &sets, &[]);
            }
        }
    }


    fn buffer_copy<B1:crate::memory::buffer::BufferSource, B2:crate::memory::buffer::BufferSource, BP1: super::BufferCopyFactory + BufferSupplier<B1>, BP2: super::BufferCopyFactory + BufferSupplier<B2>>(&self, src: &BP1, dst: &BP2) -> Result<(), CommandOpError> {
        
        if src.size() > dst.size(){
            return Err(CommandOpError::MemOpNoSpace);
            
        }

        let op = [vk::BufferCopy::builder()
        .src_offset(src.offset())
        .dst_offset(dst.offset())
        .size(src.size())
        .build()];

        unsafe{
            let device = self.device.device();
            device.cmd_copy_buffer(self.cmd, *src.buffer_provider().buffer(), *dst.buffer_provider().buffer(), &op);
        }
        Ok(())
    }

    fn buffer_image_copy<B:crate::memory::buffer::BufferSource, BS: super::BufferCopyFactory + BufferSupplier<B>, I:crate::image::ImageSource, IR: super::ImageCopyFactory + crate::image::ImageSupplier<I>>(&self, src: &BS, dst: &IR, buffer_addressing: Option<(u32,u32)>) -> Result<(), CommandOpError> {
        if dst.extent().width == 0{
            return Ok(());
        }
        if dst.extent().height== 0{
            return Ok(());
        }
        if dst.extent().depth == 0{
            return Ok(());
        }
        
        let buffer_offset = src.offset();
        let mut addressing = (0,0);
        if let Some(a) = buffer_addressing{
            addressing = a;
        }

        let subresource = dst.subresource();
        let offset = dst.offset();
        let extent = dst.extent();
        let image = dst.image_provider().image();
        let layout = dst.layout();
        
        let info = [vk::BufferImageCopy::builder()
        .buffer_offset(buffer_offset)
        .buffer_row_length(addressing.0)
        .buffer_image_height(addressing.1)
        .image_subresource(subresource)
        .image_offset(offset)
        .image_extent(extent)
        .build()];

        unsafe{
            let device = self.device.device();
            debug!("Copying {:?} bytes from buffer {:?} to layer {:?} of image {:?}", src.size(), *src.buffer_provider().buffer(), dst.subresource(), *image);
            device.cmd_copy_buffer_to_image(self.cmd, *src.buffer_provider().buffer(), *image, *layout, &info);
        }

        Ok(())
    }

    fn image_copy<I1: crate::image::ImageSource, I2: crate::image::ImageSource, IR1: super::ImageCopyFactory + crate::image::ImageSupplier<I1>, IR2: super::ImageCopyFactory + crate::image::ImageSupplier<I2>>(&self, src: &IR1, dst: &IR2) -> Result<(), CommandOpError> {
        if src.extent().width == 0{
            return Ok(());
        }
        if src.extent().height== 0{
            return Ok(());
        }
        if src.extent().depth == 0{
            return Ok(());
        }
        if dst.extent().width == 0{
            return Ok(());
        }
        if dst.extent().height== 0{
            return Ok(());
        }
        if dst.extent().depth == 0{
            return Ok(());
        }

        
        let src_layout = src.layout();
        let dst_layout = dst.layout();
        let op = [vk::ImageCopy::builder()
        .src_subresource(src.subresource())
        .dst_subresource(dst.subresource())
        .src_offset(src.offset())
        .dst_offset(dst.offset())
        .extent(src.extent())
        .build()];

        let src_image = src.image_provider().image();
        let dst_image = dst.image_provider().image();
        debug!("Copying layer {:?} for image {:?} to image {:?}", src.subresource(), *src_image, *dst_image);

        unsafe{
            let device = self.device.device();
            device.cmd_copy_image(self.cmd, *src_image, *src_layout, *dst_image, *dst_layout, &op);
        }
        Ok(())
    }

    fn image_blit<I1: crate::image::ImageSource, I2: crate::image::ImageSource, IR1: super::ImageCopyFactory + crate::image::ImageSupplier<I1>, IR2: super::ImageCopyFactory + crate::image::ImageSupplier<I2>>(&self, src: &IR1, dst: &IR2, scale_filter: vk::Filter) -> Result<(), CommandOpError> {
        if src.extent().width == 0{
            return Ok(());
        }
        if src.extent().height== 0{
            return Ok(());
        }
        if src.extent().depth == 0{
            return Ok(());
        }
        if dst.extent().width == 0{
            return Ok(());
        }
        if dst.extent().height== 0{
            return Ok(());
        }
        if dst.extent().depth == 0{
            return Ok(());
        }

        let src_layout = src.layout();
        let dst_layout = dst.layout();

        let src_image = src.image_provider().image();
        let dst_image = dst.image_provider().image();

        let src_lower = src.offset();
        let src_upper = vk::Offset3D::builder().x(src_lower.x + src.extent().width as i32).y(src_lower.y + src.extent().height as i32).z(src_lower.z + src.extent().depth as i32).build();
        let dst_lower = src.offset();
        let dst_upper = vk::Offset3D::builder().x(dst_lower.x + dst.extent().width as i32).y(dst_lower.y + dst.extent().height as i32).z(dst_lower.z + dst.extent().depth as i32).build();

        let blit = [vk::ImageBlit::builder()
        .src_subresource(src.subresource())
        .dst_subresource(dst.subresource())
        .src_offsets([src_lower, src_upper])
        .dst_offsets([dst_lower, dst_upper])
        .build()];

        unsafe{
            let device = self.device.device();
            device.cmd_blit_image(self.cmd, *src_image, *src_layout, *dst_image, *dst_layout, &blit, scale_filter);
        }

        Ok(())
    }

    fn image_buffer_copy<B:crate::memory::buffer::BufferSource, BS: super::BufferCopyFactory + BufferSupplier<B>, I:crate::image::ImageSource, IR: super::ImageCopyFactory + crate::image::ImageSupplier<I>>(&self, src: &IR, dst: &BS, buffer_addressing: Option<(u32,u32)>) -> Result<(), CommandOpError> {
        let buffer_offset = dst.offset();
        let mut addressing = (0,0);
        if let Some(a) = buffer_addressing{
            addressing = a;
        }

        let subresource = src.subresource();
        let offset = src.offset();
        let extent = src.extent();
        let image = src.image_provider().image();
        let layout = src.layout();
        
        let info = [vk::BufferImageCopy::builder()
        .buffer_offset(buffer_offset)
        .buffer_row_length(addressing.0)
        .buffer_image_height(addressing.1)
        .image_subresource(subresource)
        .image_offset(offset)
        .image_extent(extent)
        .build()];

        unsafe{
            let device = self.device.device();
            let buffer = dst.buffer_provider().buffer();
            debug!("Copying image layer {:?} from image {:?} to buffer {:?}", src.subresource(), *image, *buffer);
            device.cmd_copy_image_to_buffer(self.cmd, *image, *layout, *buffer, &info);
        }
        Ok(())
    }

    fn dispatch(&self, x: u32, y: u32, z:u32) {
        unsafe{
            self.device.device().cmd_dispatch(self.cmd, x, y, z);
        }
    }

    fn transition_img<Img:super::ImageTransitionFactory>(&self, factory:&Img, new_layout: vk::ImageLayout, src_stage: vk::PipelineStageFlags2, src_access: vk::AccessFlags2, dst_stage: vk::PipelineStageFlags2, dst_access: vk::AccessFlags2) {
        let mutex = factory.old_layout();
        let mut old_layout = mutex.lock().unwrap();
        let range = factory.range();
        let info = vk::ImageMemoryBarrier2::builder()
            .src_stage_mask(src_stage)
            .src_access_mask(src_access)
            .dst_stage_mask(dst_stage)
            .dst_access_mask(dst_access)
            .subresource_range(range)
            .old_layout(*old_layout)
            .new_layout(new_layout)
            .image(factory.image());

        let info = [info.build()];
        let dependency = vk::DependencyInfo::builder()
        .image_memory_barriers(&info);

        unsafe{
            self.device.device().cmd_pipeline_barrier2(self.cmd(), &dependency);
        }

        *old_layout = new_layout;
    }


}

