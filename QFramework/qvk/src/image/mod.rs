use std::{sync::{Arc, Mutex}, marker::PhantomData};

use ash::vk;

use crate::{device::{DeviceProvider, UsesDeviceProvider}, memory::{memory::MemoryProvider, Partition}, instance::{InstanceProvider, UsesInstanceProvider}};

use self::image::ImageProvider;

pub mod image;
pub struct Image<D:DeviceProvider, M:MemoryProvider>{
    device: Arc<D>,
    memory: Option<Arc<M>>,
    _partition: Option<Partition>,
    image: vk::Image,
    create_info: vk::ImageCreateInfo,
    current_layout: Arc<Mutex<vk::ImageLayout>>,
}


pub mod imageresource;
pub struct ImageResource<I:InstanceProvider, D:DeviceProvider + UsesInstanceProvider<I>, Img:ImageProvider + UsesDeviceProvider<D>>{
    image: Arc<Img>,
    resorces: vk::ImageSubresourceLayers,
    offset: vk::Offset3D,
    extent: vk::Extent3D,
    layout: Arc<Mutex<vk::ImageLayout>>,
    _device: PhantomData<D>,
    _instance: PhantomData<I>
}

pub mod imageview;
pub struct ImageView<D:DeviceProvider, Img:ImageProvider>{
    _device: Arc<D>,
    _image: Arc<Img>,
    _view: vk::ImageView,
}
