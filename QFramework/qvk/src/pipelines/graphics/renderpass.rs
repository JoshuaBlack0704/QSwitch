use std::sync::{Arc, Mutex, MutexGuard};

use ash::vk;
use log::{debug, info};

use crate::{image::ImageViewSource, init::DeviceSource};

use super::{
    RenderPassAttachment, RenderPassSource, Renderpass, RenderpassAttachmentSource,
    RenderpassFactory, SubpassDescription, SubpassDescriptionSource,
};

impl<D: DeviceSource + Clone, A: RenderpassAttachmentSource + Clone>
    RenderpassFactory<Arc<Renderpass<D, A>>, A> for D
{
    fn create_renderpass<S: SubpassDescriptionSource>(
        &self,
        attachments: &[&A],
        subpasses: &[&S],
        flags: Option<vk::RenderPassCreateFlags>,
    ) -> Arc<Renderpass<D, A>> {
        let mut attachments_desc = vec![];
        for attachment in attachments.iter() {
            let mut info = vk::AttachmentDescription::builder();
            if let Some(flags) = attachment.flags() {
                info = info.flags(flags);
            }
            info = info
                .format(attachment.format())
                .samples(attachment.samples())
                .load_op(attachment.load_op())
                .store_op(attachment.store_op());
            if let Some(op) = attachment.stencil_load() {
                info = info.stencil_load_op(op);
            } else {
                info = info.stencil_load_op(vk::AttachmentLoadOp::DONT_CARE);
            }
            if let Some(op) = attachment.stencil_store() {
                info = info.stencil_store_op(op);
            } else {
                info = info.stencil_store_op(vk::AttachmentStoreOp::DONT_CARE);
            }
            info = info
                .initial_layout(attachment.inital_layout())
                .final_layout(attachment.final_layout());
            *attachment.index() = attachments_desc.len() as u32;
            attachments_desc.push(info.build());
        }

        let mut subpass_desc = vec![];
        let mut subpass_dep = vec![];
        for subpass in subpasses.iter() {
            let input_attachments = subpass.input_attachments();
            let color_attachments = subpass.color_attachments();
            let resolve_attachments = subpass.resolve_attachments();
            let depth_attachment = subpass.depth_stencil_attachment();
            let preserve_attachment = subpass.preserve_attachments();

            let mut info =
                vk::SubpassDescription::builder().pipeline_bind_point(subpass.bind_point());
            if let Some(flags) = subpass.flags() {
                info = info.flags(flags);
            }
            if let Some(a) = &input_attachments {
                info = info.input_attachments(a);
            }
            if let Some(a) = &color_attachments {
                info = info.color_attachments(a);
            }
            if let Some(a) = &resolve_attachments {
                info = info.resolve_attachments(a);
            }
            if let Some(a) = &depth_attachment {
                info = info.depth_stencil_attachment(a);
            }
            if let Some(a) = &preserve_attachment {
                info = info.preserve_attachments(a);
            }
            *subpass.index() = subpass_desc.len() as u32;
            subpass_desc.push(info.build());
            if let Some(deps) = subpass.dependencies() {
                subpass_dep.extend_from_slice(deps);
            }
        }

        let mut info = vk::RenderPassCreateInfo::builder();
        if let Some(flags) = flags {
            info = info.flags(flags);
        }
        info = info
            .attachments(&attachments_desc)
            .subpasses(&subpass_desc)
            .dependencies(&subpass_dep);

        let renderpass;
        unsafe {
            renderpass = self.device().create_render_pass(&info, None).unwrap();
        }
        info!("Created renderpass {:?}", renderpass);
        Arc::new(Renderpass {
            _device: self.clone(),
            _renderpass: renderpass,
            _attachments: attachments_desc,
            _subpass_refs: subpass_desc,
            _image_views: attachments.iter().map(|a| (*a).clone()).collect(),
        })
    }
}

impl<D: DeviceSource, A: RenderpassAttachmentSource> RenderPassSource for Arc<Renderpass<D, A>> {
    fn renderpass(&self) -> ash::vk::RenderPass {
        self._renderpass
    }
}

impl<D: DeviceSource, A: RenderpassAttachmentSource> Drop for Renderpass<D, A> {
    fn drop(&mut self) {
        debug!("Destroyed renderpass {:?}", self._renderpass);
        unsafe {
            self._device
                .device()
                .destroy_render_pass(self._renderpass, None);
        }
    }
}

impl<IV: ImageViewSource + Clone> RenderPassAttachment<IV> {
    pub fn new(
        view: &IV,
        initial_layout: vk::ImageLayout,
        subpass_layout: vk::ImageLayout,
        final_layout: vk::ImageLayout,
        load_op: vk::AttachmentLoadOp,
        store_op: vk::AttachmentStoreOp,
    ) -> Arc<RenderPassAttachment<IV>> {
        Arc::new(RenderPassAttachment {
            index: Mutex::new(0),
            view: Mutex::new(view.clone()),
            initial_layout,
            subpass_layout,
            final_layout,
            load_op,
            store_op,
        })
    }
}
impl<IV: ImageViewSource + Clone> RenderpassAttachmentSource for Arc<RenderPassAttachment<IV>> {
    fn flags(&self) -> Option<vk::AttachmentDescriptionFlags> {
        None
    }

    fn inital_layout(&self) -> vk::ImageLayout {
        self.initial_layout
    }

    fn final_layout(&self) -> vk::ImageLayout {
        self.final_layout
    }

    fn subpass_layout(&self) -> vk::ImageLayout {
        self.subpass_layout
    }

    fn index(&self) -> std::sync::MutexGuard<u32> {
        self.index.lock().unwrap()
    }

    fn format(&self) -> vk::Format {
        self.view.lock().unwrap().format()
    }

    fn samples(&self) -> vk::SampleCountFlags {
        vk::SampleCountFlags::TYPE_1
    }

    fn load_op(&self) -> vk::AttachmentLoadOp {
        self.load_op
    }

    fn store_op(&self) -> vk::AttachmentStoreOp {
        self.store_op
    }

    fn stencil_load(&self) -> Option<vk::AttachmentLoadOp> {
        None
    }

    fn stencil_store(&self) -> Option<vk::AttachmentStoreOp> {
        None
    }

    fn view(&self) -> vk::ImageView {
        self.view.lock().unwrap().view()
    }
}

impl<A: RenderpassAttachmentSource + Clone> SubpassDescription<A> {
    pub fn new(
        bind_point: vk::PipelineBindPoint,
        _attachment_type: &A,
        flags: Option<vk::SubpassDescriptionFlags>,
    ) -> SubpassDescription<A> {
        SubpassDescription {
            index: Mutex::new(0),
            bind_point,
            flags,
            input_attachments: vec![],
            input_refs: Mutex::new(vec![]),
            color_attachments: vec![],
            color_refs: Mutex::new(vec![]),
            resolve_attachments: vec![],
            resolve_refs: Mutex::new(vec![]),
            depth_attachment: None,
            depth_ref: Mutex::new(vk::AttachmentReference::default()),
            preserve_attachments: vec![],
            dependencies: vec![],
        }
    }

    pub fn add_input_attachment(&mut self, attachment: &A) {
        self.input_attachments.push(attachment.clone());
    }
    pub fn add_color_attachment(&mut self, attachment: &A) {
        self.color_attachments.push(attachment.clone());
    }
    pub fn add_resolve_attachment(&mut self, attachment: &A) {
        self.resolve_attachments.push(attachment.clone());
    }
    pub fn add_depth_stencil_attachment(&mut self, attachment: &A) {
        self.depth_attachment = Some(attachment.clone());
    }
    pub fn add_preserve_attachment(&mut self, index: u32) {
        self.preserve_attachments.push(index);
    }
    pub fn add_dependency(
        &mut self,
        src_subpass: Option<u32>,
        src_stage: vk::PipelineStageFlags,
        src_access: vk::AccessFlags,
        dst_stage: vk::PipelineStageFlags,
        dst_access: vk::AccessFlags,
        flags: Option<vk::DependencyFlags>,
    ) {
        let mut info = vk::SubpassDependency::builder();
        if let Some(index) = src_subpass {
            info = info.src_subpass(index);
        } else {
            info = info.src_subpass(vk::SUBPASS_EXTERNAL);
        }
        info = info
            .src_stage_mask(src_stage)
            .src_access_mask(src_access)
            .dst_subpass(*self.index.lock().unwrap())
            .dst_stage_mask(dst_stage)
            .dst_access_mask(dst_access);

        if let Some(flags) = flags {
            info = info.dependency_flags(flags);
        }

        self.dependencies.push(info.build());
    }
    pub fn add_start_dependency(&mut self) {
        self.add_dependency(
            None,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::AccessFlags::NONE,
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
            None,
        );
    }
    pub fn add_depth_dependency(&mut self) {
        self.add_dependency(
            None,
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            vk::AccessFlags::NONE,
            vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags::LATE_FRAGMENT_TESTS,
            vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
            None,
        );
    }
}

impl<A: RenderpassAttachmentSource> SubpassDescriptionSource for SubpassDescription<A> {
    fn index(&self) -> std::sync::MutexGuard<u32> {
        self.index.lock().unwrap()
    }

    fn flags(&self) -> Option<vk::SubpassDescriptionFlags> {
        self.flags
    }

    fn bind_point(&self) -> vk::PipelineBindPoint {
        self.bind_point
    }

    fn input_attachments(&self) -> Option<MutexGuard<'_, Vec<vk::AttachmentReference>>> {
        if self.input_attachments.len() == 0 {
            return None;
        }
        let refs: Vec<vk::AttachmentReference> = self
            .input_attachments
            .iter()
            .map(|a| {
                vk::AttachmentReference::builder()
                    .attachment(*a.index())
                    .layout(a.subpass_layout())
                    .build()
            })
            .collect();

        let mut lock = self.input_refs.lock().unwrap();
        *lock = refs;

        Some(lock)
    }

    fn color_attachments(&self) -> Option<MutexGuard<'_, Vec<vk::AttachmentReference>>> {
        if self.color_attachments.len() == 0 {
            return None;
        }
        let refs: Vec<vk::AttachmentReference> = self
            .color_attachments
            .iter()
            .map(|a| {
                vk::AttachmentReference::builder()
                    .attachment(*a.index())
                    .layout(a.subpass_layout())
                    .build()
            })
            .collect();

        let mut lock = self.color_refs.lock().unwrap();
        *lock = refs;

        Some(lock)
    }

    fn resolve_attachments(&self) -> Option<MutexGuard<'_, Vec<vk::AttachmentReference>>> {
        if self.resolve_attachments.len() == 0 {
            return None;
        }
        let refs: Vec<vk::AttachmentReference> = self
            .resolve_attachments
            .iter()
            .map(|a| {
                vk::AttachmentReference::builder()
                    .attachment(*a.index())
                    .layout(a.subpass_layout())
                    .build()
            })
            .collect();

        let mut lock = self.resolve_refs.lock().unwrap();
        *lock = refs;

        Some(lock)
    }

    fn depth_stencil_attachment(&self) -> Option<MutexGuard<'_, vk::AttachmentReference>> {
        if let Some(d) = &self.depth_attachment {
            let depth_ref = vk::AttachmentReference::builder()
                .attachment(*d.index())
                .layout(d.subpass_layout())
                .build();
            let mut lock = self.depth_ref.lock().unwrap();
            *lock = depth_ref;

            return Some(lock);
        }
        None
    }

    fn preserve_attachments(&self) -> Option<&[u32]> {
        if self.preserve_attachments.len() > 0 {
            return Some(&self.preserve_attachments);
        }
        None
    }

    fn dependencies(&self) -> Option<&[vk::SubpassDependency]> {
        if self.dependencies.len() > 0 {
            return Some(&self.dependencies);
        }
        None
    }
}
