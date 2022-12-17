use std::sync::{Arc, Mutex};

use ash::vk;

use crate::{device::DeviceProvider, memory::{memory::MemoryProvider, Partition}};

use self::image::ImageProvider;

pub mod image;
pub struct Image<D:DeviceProvider, M:MemoryProvider>{
    device: Arc<D>,
    memory: Option<Arc<M>>,
    _partition: Option<Partition>,
    image: vk::Image,
    create_info: vk::ImageCreateInfo,
    current_layout: Mutex<vk::ImageLayout>,
}


pub mod imageresource;

pub mod imageview;
pub struct ImageView<D:DeviceProvider, Img:ImageProvider>{
    _device: Arc<D>,
    _image: Arc<Img>,
    _view: vk::ImageView,
}
