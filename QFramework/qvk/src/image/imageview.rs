use crate::image::{ImageStore, ImageViewStore};
use crate::init::DeviceStore;

use super::ImageView;

impl<D:DeviceStore, I:ImageStore> ImageViewStore for ImageView<D,I>{
    
}