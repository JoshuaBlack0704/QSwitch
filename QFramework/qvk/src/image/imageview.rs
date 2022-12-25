use std::sync::Arc;

use crate::image::{ImageSource, ImageViewSource};
use crate::init::DeviceSource;

use super::ImageView;

impl<D:DeviceSource, I:ImageSource> ImageViewSource for Arc<ImageView<D,I>>{
    
}