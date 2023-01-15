use std::mem::size_of;
use std::time::Instant;

use ash::vk;
use glam::{Mat4, Vec3, Vec4};
use qprimitives::{Shape, ShapeVertex};
use qvk::{
    command::{CommandBufferFactory, CommandBufferSource, CommandPoolOps},
    descriptor::{
        ApplyWriteFactory, DescriptorLayoutFactory, DescriptorPoolFactory, SetFactory, SetSource,
    },
    init::{
        device, instance,
        swapchain::{self, SwapchainSource},
        DeviceFactory, DeviceSource, InstanceFactory, Swapchain,
    },
    pipelines::{
        graphics::{
            graphics::GraphicsDefaultState, FramebufferFactory, GraphicsPipelineFactory,
            RenderPassAttachment, RenderpassFactory, SubpassDescription,
        },
        ComputePipelineFactory, PipelineLayoutFactory,
    },
    queue::{QueueOps, SubmitInfoSource, SubmitSet},
    shader::{ShaderFactory, HLSL, SPV},
    sync::SemaphoreFactory, memory::{allocators::{MemoryAllocatorFactory, ImageAllocatorFactory, BufferAllocatorFactory, TRANSFER}, image::{ImageFactory, ImageResourceFactory, ImageViewFactory, ImageSource, ImageResourceSource}, buffer::{BufferSegmentFactory, BufferSegmentSource}},
};
use rand::{thread_rng, Rng};
use raw_window_handle::HasRawDisplayHandle;
use winit::{
    event::{Event, VirtualKeyCode, WindowEvent},
    event_loop::EventLoop,
    window::WindowBuilder,
};

const VERT_PATH: &str = "examples/resources/gp-drone/vert.hlsl";
const FRAG_PATH: &str = "examples/resources/gp-drone/frag.hlsl";
const COMP_PATH: &str = "examples/resources/gp-drone/update.hlsl";
const VERT_PATH_SPV: &str = "examples/resources/gp-drone/vert.spv";
const FRAG_PATH_SPV: &str = "examples/resources/gp-drone/frag.spv";
const COMP_PATH_SPV: &str = "examples/resources/gp-drone/update.spv";
const CAMSPEED: f32 = 30.0;
const CAMRATE: f32 = 1.0;

#[derive(Clone)]
#[repr(C)]
pub struct InstanceData {
    mvp_matrix: Mat4,
    target_pos: Vec4,
    current_pos: Vec4,
}

#[repr(C)]
struct UniformData {
    projection: Mat4,
    view: Mat4,
    object_count: u32,
    target_count: u32,
    delta_time: f32,
    frame: u32,
}

fn generate_targets(max_x: f32, max_y: f32, max_z: f32, count: usize) -> Vec<Vec4> {
    let mut targets: Vec<Vec4> = vec![];

    for _ in 0..count {
        let x: f32 = thread_rng().gen_range(0.0..max_x);
        let y: f32 = thread_rng().gen_range(0.0..max_y);
        let z: f32 = thread_rng().gen_range(0.0..max_z);
        let target = Vec4::new(x, y, z, 1.0);
        targets.push(target);
    }

    targets
}

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

    let gpu_memory = device.create_gpu_mem(1024 * 1024 * 50);
    let cpu_memory = device.create_cpu_mem(1024 * 1024 * 50);
    let color_image_alloc = gpu_memory.create_image_allocator_simple(
        vk::Format::B8G8R8A8_SRGB,
        vk::ImageUsageFlags::TRANSFER_SRC
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::COLOR_ATTACHMENT,
    );
    let depth_image_alloc = gpu_memory.create_image_allocator_simple(
        vk::Format::D32_SFLOAT,
        vk::ImageUsageFlags::TRANSFER_SRC
            | vk::ImageUsageFlags::TRANSFER_DST
            | vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
    );
    let cpu_storage = cpu_memory.create_storage_buffer(
        1024 * 1024 * 5, Some(TRANSFER())
    );
    let gpu_storage = gpu_memory.create_storage_buffer(
        1024 * 1024 * 5, Some(TRANSFER())
    );
    let cpu_uniform = cpu_memory.create_uniform_buffer(1024 * 1024, Some(vk::BufferUsageFlags::TRANSFER_DST));
    let v_buffer = gpu_memory.create_vertex_buffer(1024, Some(vk::BufferUsageFlags::TRANSFER_DST));
    let i_buffer = gpu_memory.create_index_buffer(1024, Some(vk::BufferUsageFlags::TRANSFER_DST));

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
    let color_image = color_image_alloc.create_image(extent);
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
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        vk::AttachmentLoadOp::CLEAR,
        vk::AttachmentStoreOp::STORE,
    );
    let depth_image = depth_image_alloc.create_image(extent);
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
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
        vk::AttachmentLoadOp::CLEAR,
        vk::AttachmentStoreOp::DONT_CARE,
    );

    // Memory preperation
    let object_count = 100 as u32;
    let target_count = 10 as u32;
    let max_x = 100.0;
    let max_y = 100.0;
    let max_z = 100.0;

    let uniform = cpu_uniform.get_segment(1024, None);
    let instance_data =
        gpu_storage.get_segment(size_of::<InstanceData>() as u64 * object_count as u64, None);
    let target_data = gpu_storage.get_segment(size_of::<Vec4>() as u64 * target_count as u64, None);
    let v_buffer = v_buffer.get_segment(1024 * 1024, None);
    let i_buffer = i_buffer.get_segment(1024 * 1024, None);

    let mut vertex_data = vec![];
    let mut index_data = vec![];
    let _shape = Shape::tetrahedron(&mut vertex_data, &mut index_data);

    let stage = cpu_storage.get_segment(v_buffer.size(), None);
    stage.copy_from_ram(&vertex_data).unwrap();
    stage.copy_to_segment_internal(&v_buffer).unwrap();

    let stage = cpu_storage.get_segment(i_buffer.size(), None);
    stage.copy_from_ram(&index_data).unwrap();
    stage.copy_to_segment_internal(&i_buffer).unwrap();

    let temp_data = generate_targets(max_x, max_y, max_z, target_count as usize);
    let stage = cpu_storage.get_segment(target_data.size(), None);
    stage.copy_from_ram(&temp_data).unwrap();
    stage.copy_to_segment_internal(&target_data).unwrap();

    // Descriptor preperation
    let comp_dlayout = device.create_descriptor_layout(None);
    let vert_dlayout = device.create_descriptor_layout(None);

    let comp_uniform_write = comp_dlayout.form_binding(&uniform, vk::ShaderStageFlags::COMPUTE);
    let comp_instance_write =
        comp_dlayout.form_binding(&instance_data, vk::ShaderStageFlags::COMPUTE);
    let comp_target_write = comp_dlayout.form_binding(&target_data, vk::ShaderStageFlags::COMPUTE);

    let vert_uniform_write = vert_dlayout.form_binding(&uniform, vk::ShaderStageFlags::VERTEX);
    let vert_instance_write =
        vert_dlayout.form_binding(&instance_data, vk::ShaderStageFlags::VERTEX);

    uniform.apply(&comp_uniform_write);
    uniform.apply(&vert_uniform_write);

    instance_data.apply(&comp_instance_write);
    instance_data.apply(&vert_instance_write);

    target_data.apply(&comp_target_write);

    let layouts = [(&comp_dlayout, 1), (&vert_dlayout, 1)];
    let dpool = device.create_descriptor_pool(&layouts, None);

    let comp_dset = dpool.create_set(&comp_dlayout);
    comp_dset.update();
    let vert_dset = dpool.create_set(&vert_dlayout);
    vert_dset.update();

    // Compute pipeline preparation
    let code = HLSL::new(COMP_PATH, shaderc::ShaderKind::Compute, "main", None);
    let compute_shd = device.create_shader(&code, vk::ShaderStageFlags::COMPUTE, None);

    let layouts = [&comp_dlayout];
    let cplayout = device.create_pipeline_layout(&layouts, &[], None);
    let mut compute_pipeline = cplayout.create_compute_pipeline(&compute_shd, None);

    // Graphics pipeline preparation
    let mut subpass = SubpassDescription::new(vk::PipelineBindPoint::GRAPHICS, &depth_attch, None);
    subpass.add_color_attachment(&color_attch);
    subpass.add_depth_stencil_attachment(&depth_attch);
    subpass.add_dependency(
        None,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
            | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        vk::AccessFlags::NONE,
        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
            | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        vk::AccessFlags::COLOR_ATTACHMENT_WRITE | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        None,
    );
    let attachments = [&color_attch, &depth_attch];
    let subpasses = [&subpass];
    let renderpass = device.create_renderpass(&attachments, &subpasses, None);
    let framebuffer =
        renderpass.create_framebuffer(vk::Rect2D::builder().extent(extent2d).build(), None);
    let layouts = [&vert_dlayout];
    // let playout = device.create_pipeline_layout_empty();
    let playout = device.create_pipeline_layout(&layouts, &[], None);

    let code = HLSL::new(VERT_PATH, shaderc::ShaderKind::Vertex, "main", None);
    let vertex_shd = device.create_shader(&code, vk::ShaderStageFlags::VERTEX, None);
    let code = HLSL::new(FRAG_PATH, shaderc::ShaderKind::Fragment, "main", None);
    let fragment_shd = device.create_shader(&code, vk::ShaderStageFlags::FRAGMENT, None);

    let shaders = [&vertex_shd, &fragment_shd];
    let def_state = GraphicsDefaultState::<ShapeVertex>::new(extent);
    let mut state = def_state.create_state(&shaders);

    let mut graphics = device
        .create_graphics_pipeline(&state, &playout, &renderpass, 0)
        .unwrap();

    let exe = qvk::command::Executor::new(&device, vk::QueueFlags::GRAPHICS);
    let aquire = device.create_semaphore();
    let render = device.create_semaphore();
    let mut images = swapchain.images();
    let mut camera =
        qvk::camera::Camera::new(Vec3::new(max_x / 2.0, max_y / 2.0, 0.0), CAMSPEED, CAMRATE);
    // let mut camera = qvk::camera::Camera::new(Vec3::new(0.0,0.0,0.0), CAMSPEED, CAMRATE);
    let mut time_at_last_frame = Instant::now();
    let mut watch = Instant::now();
    let mut frame: u32 = 0;
    let mut delta_time = 0.0;
    event_loop.run(move |event, _, flow| {
        flow.set_poll();
        match event {
            Event::WindowEvent {
                window_id: _,
                event,
            } => {
                if let WindowEvent::KeyboardInput {
                    device_id: _,
                    input,
                    is_synthetic: _,
                } = event
                {
                    camera.process_input(input.clone());
                    if let Some(code) = input.virtual_keycode {
                        if code == VirtualKeyCode::Space {
                            println!("Recompiling shaders");
                            let code =
                                // HLSL::new(VERT_PATH, shaderc::ShaderKind::Vertex, "main", None);
                                SPV::new(VERT_PATH_SPV,  "main");
                            let vertex_shd =
                                device.create_shader(&code, vk::ShaderStageFlags::VERTEX, None);
                            let code =
                                // HLSL::new(FRAG_PATH, shaderc::ShaderKind::Fragment, "main", None);
                                SPV::new(FRAG_PATH_SPV, "main");
                            let fragment_shd =
                                device.create_shader(&code, vk::ShaderStageFlags::FRAGMENT, None);

                            let shaders = [&vertex_shd, &fragment_shd];
                            let state = &mut state;
                            *state = def_state.create_state(&shaders);
                            graphics = device
                                .create_graphics_pipeline(state, &playout, &renderpass, 0)
                                .unwrap();

                            let code =
                                // HLSL::new(COMP_PATH, shaderc::ShaderKind::Compute, "main", None);
                                SPV::new(COMP_PATH_SPV, "main");
                            let compute_shd =
                                device.create_shader(&code, vk::ShaderStageFlags::COMPUTE, None);

                            let layouts = [&comp_dlayout];
                            let cplayout = device.create_pipeline_layout(&layouts, &[], None);
                            compute_pipeline = cplayout.create_compute_pipeline(&compute_shd, None);
                        }
                    }
                }
                if let WindowEvent::CloseRequested = event {
                    flow.set_exit();
                }
                if let WindowEvent::Resized(size) = event {
                    swapchain.resize(Some((size.width, size.height)));
                    images = swapchain.images();
                    println!("{:?}", swapchain.extent());
                }
            }
            Event::MainEventsCleared => {
                let delta_time = &mut delta_time;
                *delta_time = time_at_last_frame.elapsed().as_secs_f32();
                if watch.elapsed().as_secs() >= 3 {
                    watch = Instant::now();
                    println!("Frame time: {delta_time}, FPS: {}", 1.0 / (*delta_time));
                }
                time_at_last_frame = Instant::now();
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

                let fov: f32 = 70.0 * (3.14 / 180.0);
                let aspect: f32 = ImageResourceSource::extent(&color_tgt).width as f32
                    / ImageResourceSource::extent(&color_tgt).height as f32;
                let u_data = [UniformData {
                    projection: camera.perspective(fov, aspect),
                    view: camera.view(*delta_time),
                    object_count: object_count as u32,
                    target_count: target_count as u32,
                    delta_time: *delta_time,
                    frame,
                }];
                uniform.copy_from_ram(&u_data).unwrap();

                *ImageResourceSource::layout(&color_rsc) = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
                let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
                cmd.begin(None).unwrap();
                cmd.bind_pipeline(&compute_pipeline);
                cmd.bind_set(&comp_dset, 0, &compute_pipeline);
                cmd.dispatch(object_count, 1, 1);
                cmd.begin_render_pass(&framebuffer);
                cmd.bind_pipeline(&graphics);
                cmd.bind_vertex_buffer(&v_buffer);
                cmd.bind_index_buffer(&i_buffer);
                cmd.bind_set(&vert_dset, 0, &graphics);
                unsafe {
                    device.device().cmd_draw_indexed(
                        cmd.cmd(),
                        index_data.len() as u32,
                        object_count,
                        0,
                        0,
                        0,
                    );
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
                frame += 1;
            }
            _ => {}
        }
    })
}
