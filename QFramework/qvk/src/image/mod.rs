use std::{sync::{Arc, Mutex}, marker::PhantomData};

use ash::vk;

use crate::{init::{device::{DeviceStore, UsesDeviceStore}, instance::{InstanceStore, UsesInstanceStore}}, memory::{memory::MemoryStore, Partition}};

use self::image::ImageStore;

pub mod image;
pub struct Image<D:DeviceStore, M:MemoryStore>{
    device: Arc<D>,
    memory: Option<Arc<M>>,
    _partition: Option<Partition>,
    image: vk::Image,
    create_info: vk::ImageCreateInfo,
    current_layout: Arc<Mutex<vk::ImageLayout>>,
}


pub mod imageresource;
pub struct ImageResource<I:InstanceStore, D:DeviceStore + UsesInstanceStore<I>, Img:ImageStore + UsesDeviceStore<D>>{
    image: Arc<Img>,
    resorces: vk::ImageSubresourceLayers,
    offset: vk::Offset3D,
    extent: vk::Extent3D,
    layout: Arc<Mutex<vk::ImageLayout>>,
    _device: PhantomData<D>,
    _instance: PhantomData<I>
}

pub mod imageview;
pub struct ImageView<D:DeviceStore, Img:ImageStore>{
    _device: Arc<D>,
    _image: Arc<Img>,
    _view: vk::ImageView,
}
