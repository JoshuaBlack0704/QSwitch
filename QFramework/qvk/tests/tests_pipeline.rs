use ash::vk;
use qvk::{descriptor::{self, DescriptorLayout, ApplyWriteFactory, DescriptorLayoutFactory, SetFactory, SetSource}, init::{device, instance, InstanceFactory, DeviceFactory}, memory::{buffer::{buffer, Buffer, BufferSegment, BufferSegmentStore}, memory, Memory}, pipelines, shader::{HLSL, ShaderFactory}, command::{Executor, CommandBufferFactory, CommandBufferSource}};
use qvk::init::DeviceSource;
use std::mem::size_of;

#[test]
fn compute_pipeline(){
    let settings = instance::Settings::default();
    let instance = settings.create_instance();
    
    let mut settings = device::Settings::new_simple(instance.clone());
    settings.add_extension(ash::extensions::khr::BufferDeviceAddress::name().as_ptr());
    let device = settings.create_device().expect("Could not create device");

    let settings = memory::SettingsStore::new(1024 * 1024 * 10, device.host_memory_index());
    let host_mem = Memory::new(&settings, &device).expect("Could not allocate memory");

    let src = [0u32;100];
    let mut dst = [10u32;100];
    let data = [src.len() as u32];

    let settings = buffer::SettingsStore::new(1024 * 1024, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST);
    let storage = Buffer::new(&settings, &device, &host_mem).expect("Could not bind buffer");
    let storage_access = BufferSegment::new(&storage, (size_of::<u32>() * src.len()) as u64, None).unwrap();
    storage_access.copy_from_ram(&src).unwrap();
    let settings = buffer::SettingsStore::new(1024, vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST);
    let uniform = Buffer::new(&settings, &device, &host_mem).expect("Could not bind buffer");
    let uniform_access = BufferSegment::new(&uniform, 10, None).unwrap();
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

    let mut settings = pipelines::layout::Settings::new(None);
    settings.add_layout(&dlayout);
    let playout = pipelines::Layout::new(&device, &settings);

    let compute = pipelines::Compute::new(&device, &shader, &playout, None);

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