use std::sync::Arc;

use ash::vk;

use crate::init::{DeviceSource, InstanceSource};

use super::{
    BindPipelineFactory, BindSetFactory, BufferCopyFactory, CommandBuffer, CommandBufferSource,
    CommandOpError, ImageCopyFactory,
};

impl<D: DeviceSource + Clone> CommandBuffer<D> {
    pub fn new(device_store: &D, cmd: vk::CommandBuffer) -> Arc<CommandBuffer<D>> {
        Arc::new(Self {
            device: device_store.clone(),
            cmd,
        })
    }
}

impl<D: DeviceSource> CommandBufferSource for Arc<CommandBuffer<D>> {
    fn cmd(&self) -> vk::CommandBuffer {
        self.cmd
    }

    fn begin(&self, info: Option<vk::CommandBufferBeginInfo>) -> Result<(), vk::Result> {
        unsafe {
            let mut begin = vk::CommandBufferBeginInfo::default();
            if let Some(i) = info {
                begin = i;
            }
            self.device.device().begin_command_buffer(self.cmd, &begin)
        }
    }

    fn end(&self) -> Result<(), vk::Result> {
        unsafe { self.device.device().end_command_buffer(self.cmd) }
    }

    fn barrier(&self, info: vk::DependencyInfo) {
        unsafe {
            self.device.device().cmd_pipeline_barrier2(self.cmd, &info);
        }
    }

    fn bind_pipeline<BP: BindPipelineFactory>(&self, pipeline: &BP) {
        unsafe {
            self.device.device().cmd_bind_pipeline(
                self.cmd,
                pipeline.bind_point(),
                pipeline.pipeline(),
            );
        }
    }

    fn bind_set<BP: BindPipelineFactory, BS: BindSetFactory>(
        &self,
        set: &BS,
        set_index: u32,
        pipeline: &BP,
    ) {
        unsafe {
            if let Some(o) = set.dynamic_offsets() {
                let sets = [set.set()];
                self.device.device().cmd_bind_descriptor_sets(
                    self.cmd,
                    pipeline.bind_point(),
                    pipeline.layout(),
                    set_index,
                    &sets,
                    &o,
                );
            } else {
                let sets = [set.set()];
                self.device.device().cmd_bind_descriptor_sets(
                    self.cmd,
                    pipeline.bind_point(),
                    pipeline.layout(),
                    set_index,
                    &sets,
                    &[],
                );
            }
        }
    }

    fn buffer_copy<BP1: BufferCopyFactory, BP2: BufferCopyFactory>(
        &self,
        src: &BP1,
        dst: &BP2,
    ) -> Result<(), CommandOpError> {
        if src.size() > dst.size() {
            return Err(CommandOpError::MemOpNoSpace);
        }

        let op = [vk::BufferCopy::builder()
            .src_offset(src.offset())
            .dst_offset(dst.offset())
            .size(src.size())
            .build()];

        unsafe {
            let device = self.device.device();
            device.cmd_copy_buffer(self.cmd, src.buffer(), dst.buffer(), &op);
        }
        Ok(())
    }

    fn buffer_image_copy<BS: BufferCopyFactory, IR: ImageCopyFactory>(
        &self,
        src: &BS,
        dst: &IR,
        buffer_addressing: Option<(u32, u32)>,
    ) -> Result<(), CommandOpError> {
        if dst.extent().width == 0 {
            return Ok(());
        }
        if dst.extent().height == 0 {
            return Ok(());
        }
        if dst.extent().depth == 0 {
            return Ok(());
        }

        let buffer_offset = src.offset();
        let mut addressing = (0, 0);
        if let Some(a) = buffer_addressing {
            addressing = a;
        }

        let subresource = dst.subresource();
        let offset = dst.offset();
        let extent = dst.extent();
        let image = dst.image();
        let layout = dst.layout();

        let info = [vk::BufferImageCopy::builder()
            .buffer_offset(buffer_offset)
            .buffer_row_length(addressing.0)
            .buffer_image_height(addressing.1)
            .image_subresource(subresource)
            .image_offset(offset)
            .image_extent(extent)
            .build()];

        unsafe {
            let device = self.device.device();
            // debug!(
            //     "Copying {:?} bytes from buffer {:?} to layer {:?} of image {:?}",
            //     src.size(),
            //     src.buffer(),
            //     dst.subresource(),
            //     image
            // );
            device.cmd_copy_buffer_to_image(self.cmd, src.buffer(), image, *layout, &info);
        }

        Ok(())
    }

    fn image_copy<IR1: ImageCopyFactory, IR2: ImageCopyFactory>(
        &self,
        src: &IR1,
        dst: &IR2,
    ) -> Result<(), CommandOpError> {
        if src.extent().width == 0 {
            return Ok(());
        }
        if src.extent().height == 0 {
            return Ok(());
        }
        if src.extent().depth == 0 {
            return Ok(());
        }
        if dst.extent().width == 0 {
            return Ok(());
        }
        if dst.extent().height == 0 {
            return Ok(());
        }
        if dst.extent().depth == 0 {
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

        let src_image = src.image();
        let dst_image = dst.image();
        // debug!(
        //     "Copying layer {:?} for image {:?} to image {:?}",
        //     src.subresource(),
        //     src_image,
        //     dst_image
        // );

        unsafe {
            let device = self.device.device();
            device.cmd_copy_image(
                self.cmd,
                src_image,
                *src_layout,
                dst_image,
                *dst_layout,
                &op,
            );
        }
        Ok(())
    }

    fn image_blit<IR1: ImageCopyFactory, IR2: ImageCopyFactory>(
        &self,
        src: &IR1,
        dst: &IR2,
        scale_filter: vk::Filter,
    ) -> Result<(), CommandOpError> {
        if src.extent().width == 0 {
            return Ok(());
        }
        if src.extent().height == 0 {
            return Ok(());
        }
        if src.extent().depth == 0 {
            return Ok(());
        }
        if dst.extent().width == 0 {
            return Ok(());
        }
        if dst.extent().height == 0 {
            return Ok(());
        }
        if dst.extent().depth == 0 {
            return Ok(());
        }

        let src_layout = src.layout();
        let dst_layout = dst.layout();

        let src_image = src.image();
        let dst_image = dst.image();

        let src_lower = src.offset();
        let src_upper = vk::Offset3D::builder()
            .x(src_lower.x + src.extent().width as i32)
            .y(src_lower.y + src.extent().height as i32)
            .z(src_lower.z + src.extent().depth as i32)
            .build();
        let dst_lower = src.offset();
        let dst_upper = vk::Offset3D::builder()
            .x(dst_lower.x + dst.extent().width as i32)
            .y(dst_lower.y + dst.extent().height as i32)
            .z(dst_lower.z + dst.extent().depth as i32)
            .build();

        let blit = [vk::ImageBlit::builder()
            .src_subresource(src.subresource())
            .dst_subresource(dst.subresource())
            .src_offsets([src_lower, src_upper])
            .dst_offsets([dst_lower, dst_upper])
            .build()];

        unsafe {
            let device = self.device.device();
            device.cmd_blit_image(
                self.cmd,
                src_image,
                *src_layout,
                dst_image,
                *dst_layout,
                &blit,
                scale_filter,
            );
        }

        Ok(())
    }

    fn image_buffer_copy<BS: BufferCopyFactory, IR: ImageCopyFactory>(
        &self,
        src: &IR,
        dst: &BS,
        buffer_addressing: Option<(u32, u32)>,
    ) -> Result<(), CommandOpError> {
        let buffer_offset = dst.offset();
        let mut addressing = (0, 0);
        if let Some(a) = buffer_addressing {
            addressing = a;
        }

        let subresource = src.subresource();
        let offset = src.offset();
        let extent = src.extent();
        let image = src.image();
        let layout = src.layout();

        let info = [vk::BufferImageCopy::builder()
            .buffer_offset(buffer_offset)
            .buffer_row_length(addressing.0)
            .buffer_image_height(addressing.1)
            .image_subresource(subresource)
            .image_offset(offset)
            .image_extent(extent)
            .build()];

        unsafe {
            let device = self.device.device();
            let buffer = dst.buffer();
            // debug!(
            //     "Copying image layer {:?} from image {:?} to buffer {:?}",
            //     src.subresource(),
            //     image,
            //     buffer
            // );
            device.cmd_copy_image_to_buffer(self.cmd, image, *layout, buffer, &info);
        }
        Ok(())
    }

    fn dispatch(&self, x: u32, y: u32, z: u32) {
        unsafe {
            self.device.device().cmd_dispatch(self.cmd, x, y, z);
        }
    }

    fn transition_img<Img: super::ImageTransitionFactory>(
        &self,
        factory: &Img,
        new_layout: vk::ImageLayout,
        src_stage: vk::PipelineStageFlags2,
        src_access: vk::AccessFlags2,
        dst_stage: vk::PipelineStageFlags2,
        dst_access: vk::AccessFlags2,
    ) {
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
        let dependency = vk::DependencyInfo::builder().image_memory_barriers(&info);

        unsafe {
            self.device
                .device()
                .cmd_pipeline_barrier2(self.cmd(), &dependency);
        }

        *old_layout = new_layout;
    }

    fn bind_vertex_buffer(&self, factory: &impl super::BindVertexBufferFactory) {
        let buffers = [factory.buffer()];
        let offset = [factory.offset()];
        unsafe {
            self.device()
                .cmd_bind_vertex_buffers(self.cmd(), 0, &buffers, &offset);
        }
    }

    fn bind_index_buffer(&self, factory: &impl super::BindIndexBufferFactory) {
        unsafe {
            self.device().cmd_bind_index_buffer(
                self.cmd(),
                factory.buffer(),
                factory.offset(),
                factory.index_type(),
            );
        }
    }

    fn begin_render_pass(&self, factory: &impl super::BeginRenderPassFactory) {
        let info = vk::RenderPassBeginInfo::builder()
            .render_pass(factory.renderpass())
            .framebuffer(factory.framebuffer())
            .render_area(factory.render_area())
            .clear_values(factory.clear_values());
        unsafe {
            self.device()
                .cmd_begin_render_pass(self.cmd(), &info, factory.subpass_contents());
        }
    }

    fn draw_indexed(&self) {
        todo!()
    }

    fn end_render_pass(&self) {
        unsafe {
            self.device().cmd_end_render_pass(self.cmd());
        }
    }

    fn mem_barrier(
        &self,
        src_stage: vk::PipelineStageFlags2,
        src_access: vk::AccessFlags2,
        dst_stage: vk::PipelineStageFlags2,
        dst_access: vk::AccessFlags2,
    ) {
        let barrier = [vk::MemoryBarrier2::builder()
            .src_stage_mask(src_stage)
            .src_access_mask(src_access)
            .dst_stage_mask(dst_stage)
            .dst_access_mask(dst_access)
            .build()];

        let info = vk::DependencyInfo::builder().memory_barriers(&barrier);

        unsafe {
            self.device
                .device()
                .cmd_pipeline_barrier2(self.cmd(), &info);
        }
    }
}

impl<D: DeviceSource + InstanceSource> InstanceSource for Arc<CommandBuffer<D>> {
    fn instance(&self) -> &ash::Instance {
        self.device.instance()
    }

    fn entry(&self) -> &ash::Entry {
        self.device.entry()
    }
}

impl<D: DeviceSource> DeviceSource for Arc<CommandBuffer<D>> {
    fn device(&self) -> &ash::Device {
        self.device.device()
    }

    fn surface(&self) -> &Option<vk::SurfaceKHR> {
        self.device.surface()
    }

    fn physical_device(&self) -> &crate::init::PhysicalDeviceData {
        self.device.physical_device()
    }

    fn get_queue(&self, target_flags: vk::QueueFlags) -> Option<(vk::Queue, u32)> {
        self.device.get_queue(target_flags)
    }

    fn grahics_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.grahics_queue()
    }

    fn compute_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.compute_queue()
    }

    fn transfer_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.transfer_queue()
    }

    fn present_queue(&self) -> Option<(vk::Queue, u32)> {
        self.device.present_queue()
    }

    fn memory_type(&self, properties: vk::MemoryPropertyFlags) -> u32 {
        self.device.memory_type(properties)
    }

    fn device_memory_index(&self) -> u32 {
        self.device.device_memory_index()
    }

    fn host_memory_index(&self) -> u32 {
        self.device.host_memory_index()
    }
}
