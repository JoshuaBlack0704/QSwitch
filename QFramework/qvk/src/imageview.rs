use crate::{device::DeviceProvider, image, ImageView};

pub trait ImageViewProvider{
    
}

impl<D:DeviceProvider, I:image::ImageProvider> ImageViewProvider for ImageView<D,I>{
    
}