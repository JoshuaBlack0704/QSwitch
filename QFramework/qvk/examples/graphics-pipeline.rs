use ash::vk;
use qvk::{
    self,
    command::{CommandBufferFactory, CommandBufferSource, CommandPoolOps},
    descriptor::{
        ApplyWriteFactory, DescriptorLayoutFactory, DescriptorPoolFactory, SetFactory, SetSource,
    },
    image::{
        ImageFactory, ImageResourceFactory, ImageResourceSource, ImageSource, ImageViewFactory,
    },
    init::{
        device, instance,
        swapchain::{self, SwapchainSource},
        DeviceFactory, DeviceSource, InstanceFactory, Swapchain,
    },
    memory::{
        buffer::{BufferFactory, BufferSegmentFactory, BufferSegmentSource},
        MemoryFactory,
    },
    pipelines::{
        graphics::{
            graphics::{DefaultVertex, GraphicsDefaultState},
            FramebufferFactory, GraphicsPipelineFactory, RenderPassAttachment, RenderpassFactory,
            SubpassDescription,
        },
        PipelineLayoutFactory,
    },
    queue::{QueueOps, SubmitInfoSource, SubmitSet},
    shader::{ShaderFactory, HLSL, GLSL},
    sync::SemaphoreFactory,
};
use raw_window_handle::HasRawDisplayHandle;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

fn main() {
    pretty_env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut settings = instance::Settings::default();
    settings.use_window_extensions(window.raw_display_handle());
    let instance = settings.create_instance();

    let mut settings = device::Settings::new_simple(instance.clone());
    settings.add_window(&window);
    settings.add_extension(ash::extensions::khr::Swapchain::name().as_ptr());
    let device = settings.create_device().expect("Could not create device");

    let settings = swapchain::SettingsStore::default();
    let swapchain = Swapchain::new(&device, &settings, None).expect("Could not create swapchain");

    let gpu_memory = device
        .create_memory(1024 * 1024 * 100, device.device_memory_index(), None)
        .unwrap();
    let cpu_memory = device
        .create_memory(1024 * 1024 * 100, device.host_memory_index(), None)
        .unwrap();
    let extent = vk::Extent3D::builder()
        .width(1920)
        .height(1080)
        .depth(1)
        .build();
    let extent2d = vk::Extent2D::builder()
        .width(extent.width)
        .height(extent.height)
        .build();
    let clear_value_color = vk::ClearColorValue {
        float32: [0.0, 0.0, 0.0, 1.0],
    };
    let clear_depth_value = vk::ClearDepthStencilValue {
        depth: 1.0,
        stencil: 0,
    };
    let color_image = gpu_memory
        .create_image(
            &device,
            vk::Format::B8G8R8A8_SRGB,
            extent,
            1,
            1,
            vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT,
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
        vk::ClearValue {
            color: clear_value_color,
        },
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        vk::AttachmentLoadOp::CLEAR,
        vk::AttachmentStoreOp::STORE,
    );
    let depth_image = gpu_memory
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
        vk::ClearValue {
            depth_stencil: clear_depth_value,
        },
        vk::ImageLayout::UNDEFINED,
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        vk::AttachmentLoadOp::CLEAR,
        vk::AttachmentStoreOp::STORE,
    );

    let fov = 70.0 * (3.14 / 180.0);
    let aspect = extent.width as f32 / extent.height as f32;
    let perspective = [glam::Mat4::perspective_rh(fov, aspect, 0.1, 10.0)];
    let triangle = [
        DefaultVertex {
            data: [0.5, -0.5, 0.0, 1.0, 0.0, 0.0],
        },
        DefaultVertex {
            data: [0.0, 0.5, 0.0, 0.0, 1.0, 0.0],
        },
        DefaultVertex {
            data: [-0.5, -0.5, 0.0, 0.0, 0.0, 1.0],
        },
    ];
    let indices = [0, 1, 2];

    let ubuff = cpu_memory
        .create_buffer(
            1024,
            vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::UNIFORM_BUFFER,
            None,
            None,
        )
        .unwrap();
    let ubuff = ubuff.create_segment(1024, None).unwrap();
    let v_buff = cpu_memory
        .create_buffer(
            1024,
            vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::VERTEX_BUFFER,
            None,
            None,
        )
        .unwrap();
    let v_buff = v_buff.create_segment(1024, None).unwrap();
    let i_buff = cpu_memory
        .create_buffer(
            1024,
            vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::INDEX_BUFFER,
            None,
            None,
        )
        .unwrap();
    let i_buff = i_buff.create_segment(1024, None).unwrap();
    ubuff.copy_from_ram(&perspective).unwrap();
    v_buff.copy_from_ram(&triangle).unwrap();
    i_buff.copy_from_ram(&indices).unwrap();

    let dlayout = device.create_descriptor_layout(None);
    let uniform_write = dlayout.form_binding(&ubuff, vk::ShaderStageFlags::VERTEX);
    let layouts = [(&dlayout, 1)];
    let dpool = device.create_descriptor_pool(&layouts, None);
    let dset = dpool.create_set(&dlayout);
    ubuff.apply(&uniform_write);
    dset.update();

    let mut subpass = SubpassDescription::new(vk::PipelineBindPoint::GRAPHICS, &depth_attch, None);
    subpass.add_color_attachment(&color_attch);
    subpass.add_depth_stencil_attachment(&depth_attch);
    subpass.add_dependency(
        None,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        vk::AccessFlags::NONE,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE, None
    );
    let attachments = [&color_attch, &depth_attch];
    let subpasses = [&subpass];
    let renderpass = device.create_renderpass(&attachments, &subpasses, None);
    let framebuffer =
        renderpass.create_framebuffer(vk::Rect2D::builder().extent(extent2d).build(), None);
    let layouts = [&dlayout];
    // let playout = device.create_pipeline_layout_empty();
    let playout = device.create_pipeline_layout(&layouts, &[], None);

    let code = HLSL::new(
        "examples/resources/shaders/gp-vertex.hlsl",
        shaderc::ShaderKind::Vertex,
        "main",
        None,
    );
    let vertex_shd = device.create_shader(&code, vk::ShaderStageFlags::VERTEX, None);
    let code = GLSL::new(
        "examples/resources/shaders/gp-fragment.frag",
        shaderc::ShaderKind::Fragment,
        "main",
        None,
    );
    let fragment_shd = device.create_shader(&code, vk::ShaderStageFlags::FRAGMENT, None);

    let shaders = [&vertex_shd, &fragment_shd];
    let state = GraphicsDefaultState::new(extent);
    let state = state.create_state(&shaders);

    let graphics = device
        .create_graphics_pipeline(&state, &playout, &renderpass, 0)
        .unwrap();

    let exe = qvk::command::Executor::new(&device, vk::QueueFlags::GRAPHICS);
    let aquire = device.create_semaphore();
    let render = device.create_semaphore();
    let dbuff = gpu_memory
        .create_buffer(
            1024 * 1024 * 50,
            vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
            None,
            None,
        )
        .unwrap();
    let dbuff = dbuff.create_segment(1024 * 1024 * 50, None).unwrap();
    let mut images = swapchain.images();
    event_loop.run(move |event, _, flow| {
        flow.set_poll();
        match event {
            Event::WindowEvent {
                window_id: _,
                event,
            } => {
                if let WindowEvent::CloseRequested = event {
                    flow.set_exit();
                }
                if let WindowEvent::Resized(_) = event {
                    swapchain.resize();
                    images = swapchain.images();
                    println!("{:?}", swapchain.extent());
                }
            }
            Event::MainEventsCleared => {
                let index = swapchain.gpu_aquire_next_image(u64::MAX, &aquire);
                let color_tgt = images[index as usize].clone();
                let color_tgt = color_tgt
                    .create_resource(
                        vk::Offset3D::default(),
                        color_tgt.extent(),
                        0,
                        vk::ImageAspectFlags::COLOR,
                    )
                    .unwrap();
                *ImageResourceSource::layout(&color_rsc) = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
                *ImageResourceSource::layout(&depth_rsc) = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
                let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
                cmd.begin(None).unwrap();
                cmd.begin_render_pass(&framebuffer);
                cmd.bind_pipeline(&graphics);
                cmd.bind_vertex_bufer(&v_buff);
                cmd.bind_index_bufer(&i_buff);
                cmd.bind_set(&dset, 0, &graphics);
                unsafe {
                    device
                        .device()
                        .cmd_draw(cmd.cmd(), triangle.len() as u32, 1, 0, 0);
                }
                cmd.end_render_pass();
                cmd.transition_img(
                    &color_tgt,
                    vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    vk::PipelineStageFlags2::TRANSFER,
                    vk::AccessFlags2::MEMORY_WRITE,
                    vk::PipelineStageFlags2::TRANSFER,
                    vk::AccessFlags2::MEMORY_READ,
                );
                // cmd.image_buffer_copy(&depth_rsc, &dbuff, None).unwrap();
                // cmd.mem_barrier(
                //     vk::PipelineStageFlags2::TRANSFER,
                //     vk::AccessFlags2::TRANSFER_WRITE,
                //     vk::PipelineStageFlags2::TRANSFER,
                //     vk::AccessFlags2::MEMORY_READ,
                // );
                // cmd.buffer_image_copy(&dbuff, &color_rsc, None).unwrap();
                cmd.transition_img(
                    &color_rsc,
                    vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    vk::PipelineStageFlags2::TRANSFER,
                    vk::AccessFlags2::MEMORY_WRITE,
                    vk::PipelineStageFlags2::TRANSFER,
                    vk::AccessFlags2::MEMORY_READ,
                );
                cmd.image_blit(&color_rsc, &color_tgt, vk::Filter::LINEAR)
                    .unwrap();
                cmd.transition_img(
                    &color_tgt,
                    vk::ImageLayout::PRESENT_SRC_KHR,
                    vk::PipelineStageFlags2::TRANSFER,
                    vk::AccessFlags2::MEMORY_WRITE,
                    vk::PipelineStageFlags2::TRANSFER,
                    vk::AccessFlags2::MEMORY_READ,
                );
                cmd.end().unwrap();
                let mut submit = SubmitSet::new(&cmd);
                submit.add_wait(&aquire, vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT);
                submit.add_signal(&render, vk::PipelineStageFlags2::BOTTOM_OF_PIPE);
                let submit = [submit];
                exe.gpu_submit(&submit).unwrap();

                let waits = [&render];
                swapchain.wait_present(index as u32, Some(&waits));

                exe.reset_cmdpool();
            }
            _ => {}
        }
    })
}
