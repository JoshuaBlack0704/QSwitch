use std::sync::{Arc, Mutex};

use ash::vk;
use log::info;

use crate::{init::DeviceSource, image::ImageViewSource};

use super::{RenderpassFactory, Renderpass, RenderPassSource, RenderpassAttachmentSource, SubpassDescriptionSource, RenderPassAttachment, SubpassDescription};

impl<D:DeviceSource + Clone, A:RenderpassAttachmentSource + Clone> RenderpassFactory<Arc<Renderpass<D, A>>, A> for D{
    fn create_renderpass<S:SubpassDescriptionSource>(&self, attachments: &[&A], subpasses: &[&S], flags: Option<vk::RenderPassCreateFlags>) -> Arc<Renderpass<D,A>> {

        let mut attachments_desc = vec![];
        for attachment in attachments.iter(){
            let mut info = vk::AttachmentDescription::builder();
            if let Some(flags) = attachment.flags(){
                info = info.flags(flags);
            }
            info = info.format(attachment.format())
            .samples(attachment.samples())
            .load_op(attachment.load_op())
            .store_op(attachment.store_op());
            if let Some(op) = attachment.stencil_load(){
                info = info.stencil_load_op(op);
            }
            else{
                info = info.stencil_load_op(vk::AttachmentLoadOp::DONT_CARE);
            }
            if let Some(op) = attachment.stencil_store(){
                info = info.stencil_store_op(op);
            }
            else{
                info = info.stencil_store_op(vk::AttachmentStoreOp::DONT_CARE);
            }
            info = info.initial_layout(attachment.inital_layout())
            .final_layout(attachment.final_layout());
            *attachment.index() = attachments_desc.len() as u32;
            attachments_desc.push(info.build());
        }

        let mut subpass_desc = vec![];
        let mut subpass_dep = vec![];
        for subpass in subpasses.iter(){
            let mut info = vk::SubpassDescription::builder()
            .pipeline_bind_point(subpass.bind_point());
            if let Some(flags) = subpass.flags(){
                info = info.flags(flags);
            }
            if let Some(a) = subpass.input_attachments(){
                info = info.input_attachments(a);
            }
            if let Some(a) = subpass.color_attachments(){
                info = info.color_attachments(a);
            }
            if let Some(a) = subpass.resolve_attachments(){
                info = info.resolve_attachments(a);
            }
            if let Some(a) = subpass.depth_stencil_attachment(){
                info = info.depth_stencil_attachment(a);
            }
            if let Some(a) = subpass.preserve_attachments(){
                info = info.preserve_attachments(a);
            }
            *subpass.index() = subpass_desc.len() as u32;
            subpass_desc.push(info.build());
            if let Some(deps) = subpass.dependencies(){
                subpass_dep.extend_from_slice(deps);
            }
        }
        
        let mut info = vk::RenderPassCreateInfo::builder();
        if let Some(flags) = flags{
            info = info.flags(flags);
        }
        info = info
        .attachments(&attachments_desc)
        .subpasses(&subpass_desc)
        .dependencies(&subpass_dep);

        let renderpass;
        unsafe{
            renderpass = self.device().create_render_pass(&info, None).unwrap();
        }
        info!("Created renderpass {:?}", renderpass);
        Arc::new(
            Renderpass{
                device: self.clone(),
                attachments: attachments_desc,
                subpass_refs: subpass_desc,
                image_views: attachments.iter().map(|a| (*a).clone()).collect(),
            }
        )
    }
}

impl<D:DeviceSource, A:RenderpassAttachmentSource> RenderPassSource for Arc<Renderpass<D,A>>{
    fn renderpass(&self) -> ash::vk::RenderPass {
        todo!()
    }
}

impl<IV:ImageViewSource + Clone> RenderPassAttachment<IV>{
    pub fn new(view: &IV, initial_layout: vk::ImageLayout, subpass_layout: vk::ImageLayout, final_layout: vk::ImageLayout, load_op: vk::AttachmentLoadOp, store_op: vk::AttachmentStoreOp) -> RenderPassAttachment<IV> {
        RenderPassAttachment{
            index: Mutex::new(0),
            view: Mutex::new(view.clone()),
            initial_layout,
            subpass_layout,
            final_layout,
            load_op,
            store_op,
        }
    }
}
impl<IV:ImageViewSource + Clone> RenderpassAttachmentSource for RenderPassAttachment<IV>{
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

impl<A:RenderpassAttachmentSource> SubpassDescription<A>{
    
}

impl<A:RenderpassAttachmentSource> SubpassDescriptionSource for SubpassDescription<A>{
    fn index(&self) -> std::sync::MutexGuard<u32> {
        self.index.lock().unwrap()
    }

    fn flags(&self) -> Option<vk::SubpassDescriptionFlags> {
        self.flags
    }

    fn bind_point(&self) -> vk::PipelineBindPoint {
        self.bind_point
    }

    fn input_attachments(&self) -> Option<&[vk::AttachmentReference]> {
        if self.input_attachments.len() == 0{
            return None;
        }
        let refs:Vec<vk::AttachmentReference> = self.input_attachments.iter().map(|a| {
            vk::AttachmentReference::builder()
            .attachment(*a.index())
            .layout(a.subpass_layout())
            .build()
        }).collect();

        let mut lock = self.input_refs.lock().unwrap();
        *lock = refs;

        Some(&(*self.input_refs)

        

    }

    fn color_attachments(&self) -> Option<&[vk::AttachmentReference]> {
        todo!()
    }

    fn resolve_attachments(&self) -> Option<&[vk::AttachmentReference]> {
        todo!()
    }

    fn depth_stencil_attachment(&self) -> Option<&vk::AttachmentReference> {
        todo!()
    }

    fn preserve_attachments(&self) -> Option<&[u32]> {
        todo!()
    }

    fn dependencies(&self) -> Option<&[vk::SubpassDependency]> {
        todo!()
    }
}
