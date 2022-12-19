use crate::{device::DeviceStore, image::ImageStore};

use super::ImageView;

pub trait ImageViewStore{
    
}

impl<D:DeviceStore, I:ImageStore> ImageViewStore for ImageView<D,I>{
    
}