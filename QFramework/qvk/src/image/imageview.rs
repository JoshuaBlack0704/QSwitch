use crate::init::device::DeviceStore;

use super::{ImageView, image::ImageStore};

pub trait ImageViewStore{
    
}

impl<D:DeviceStore, I:ImageStore> ImageViewStore for ImageView<D,I>{
    
}