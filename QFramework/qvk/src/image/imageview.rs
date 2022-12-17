use crate::{device::DeviceProvider, image::ImageProvider};

use super::ImageView;

pub trait ImageViewProvider{
    
}

impl<D:DeviceProvider, I:ImageProvider> ImageViewProvider for ImageView<D,I>{
    
}