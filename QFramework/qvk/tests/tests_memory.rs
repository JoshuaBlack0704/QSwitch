use std::mem::size_of;

use ash::vk;
use qvk::{init::{instance, device, DeviceStore, InstanceFactory, DeviceFactory}, memory::{memory, Memory, buffer::{buffer, Buffer, BufferSegment, BufferSegmentStore}}, image::{image,Image, ImageResource, ImageStore, ImageSubresourceStore}};

#[test]
fn buffer_image(){
    
    
    let settings = instance::Settings::default();
    let instance = settings.create_instance();
    
    let settings = device::Settings::new_simple(instance.clone());
    let device = settings.create_device().expect("Could not create device");
    
    let image_extent = vk::Extent3D::builder().width(100).height(100).depth(1).build();

    let settings = memory::SettingsStore::new(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2 * 3, device.host_memory_index());
    let host_mem = Memory::new(&settings, &device).expect("Could not allocate memory");

    let settings = memory::SettingsStore::new(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2, device.device_memory_index());
    let dev_mem = Memory::new(&settings, &device).expect("Could not allocate memory");
    
    let image_settings = image::SettingsStore::new_simple(vk::Format::R8G8B8A8_SRGB, image_extent, vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, Some(vk::ImageLayout::TRANSFER_DST_OPTIMAL));    
    let image = Image::new(&device, &dev_mem, &image_settings).unwrap();
    let resource = ImageResource::new(&image, vk::ImageAspectFlags::COLOR, 0, 0, 1, vk::Offset3D::default(), image_extent).unwrap();
    
    let settings = buffer::SettingsStore::new(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2 * 3, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST);
    let s1 = Buffer::new(&settings, &device, &host_mem).expect("Could not bind buffer");
    let src = BufferSegment::new(&s1, size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64, None).unwrap();
    let dst = BufferSegment::new(&s1, size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64, None).unwrap();

    let data = vec![0x0000ffff; (image_extent.width * image_extent.height) as usize];
    let mut res = vec![0u32; (image_extent.width * image_extent.height) as usize];

    src.copy_from_ram(&data).unwrap();
    src.copy_to_image_internal(&resource, None).unwrap();
    image.internal_transistion(vk::ImageLayout::TRANSFER_SRC_OPTIMAL, None);
    resource.copy_to_buffer_internal(&dst, None).unwrap();
    dst.copy_to_ram(&mut res).unwrap();

    println!("{:?}", res);
    
    
    assert_eq!(res, data);
}

#[test]
fn buffer_ram(){
    
    let settings = instance::Settings::default();
    let instance = settings.create_instance();
    
    let settings = device::Settings::new_simple(instance.clone());
    let device = settings.create_device().expect("Could not create device");

    let settings = memory::SettingsStore::new(1024, device.host_memory_index());
    let host_mem = Memory::new(&settings, &device).expect("Could not allocate memory");

    let settings = buffer::SettingsStore::new(1024, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST);
    let storage = Buffer::new(&settings, &device, &host_mem).expect("Could not bind buffer");
    let storge_access = BufferSegment::new(&storage, 200, None).unwrap();

    let data = [20u8; 200];
    let mut dst = [0u8; 200];

    storge_access.copy_from_ram(&data).unwrap();
    storge_access.copy_to_ram(&mut dst).unwrap();

    println!("{:?}", dst);
    
    
    assert_eq!(dst, data);
}

#[test]
fn buffer_buffer(){
    
    let settings = instance::Settings::default();
    let instance = settings.create_instance();
    
    let settings = device::Settings::new_simple(instance.clone());
    let device = settings.create_device().expect("Could not create device");

    let settings = memory::SettingsStore::new(1024, device.host_memory_index());
    let host_mem = Memory::new(&settings, &device).expect("Could not allocate memory");
    let settings = memory::SettingsStore::new(1024, device.device_memory_index());
    let dev_mem = Memory::new(&settings, &device).expect("Could not allocate memory");

    let settings = buffer::SettingsStore::new(200, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST);
    let s1 = Buffer::new(&settings, &device, &host_mem).expect("Could not bind buffer");
    let src = BufferSegment::new(&s1, 200, None).unwrap();
    let s2 = Buffer::new(&settings, &device, &dev_mem).expect("Could not bind buffer");
    let dst = BufferSegment::new(&s2, 200, None).unwrap();
    let s3 = Buffer::new(&settings, &device, &host_mem).expect("Could not bind buffer");
    let fin = BufferSegment::new(&s3, 200, None).unwrap();

    let data = [20u8; 200];
    let mut res = [0u8; 200];

    src.copy_from_ram(&data).unwrap();
    src.copy_to_segment_internal(&dst).unwrap();
    dst.copy_to_segment_internal(&fin).unwrap();
    fin.copy_to_ram(&mut res).unwrap();

    println!("{:?}", res);
    
    
    assert_eq!(res, data);
}

#[test]
fn image_image(){
    
    
    let settings = instance::Settings::default();
    let instance = settings.create_instance();
    
    let settings = device::Settings::new_simple(instance.clone());
    let device = settings.create_device().expect("Could not create device");
    
    let image_extent = vk::Extent3D::builder().width(100).height(100).depth(1).build();

    let settings = memory::SettingsStore::new(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2 * 3, device.host_memory_index());
    let host_mem = Memory::new(&settings, &device).expect("Could not allocate memory");

    let settings = memory::SettingsStore::new(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2, device.device_memory_index());
    let dev_mem = Memory::new(&settings, &device).expect("Could not allocate memory");
    
    let image_settings = image::SettingsStore::new_simple(vk::Format::R8G8B8A8_SRGB, image_extent, vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, Some(vk::ImageLayout::TRANSFER_DST_OPTIMAL));    
    let image1 = Image::new(&device, &dev_mem, &image_settings).unwrap();
    let resource1 = ImageResource::new(&image1, vk::ImageAspectFlags::COLOR, 0, 0, 1, vk::Offset3D::default(), image_extent).unwrap();
    
    let settings = memory::SettingsStore::new(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2, device.device_memory_index());
    let dev_mem = Memory::new(&settings, &device).expect("Could not allocate memory");
    
    let image_settings = image::SettingsStore::new_simple(vk::Format::R8G8B8A8_SRGB, image_extent, vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, Some(vk::ImageLayout::TRANSFER_DST_OPTIMAL));    
    let image2 = Image::new(&device, &dev_mem, &image_settings).unwrap();
    let resource2 = ImageResource::new(&image2, vk::ImageAspectFlags::COLOR, 0, 0, 1, vk::Offset3D::default(), image_extent).unwrap();
    
    let settings = buffer::SettingsStore::new(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2 * 3, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST);
    let s1 = Buffer::new(&settings, &device, &host_mem).expect("Could not bind buffer");
    let src = BufferSegment::new(&s1, size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64, None).unwrap();
    let dst = BufferSegment::new(&s1, size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64, None).unwrap();

    let data = vec![0x0000ffff; (image_extent.width * image_extent.height) as usize];
    let mut res = vec![0u32; (image_extent.width * image_extent.height) as usize];

    src.copy_from_ram(&data).unwrap();
    src.copy_to_image_internal(&resource1, None).unwrap();
    image1.internal_transistion(vk::ImageLayout::TRANSFER_SRC_OPTIMAL, None);
    resource1.copy_to_image_internal(&resource2).unwrap();
    image2.internal_transistion(vk::ImageLayout::TRANSFER_SRC_OPTIMAL, None);
    resource2.copy_to_buffer_internal(&dst, None).unwrap();
    dst.copy_to_ram(&mut res).unwrap();

    println!("{:?}", res);
    
    
    assert_eq!(res, data);
}