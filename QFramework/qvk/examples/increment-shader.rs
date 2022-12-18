use ash::vk;
use qvk::{instance, Instance, device::{self, DeviceProvider}, Device, memory::{self, Memory, buffer::{Buffer, BufferPartition, buffer, bufferpartition}}, descriptor::{DescriptorLayout, descriptorlayout::DescriptorLayoutProvider, Set, self}, shader::HLSL};


fn main(){
    
    pretty_env_logger::init();
    
    let mut settings = instance::SettingsProvider::default();
    let instance = Instance::new(&settings);
    
    let mut settings = device::SettingsProvider::default();
    settings.add_extension(ash::extensions::khr::BufferDeviceAddress::name().as_ptr());
    let device = Device::new(&settings, &instance).expect("Could not create device");

    let mut settings = memory::memory::SettingsProvider::new(1024 * 1024 * 100, device.host_memory_index());
    let host_mem = Memory::new(&settings, &device).expect("Could not allocate memory");

    let settings = buffer::SettingsProvider::new(1024 * 1024 * 100, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST);
    let buf = Buffer::new(&settings, &device, &host_mem).expect("Could not bind buffer");
    let part = BufferPartition::new(&buf, 100, None).unwrap();

    let dlayout = DescriptorLayout::new(&device, None);
    DescriptorLayout::form_binding(&dlayout, &part, vk::ShaderStageFlags::COMPUTE);
    dlayout.layout();

    let pool_layouts = [(&dlayout, 1)];
    let dpool = descriptor::Pool::new(&device, &pool_layouts, None);

    let dset = Set::new(&device, &dlayout, &dpool);

    let code = HLSL::new("examples/resources/shaders/increment-set.hlsl", shaderc::ShaderKind::Compute, None);
    let shader = qvk::shader::Shader::new(&device, &code, vk::ShaderStageFlags::COMPUTE, None);
    
}