use std::mem::size_of;

use ash::vk;
use qvk::{init::{instance, device, DeviceSource, InstanceFactory, DeviceFactory}, memory::{buffer::{BufferSegmentSource, BufferFactory, BufferSegmentFactory}, MemoryFactory}, image::{ImageSource, ImageResourceSource, ImageFactory, ImageResourceFactory}};

#[test]
fn buffer_image(){
    
    
    let settings = instance::Settings::default();
    let instance = settings.create_instance();
    
    let settings = device::Settings::new_simple(instance.clone());
    let device = settings.create_device().expect("Could not create device");
    
    let image_extent = vk::Extent3D::builder().width(100).height(100).depth(1).build();

    let host_mem = device.create_memory(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2 * 3, device.host_memory_index(), None).unwrap();

    let dev_mem = device.create_memory(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2, device.device_memory_index(), None).unwrap();
    
    let image = dev_mem.create_image(vk::Format::R8G8B8A8_SRGB, image_extent, 1, 1, vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, None).unwrap();
    image.internal_transistion(vk::ImageLayout::TRANSFER_DST_OPTIMAL, None);
    let resource = image.create_resource(vk::Offset3D::default(), image_extent, 0, vk::ImageAspectFlags::COLOR).unwrap();
    
    let s1 = host_mem.create_buffer(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2 * 3, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, None, None).unwrap();
    let src = s1.create_segment(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64, None).unwrap();
    let dst = s1.create_segment(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64, None).unwrap();

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

    let host_mem = device.create_memory(1024, device.host_memory_index(), None).unwrap();

    let storage = host_mem.create_buffer(1024, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, None, None).unwrap();
    let storge_access = storage.create_segment(200, None).unwrap();

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

    let host_mem = device.create_memory(1024, device.host_memory_index(), None).unwrap();
    let dev_mem = device.create_memory(1024, device.device_memory_index(), None).unwrap();

    let s1 = host_mem.create_buffer(200, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, None, None).unwrap();
    let src = s1.create_segment(200, None).unwrap();
    let s2 = dev_mem.create_buffer(200, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, None, None).unwrap();
    let dst = s2.create_segment(200, None).unwrap();
    let s3 = host_mem.create_buffer(200, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, None, None).unwrap();
    let fin = s3.create_segment(200, None).unwrap();

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

    let host_mem = device.create_memory(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2 * 3, device.host_memory_index(), None).unwrap();

    let dev_mem = device.create_memory(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2, device.device_memory_index(), None).unwrap();
    
    let image1 = dev_mem.create_image(vk::Format::R8G8B8A8_SRGB, image_extent, 1, 1, vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, None).unwrap();
    image1.internal_transistion(vk::ImageLayout::TRANSFER_DST_OPTIMAL, None);
    let resource1 = image1.create_resource(vk::Offset3D::default(), image_extent, 0, vk::ImageAspectFlags::COLOR).unwrap();
    
    let dev_mem = device.create_memory(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2, device.device_memory_index(), None).unwrap();
    
    let image2 = dev_mem.create_image(vk::Format::R8G8B8A8_SRGB, image_extent, 1, 1, vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST, None).unwrap();
    image2.internal_transistion(vk::ImageLayout::TRANSFER_DST_OPTIMAL, None);
    let resource2 = image2.create_resource(vk::Offset3D::default(), image_extent, 0, vk::ImageAspectFlags::COLOR).unwrap();
    
    let s1 = host_mem.create_buffer(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64 * 2 * 3, vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST, None, None).unwrap();
    let src = s1.create_segment(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64, None).unwrap();
    let dst = s1.create_segment(size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64, None).unwrap();

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