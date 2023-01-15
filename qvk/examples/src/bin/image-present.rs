use ash::vk;
use qvk::{
    init::{self, IVulkanInit, SwapchainStore, WindowedInitalizer},
    memory::{
        AlignmentType, AllocationAllocatorProfile, Allocator, AllocatorProfileStack,
        AllocatorProfileType, BufferAllocatorProfile, CreateAllocationOptions, CreateBufferOptions,
        ImageAllocatorProfile,
    },
    sync, IDisposable,
};

#[cfg(debug_assertions)]
fn get_vulkan_validate(options: &mut Vec<init::VulkanInitOptions>) {
    println!("Validation Layers Active");
    let validation_features = [
        vk::ValidationFeatureEnableEXT::BEST_PRACTICES,
        vk::ValidationFeatureEnableEXT::GPU_ASSISTED,
        //vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION,
    ];
    options.push(init::VulkanInitOptions::UseValidation(
        Some(validation_features.to_vec()),
        None,
    ))
}
#[cfg(not(debug_assertions))]
fn get_vulkan_validate(options: &mut Vec<init::VulkanInitOptions>) {
    println!("Validation Layers Inactive");
}

#[allow(unused)]
fn main() {
    let (event_loop, engine);
    {
        match pretty_env_logger::try_init() {
            Ok(_) => {}
            Err(_) => {}
        };
        let mut engine_options = vec![
            init::VulkanInitOptions::UseDebugUtils,
            init::VulkanInitOptions::DeviceExtensions(vec![]),
        ];
        get_vulkan_validate(&mut engine_options);
        (event_loop, engine) = WindowedInitalizer::init(&mut engine_options);
    }

    let mut swapchain = SwapchainStore::new(
        engine.clone(),
        &[init::CreateSwapchainOptions::ImageUsages(
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT,
        )],
    );

    let mut width: u32 = swapchain.get_extent().width;
    let mut height: u32 = swapchain.get_extent().height;
    let mut extent = vk::Extent3D::builder()
        .width(width)
        .height(height)
        .depth(1)
        .build();

    let mem_options = vec![CreateAllocationOptions::MinimumSize(1024 * 1024 * 100)];
    let buffer_options = vec![CreateBufferOptions::MinimumSize(1024 * 1024)];
    let mut allocator = Allocator::new(engine.clone());

    let cpu_mem_profile = allocator.add_profile(AllocatorProfileType::Allocation(
        AllocationAllocatorProfile::new(vk::MemoryPropertyFlags::HOST_COHERENT, &mem_options),
    ));
    let gpu_mem_profile = allocator.add_profile(AllocatorProfileType::Allocation(
        AllocationAllocatorProfile::new(vk::MemoryPropertyFlags::DEVICE_LOCAL, &mem_options),
    ));
    let buffer_profile =
        allocator.add_profile(AllocatorProfileType::Buffer(BufferAllocatorProfile::new(
            vk::BufferUsageFlags::STORAGE_BUFFER
                | vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::TRANSFER_DST,
            &buffer_options,
        )));
    let image_profile =
        allocator.add_profile(AllocatorProfileType::Image(ImageAllocatorProfile::new(
            vk::ImageUsageFlags::TRANSFER_SRC
                | vk::ImageUsageFlags::TRANSFER_DST
                | vk::ImageUsageFlags::STORAGE,
            vk::Format::B8G8R8A8_UNORM,
            extent,
            &[],
        )));
    let cpu_stack = AllocatorProfileStack::TargetBuffer(cpu_mem_profile, buffer_profile);
    let gpu_stack = AllocatorProfileStack::TargetImage(gpu_mem_profile, image_profile);

    let pool = unsafe {
        engine
            .get_device()
            .create_command_pool(
                &vk::CommandPoolCreateInfo::builder()
                    .queue_family_index(
                        engine
                            .get_queue_store()
                            .get_queue(vk::QueueFlags::TRANSFER)
                            .unwrap()
                            .1,
                    )
                    .build(),
                None,
            )
            .expect("Could not create command pool")
    };
    let cmd = unsafe {
        engine
            .get_device()
            .allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(pool)
                    .command_buffer_count(1)
                    .build(),
            )
            .expect("Could not allocate command buffers")
    }[0];

    let mut data: Vec<u32> = vec![u32::from_be_bytes([255, 255, 0, 0]); (width * height) as usize];

    let mut staging =
        allocator.get_buffer_region::<u32>(&cpu_stack, data.len(), &AlignmentType::Free, &[]);

    let mut processing = allocator.get_image_resources(
        &gpu_stack,
        vk::ImageAspectFlags::COLOR,
        0,
        1,
        0,
        1,
        vk::ImageViewType::TYPE_2D,
        vk::Format::B8G8R8A8_UNORM,
        &[],
    );

    let mut copy_done = sync::Fence::new(engine.clone(), true);
    let mut aquire_semaphore = sync::Semaphore::new(engine.clone());
    let mut present_semaphore = sync::Semaphore::new(engine.clone());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            winit::event::Event::WindowEvent { event, .. } => match event {
                winit::event::WindowEvent::CloseRequested => {
                    *control_flow = winit::event_loop::ControlFlow::Exit;
                }
                winit::event::WindowEvent::Resized(_) => {
                    copy_done.wait();
                    swapchain = SwapchainStore::new(
                        engine.clone(),
                        &[
                            init::CreateSwapchainOptions::OldSwapchain(&swapchain),
                            init::CreateSwapchainOptions::ImageUsages(
                                vk::ImageUsageFlags::TRANSFER_DST
                                    | vk::ImageUsageFlags::COLOR_ATTACHMENT,
                            ),
                        ],
                    );

                    width = swapchain.get_extent().width;
                    height = swapchain.get_extent().height;
                    data = vec![u32::from_le_bytes([255, 0, 0, 0]); (width * height) as usize];
                    extent = vk::Extent3D::builder()
                        .width(width)
                        .height(height)
                        .depth(1)
                        .build();

                    processing.dispose();

                    allocator.update_image(
                        image_profile,
                        &ImageAllocatorProfile::new(
                            vk::ImageUsageFlags::TRANSFER_SRC
                                | vk::ImageUsageFlags::TRANSFER_DST
                                | vk::ImageUsageFlags::STORAGE,
                            vk::Format::B8G8R8A8_UNORM,
                            extent,
                            &[],
                        ),
                    );

                    staging = allocator.get_buffer_region::<u32>(
                        &cpu_stack,
                        data.len(),
                        &AlignmentType::Free,
                        &[],
                    );

                    processing = allocator.get_image_resources(
                        &gpu_stack,
                        vk::ImageAspectFlags::COLOR,
                        0,
                        1,
                        0,
                        1,
                        vk::ImageViewType::TYPE_2D,
                        vk::Format::B8G8R8A8_UNORM,
                        &[],
                    );
                }
                winit::event::WindowEvent::KeyboardInput {
                    device_id,
                    input,
                    is_synthetic,
                } => {}
                _ => {}
            },
            winit::event::Event::MainEventsCleared => {
                allocator.copy_from_ram_slice(&data, &staging);

                unsafe {
                    copy_done.wait_reset();
                    engine
                        .get_device()
                        .reset_command_pool(pool, vk::CommandPoolResetFlags::empty())
                        .expect("Could not reset command pool");
                    let (index, next_image) =
                        swapchain.get_next_image(u64::MAX, Some(aquire_semaphore.semaphore), None);

                    engine
                        .get_device()
                        .begin_command_buffer(
                            cmd,
                            &vk::CommandBufferBeginInfo::builder()
                                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                                .build(),
                        )
                        .unwrap();
                    let (processing_dst, _) = processing.transition(
                        vk::AccessFlags::NONE,
                        vk::AccessFlags::TRANSFER_WRITE,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    );
                    let (swap_chain_dst, _) = next_image.transition(
                        vk::AccessFlags::NONE,
                        vk::AccessFlags::TRANSFER_WRITE,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    );
                    let image_barriers = [processing_dst, swap_chain_dst];
                    engine.get_device().cmd_pipeline_barrier(
                        cmd,
                        vk::PipelineStageFlags::TOP_OF_PIPE,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &image_barriers,
                    );

                    staging.copy_to_image(cmd, &processing);

                    //let mem_barrier = vk::MemoryBarrier::builder().src_access_mask(vk::AccessFlags::TRANSFER_WRITE).dst_access_mask(vk::AccessFlags::MEMORY_READ).build();
                    let (processing_src, _) = processing.transition(
                        vk::AccessFlags::TRANSFER_WRITE,
                        vk::AccessFlags::TRANSFER_READ,
                        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                    );
                    let image_barriers = [processing_src];
                    engine.get_device().cmd_pipeline_barrier(
                        cmd,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &image_barriers,
                    );

                    processing.copy_to_image(cmd, &next_image);

                    let (swapchain_present, _) = next_image.transition(
                        vk::AccessFlags::TRANSFER_WRITE,
                        vk::AccessFlags::NONE,
                        vk::ImageLayout::PRESENT_SRC_KHR,
                    );
                    let image_barriers = [swapchain_present];
                    engine.get_device().cmd_pipeline_barrier(
                        cmd,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &image_barriers,
                    );

                    engine.get_device().end_command_buffer(cmd).unwrap();
                    let cmds = [cmd];
                    let wait_semaphores = [aquire_semaphore.semaphore];
                    let wait_masks = [vk::PipelineStageFlags::TOP_OF_PIPE];
                    let signal_semaphores = [present_semaphore.semaphore];

                    let submit = [vk::SubmitInfo::builder()
                        .wait_semaphores(&wait_semaphores)
                        .wait_dst_stage_mask(&wait_masks)
                        .signal_semaphores(&signal_semaphores)
                        .command_buffers(&cmds)
                        .build()];
                    engine
                        .get_device()
                        .queue_submit(
                            engine
                                .get_queue_store()
                                .get_queue(vk::QueueFlags::TRANSFER)
                                .unwrap()
                                .0,
                            &submit,
                            copy_done.get_fence(),
                        )
                        .unwrap();

                    swapchain.present(
                        engine
                            .get_queue_store()
                            .get_queue(vk::QueueFlags::GRAPHICS)
                            .unwrap()
                            .0,
                        index,
                        &signal_semaphores,
                    );
                }
            }
            winit::event::Event::LoopDestroyed => {
                unsafe {
                    engine
                        .get_device()
                        .device_wait_idle()
                        .expect("Could not wait for device to idle")
                };
                unsafe {
                    engine.get_device().destroy_command_pool(pool, None);
                }
                processing.dispose();
                allocator.dispose();
                copy_done.dispose();
                aquire_semaphore.dispose();
                present_semaphore.dispose();
            }
            _ => {}
        }
    });
    {}
    println!("Hello World");
    engine.get_device();
}
