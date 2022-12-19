use ash::vk;
use qvk::{instance, Instance, device::{self, DeviceStore}, Device, memory::{self, Memory, buffer::{Buffer, BufferSegment, buffer, buffersegment}}, descriptor::{DescriptorLayout, descriptorlayout::DescriptorLayoutStore, Set, self}, shader::HLSL, pipelines};


fn main(){
    
    pretty_env_logger::init();
    
    let mut settings = instance::Settings::default();
    let instance = Instance::new(&settings);
    
    let mut settings = device::Settings::default();
    settings.add_extension(ash::extensions::khr::BufferDeviceAddress::name().as_ptr());
    let device = Device::new(&settings, &instance).expect("Could not create device");

    let mut settings = memory::memory::SettingsStore::new(1024 * 1024 * 100, device.host_memory_index());
    let host_mem = Memory::new(&settings, &device).expect("Could not allocate memory");

    let settings = buffer::SettingsStore::new(1024 * 1024 * 50, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST);
    let storage = Buffer::new(&settings, &device, &host_mem).expect("Could not bind buffer");
    let starge_access = BufferSegment::new(&storage, 100, None).unwrap();
    let settings = buffer::SettingsStore::new(1024 * 1024, vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST);
    let uniform = Buffer::new(&settings, &device, &host_mem).expect("Could not bind buffer");
    let uniform_access = BufferSegment::new(&uniform, 100, None).unwrap();

    let dlayout = DescriptorLayout::new(&device, None);
    let storage_write = DescriptorLayout::form_binding(&dlayout, &starge_access, vk::ShaderStageFlags::COMPUTE);
    let uniform_write = DescriptorLayout::form_binding(&dlayout, &uniform_access, vk::ShaderStageFlags::COMPUTE);
    dlayout.layout();

    let pool_layouts = [(&dlayout, 1)];
    let dpool = descriptor::Pool::new(&device, &pool_layouts, None);

    let dset = Set::new(&device, &dlayout, &dpool);

    let code = HLSL::new("examples/resources/shaders/increment-set.hlsl", shaderc::ShaderKind::Compute, "main", None);
    let shader = qvk::shader::Shader::new(&device, &code, vk::ShaderStageFlags::COMPUTE, None);

    let mut settings = pipelines::layout::Settings::new(None);
    settings.add_layout(&dlayout);
    let playout = pipelines::Layout::new(&device, &settings);

    let compute = pipelines::Compute::new(&device, &shader, &playout, None);
    
}