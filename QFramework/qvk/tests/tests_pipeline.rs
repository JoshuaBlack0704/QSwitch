use ash::vk;
use qvk::{descriptor::{self, DescriptorLayout, ApplyWriteFactory, DescriptorLayoutFactory, SetFactory, SetSource}, init::{device, instance, InstanceFactory, DeviceFactory}, memory::{buffer::{BufferSegmentSource, BufferFactory, BufferSegmentFactory}, MemoryFactory}, pipelines::{PipelineLayoutFactory, ComputePipelineFactory}, shader::{HLSL, ShaderFactory}, command::{Executor, CommandBufferFactory, CommandBufferSource}};
use qvk::init::DeviceSource;
use std::mem::size_of;

#[test]
fn compute_pipeline(){
    let settings = instance::Settings::default();
    let instance = settings.create_instance();
    
    let mut settings = device::Settings::new_simple(instance.clone());
    settings.add_extension(ash::extensions::khr::BufferDeviceAddress::name().as_ptr());
    let device = settings.create_device().expect("Could not create device");

    let host_mem = device.create_memory(1024 * 1024 * 10, device.host_memory_index(), None).unwrap();

    let src = [0u32;100];
    let mut dst = [10u32;100];
    let data = [src.len() as u32];

    let storage = host_mem.create_buffer(1024 * 1024, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, None, None).unwrap();
    let storage_access = storage.create_segment((size_of::<u32>() * src.len()) as u64, None).unwrap();
    storage_access.copy_from_ram(&src).unwrap();
    let uniform = host_mem.create_buffer(1024, vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, None, None).unwrap();
    let uniform_access = uniform.create_segment(10, None).unwrap();
    uniform_access.copy_from_ram(&data).unwrap();

    let dlayout = device.create_descriptor_layout(None);
    let storage_write = DescriptorLayout::form_binding(&dlayout, &storage_access, vk::ShaderStageFlags::COMPUTE);
    storage_access.apply(&storage_write);
    let uniform_write = DescriptorLayout::form_binding(&dlayout, &uniform_access, vk::ShaderStageFlags::COMPUTE);
    uniform_access.apply(&uniform_write);

    let pool_layouts = [(&dlayout, 1)];
    let dpool = descriptor::Pool::new(&device, &pool_layouts, None);

    let dset = dpool.create_set(&dlayout);
    dset.update();

    let code = HLSL::new("tests/resources/shaders/increment-set.hlsl", shaderc::ShaderKind::Compute, "main", None);
    let shader = device.create_shader(&code, vk::ShaderStageFlags::COMPUTE, None);

    let playout = device.create_pipeline_layout(&[&dlayout], &[], None);

    let compute = playout.create_compute_pipeline(&shader, None);

    let exe = Executor::new(&device, vk::QueueFlags::COMPUTE);
    let cmd = exe.next_cmd(vk::CommandBufferLevel::PRIMARY);
    cmd.begin(None).unwrap();
    cmd.bind_pipeline(&compute);
    cmd.bind_set(&dset, 0, &compute);
    cmd.dispatch(data[0],1,1);
    cmd.end().unwrap();
    exe.wait_submit_internal();

    storage_access.copy_to_ram(&mut dst).unwrap();
    for (index, num) in dst.iter().enumerate(){
        assert_eq!(*num, src[index] + 1);
    }
    println!("{:?}",dst);
}