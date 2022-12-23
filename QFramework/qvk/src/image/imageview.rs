use std::sync::Arc;

use crate::image::{ImageStore, ImageViewStore};
use crate::init::DeviceSource;

use super::ImageView;

impl<D:DeviceSource, I:ImageStore> ImageViewStore for Arc<ImageView<D,I>>{
    
}