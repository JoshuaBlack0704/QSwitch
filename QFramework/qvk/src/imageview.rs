use crate::{device::DeviceProvider, image::ImageProvider, ImageView};

pub trait ImageViewProvider{
    
}

impl<D:DeviceProvider, I:ImageProvider> ImageViewProvider for ImageView<D,I>{
    
}