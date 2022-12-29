use std::sync::Arc;

use ash::vk;
use qvk::{
    self,
    image::{ImageFactory, ImageResourceFactory, ImageViewFactory},
    init::{device, instance, DeviceFactory, DeviceSource, InstanceFactory},
    memory::MemoryFactory,
    pipelines::{
        graphics::{RenderPassAttachment, RenderpassFactory, SubpassDescription, graphics::GraphicsDefaultState, GraphicsPipelineFactory},
        PipelineLayoutFactory,
    }, descriptor::WriteHolder, shader::{HLSL, ShaderFactory},
};

fn main() {
    pretty_env_logger::init();
    let settings = instance::Settings::default();
    let instance = settings.create_instance();

    let settings = device::Settings::new_simple(instance.clone());
    let device = settings.create_device().expect("Could not create device");

    let memory = device
        .create_memory(1024 * 1024 * 100, device.device_memory_index(), None)
        .unwrap();
    let extent = vk::Extent3D::builder()
        .width(100)
        .height(100)
        .depth(1)
        .build();
    let color_image = memory
        .create_image(
            &device,
            vk::Format::B8G8R8A8_SRGB,
            extent,
            1,
            1,
            vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            None,
        )
        .unwrap();
    let color_rsc = color_image
        .create_resource(
            vk::Offset3D::default(),
            extent,
            0,
            vk::ImageAspectFlags::COLOR,
        )
        .unwrap();
    let color_view = color_rsc.create_image_view(
        vk::Format::B8G8R8A8_SRGB,
        vk::ImageViewType::TYPE_2D,
        None,
        None,
    );
    let color_attch = RenderPassAttachment::new(
        &color_view,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        vk::AttachmentLoadOp::CLEAR,
        vk::AttachmentStoreOp::STORE,
    );
    let depth_image = memory
        .create_image(
            &device,
            vk::Format::D32_SFLOAT,
            extent,
            1,
            1,
            vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
            None,
        )
        .unwrap();
    let depth_rsc = depth_image
        .create_resource(
            vk::Offset3D::default(),
            extent,
            0,
            vk::ImageAspectFlags::DEPTH,
        )
        .unwrap();
    let depth_view = depth_rsc.create_image_view(
        vk::Format::D32_SFLOAT,
        vk::ImageViewType::TYPE_2D,
        None,
        None,
    );
    let depth_attch = RenderPassAttachment::new(
        &depth_view,
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        vk::AttachmentLoadOp::CLEAR,
        vk::AttachmentStoreOp::DONT_CARE,
    );
    let mut subpass = SubpassDescription::new(vk::PipelineBindPoint::GRAPHICS, &depth_attch, None);
    subpass.add_color_attachment(&color_attch);
    subpass.add_depth_stencil_attachment(&depth_attch);
    subpass.add_start_dependency();
    subpass.add_depth_dependency();
    let attachments = [&color_attch, &depth_attch];
    let subpasses = [&subpass];
    let renderpass = device.create_renderpass(&attachments, &subpasses, None);
    let playout = device.create_pipeline_layout_empty();
    
    let code = HLSL::new("examples/resources/shaders/gp-vertex.hlsl", shaderc::ShaderKind::Vertex, "main", None);
    let vertex_shd = device.create_shader(&code, vk::ShaderStageFlags::VERTEX, None);
    let code = HLSL::new("examples/resources/shaders/gp-fragment.hlsl", shaderc::ShaderKind::Fragment, "main", None);
    let fragment_shd = device.create_shader(&code, vk::ShaderStageFlags::FRAGMENT, None);

    let shaders = [&vertex_shd, &fragment_shd];
    let state = GraphicsDefaultState::new(extent);
    let state = state.create_state(&shaders);

    let graphics = device.create_graphics_pipeline(&state, &playout, &renderpass, 0).unwrap();
}
