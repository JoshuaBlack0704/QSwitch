use std::mem::size_of;

use ash::vk;
use qvk::{
    init::{device, instance, DeviceFactory, InstanceFactory},
    memory::{
        allocators::{BufferAllocatorFactory, ImageAllocatorFactory, MemoryAllocatorFactory},
        buffer::{BufferSegmentFactory, BufferSegmentSource},
        image::{ImageFactory, ImageResourceFactory, ImageResourceSource, ImageSource},
    },
};

#[test]
fn buffer_image() {
    let settings = instance::Settings::default();
    let instance = settings.create_instance();

    let settings = device::Settings::new_simple(instance.clone());
    let device = settings.create_device().expect("Could not create device");

    let image_extent = vk::Extent3D::builder()
        .width(100)
        .height(100)
        .depth(1)
        .build();

    let host_mem = device.create_cpu_mem(1024 * 1024 * 5);
    let dev_mem = device.create_gpu_mem(1024 * 1024 * 5);
    let image_alloc = dev_mem.create_image_allocator_simple(
        vk::Format::B8G8R8A8_SRGB,
        vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST,
    );
    let image = image_alloc.create_image(image_extent);
    image.internal_transistion(vk::ImageLayout::TRANSFER_DST_OPTIMAL);
    let resource = image
        .create_resource(
            vk::Offset3D::default(),
            image_extent,
            0,
            vk::ImageAspectFlags::COLOR,
        )
        .unwrap();

    let s1 = host_mem.create_storage_buffer(
        1024 * 1024,
        Some(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST),
    );
    let src = s1.get_segment(
        size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64,
        None,
    );
    let dst = s1.get_segment(
        size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64,
        None,
    );

    let data = vec![0x0000ffff; (image_extent.width * image_extent.height) as usize];
    let mut res = vec![0u32; (image_extent.width * image_extent.height) as usize];

    src.copy_from_ram(&data).unwrap();
    src.copy_to_image_internal(&resource, None).unwrap();
    image.internal_transistion(vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
    resource.copy_to_buffer_internal(&dst, None).unwrap();
    dst.copy_to_ram(&mut res).unwrap();

    println!("{:?}", res);

    assert_eq!(res, data);
}

#[test]
fn buffer_ram() {
    let settings = instance::Settings::default();
    let instance = settings.create_instance();

    let settings = device::Settings::new_simple(instance.clone());
    let device = settings.create_device().expect("Could not create device");

    let host_mem = device.create_cpu_mem(1024);

    let storage = host_mem.create_storage_buffer(
        1024,
        Some(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST),
    );
    let storge_access = storage.get_segment(200, None);

    let data = [20u8; 200];
    let mut dst = [0u8; 200];

    storge_access.copy_from_ram(&data).unwrap();
    storge_access.copy_to_ram(&mut dst).unwrap();

    println!("{:?}", dst);

    assert_eq!(dst, data);
}

#[test]
fn buffer_buffer() {
    let settings = instance::Settings::default();
    let instance = settings.create_instance();

    let settings = device::Settings::new_simple(instance.clone());
    let device = settings.create_device().expect("Could not create device");

    let host_mem = device.create_cpu_mem(1024);
    let dev_mem = device.create_gpu_mem(1024);

    let s1 = host_mem.create_storage_buffer(
        200,
        Some(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST),
    );
    let src = s1.get_segment(200, None);
    let s2 = dev_mem.create_storage_buffer(
        200,
        Some(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST),
    );
    let dst = s2.get_segment(200, None);
    let s3 = host_mem.create_storage_buffer(
        200,
        Some(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST),
    );
    let fin = s3.get_segment(200, None);

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
fn image_image() {
    let settings = instance::Settings::default();
    let instance = settings.create_instance();

    let settings = device::Settings::new_simple(instance.clone());
    let device = settings.create_device().expect("Could not create device");

    let image_extent = vk::Extent3D::builder()
        .width(100)
        .height(100)
        .depth(1)
        .build();

    let host_mem = device.create_cpu_mem(1024 * 1024 * 5);
    let dev_mem = device.create_gpu_mem(1024 * 1024 * 5);
    let image_alloc = dev_mem.create_image_allocator_simple(
        vk::Format::B8G8R8A8_SRGB,
        vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST,
    );
    let image1 = image_alloc.create_image(image_extent);
    image1.internal_transistion(vk::ImageLayout::TRANSFER_DST_OPTIMAL);
    let resource1 = image1
        .create_resource(
            vk::Offset3D::default(),
            image_extent,
            0,
            vk::ImageAspectFlags::COLOR,
        )
        .unwrap();

    let image2 = image_alloc.create_image(image_extent);
    image2.internal_transistion(vk::ImageLayout::TRANSFER_DST_OPTIMAL);
    let resource2 = image2
        .create_resource(
            vk::Offset3D::default(),
            image_extent,
            0,
            vk::ImageAspectFlags::COLOR,
        )
        .unwrap();

    let s1 = host_mem.create_storage_buffer(
        1024 * 1024,
        Some(vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST),
    );
    let src = s1.get_segment(
        size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64,
        None,
    );
    let dst = s1.get_segment(
        size_of::<u32>() as u64 * image_extent.width as u64 * image_extent.height as u64,
        None,
    );

    let data = vec![0x0000ffff; (image_extent.width * image_extent.height) as usize];
    let mut res = vec![0u32; (image_extent.width * image_extent.height) as usize];

    src.copy_from_ram(&data).unwrap();
    src.copy_to_image_internal(&resource1, None).unwrap();
    image1.internal_transistion(vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
    resource1.copy_to_image_internal(&resource2).unwrap();
    image2.internal_transistion(vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
    resource2.copy_to_buffer_internal(&dst, None).unwrap();
    dst.copy_to_ram(&mut res).unwrap();

    println!("{:?}", res);

    assert_eq!(res, data);
}
