use std::mem::size_of;

use ash::vk;
use qvk::descriptor::SetFactory;
use qvk::memory::allocators::{BufferAllocatorFactory, MemoryAllocatorFactory, TRANSFER};
use qvk::memory::buffer::{BufferSegmentFactory, BufferSegmentSource};
use qvk::{
    command::{CommandBufferFactory, CommandBufferSource, Executor},
    descriptor::{
        ApplyWriteFactory, DescriptorLayout, DescriptorLayoutFactory, DescriptorPoolFactory,
        SetSource,
    },
    init::{device, instance, DeviceFactory, InstanceFactory},
    pipelines::{ComputePipelineFactory, PipelineLayoutFactory},
    shader::{ShaderFactory, HLSL},
};

fn main() {
    pretty_env_logger::init();

    let settings = instance::Settings::default();
    let instance = settings.create_instance();

    let mut settings = device::Settings::new_simple(instance.clone());
    settings.add_extension(ash::extensions::khr::BufferDeviceAddress::name().as_ptr());
    let device = settings.create_device().expect("Could not create device");

    let host_mem = device.create_cpu_mem(1024 * 1024 * 10);
    let storage = host_mem.create_storage_buffer(1024 * 1024, Some(TRANSFER()));
    let uniform = host_mem.create_uniform_buffer(1024, Some(TRANSFER()));

    let src = [0u32; 100];
    let mut dst = [10u32; 100];
    let data = [src.len() as u32];

    let storage_access = storage.get_segment((size_of::<u32>() * src.len()) as u64, None);
    storage_access.copy_from_ram(&src).unwrap();
    let uniform_access = uniform.get_segment(10, None);
    uniform_access.copy_from_ram(&data).unwrap();

    let dlayout = device.create_descriptor_layout(None);
    let storage_write =
        DescriptorLayout::form_binding(&dlayout, &storage_access, vk::ShaderStageFlags::COMPUTE);
    storage_access.apply(&storage_write);
    let uniform_write =
        DescriptorLayout::form_binding(&dlayout, &uniform_access, vk::ShaderStageFlags::COMPUTE);
    uniform_access.apply(&uniform_write);

    let pool_layouts = [(&dlayout, 1)];
    let dpool = device.create_descriptor_pool(&pool_layouts, None);

    let dset = dpool.create_set(&dlayout);
    dset.update();

    let code = HLSL::new(
        "examples/resources/shaders/increment-set.hlsl",
        shaderc::ShaderKind::Compute,
        "main",
        None,
    );
    let shader = device.create_shader(&code, vk::ShaderStageFlags::COMPUTE, None);

    let playout = device.create_pipeline_layout(&[&dlayout], &[], None);

    let compute = playout.create_compute_pipeline(&shader, None);

    let exe = Executor::new(&device, vk::QueueFlags::COMPUTE);
    let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
    cmd.begin(None).unwrap();
    cmd.bind_pipeline(&compute);
    cmd.bind_set(&dset, 0, &compute);
    cmd.dispatch(data[0], 1, 1);
    cmd.end().unwrap();
    exe.wait_submit_internal();

    storage_access.copy_to_ram(&mut dst).unwrap();
    println!("{:?}", dst);
}
