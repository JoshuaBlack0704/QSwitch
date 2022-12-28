use ash::vk;
use qvk::{self, init::{instance, device, InstanceFactory, DeviceFactory, DeviceSource}, memory::MemoryFactory, image::{ImageFactory, ImageViewFactory, ImageResourceFactory}, pipelines::graphics::{RenderPassAttachment, SubpassDescription, RenderpassFactory}};

fn main(){
    pretty_env_logger::init();
    let settings = instance::Settings::default();
    let instance = settings.create_instance();
    
    let settings = device::Settings::new_simple(instance.clone());
    let device = settings.create_device().expect("Could not create device");

    let memory = device.create_memory(1024 * 1024 * 100, device.device_memory_index(), None).unwrap();
    let extent = vk::Extent3D::builder().width(100).height(100).depth(1).build();
    let image = memory.create_image(&device, vk::Format::B8G8R8A8_SRGB, extent, 1, 1, vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::COLOR_ATTACHMENT, None).unwrap();
    let resource = image.create_resource(vk::Offset3D::default(), extent, 0, vk::ImageAspectFlags::COLOR).unwrap();
    let view = resource.create_image_view(vk::Format::B8G8R8A8_SRGB, vk::ImageViewType::TYPE_2D, None, None);
    let attachment = RenderPassAttachment::new(&view, vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::TRANSFER_SRC_OPTIMAL, vk::AttachmentLoadOp::CLEAR, vk::AttachmentStoreOp::STORE);
    let mut subpass = SubpassDescription::new(vk::PipelineBindPoint::GRAPHICS, &attachment, None);
    subpass.add_color_attachment(&attachment);
    subpass.add_start_dependency();
    let attachments = [&attachment];
    let subpasses = [&subpass];
    let renderpass = device.create_renderpass(&attachments, &subpasses, None);
}
