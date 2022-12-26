use std::sync::Arc;

use ash::vk;

use crate::image::{ImageSource, ImageViewSource};
use crate::init::{DeviceSource, DeviceSupplier};

use super::{ImageView, ImageSupplier, ImageViewFactory, ImageResourceSource, ImageResourceSupplier};

impl<D:DeviceSource + Clone, Img: ImageSource, IR:ImageResourceSource + Clone, Factory: DeviceSupplier<D> + ImageSupplier<Img> + ImageResourceSupplier<IR>> ImageViewFactory<Arc<ImageView<D,Img, IR>>> for Factory{
    fn create_image_view(&self, format: vk::Format, view_type: vk::ImageViewType, swizzle: Option<vk::ComponentMapping>, flags: Option<vk::ImageViewCreateFlags>) -> Arc<ImageView<D,Img, IR>> {
        let components;
        if let Some(c) = swizzle{
            components = c;
        }
        else{
            components = vk::ComponentMapping::builder()
                .r(vk::ComponentSwizzle::R)
                .g(vk::ComponentSwizzle::G)
                .b(vk::ComponentSwizzle::B)
                .a(vk::ComponentSwizzle::A)
            .build();
        }

        let mut info = vk::ImageViewCreateInfo::builder();
        if let Some(flags) = flags{
            info = info.flags(flags);
        }

        let range = vk::ImageSubresourceRange::builder()
        .aspect_mask(self.image_resource().aspect())
        .base_mip_level(self.image_resource().level())
        .base_array_layer(0)
        .level_count(1)
        .level_count(1);

        
        info = info
        .image(*self.image_provider().image())
        .view_type(view_type)
        .format(format)
        .components(components)
        .subresource_range(range.build());

        let view;
        unsafe{
            view = self.device_provider().device().create_image_view(&info, None).unwrap();
        }

        Arc::new(
            ImageView{
                _device: self.device_provider().clone(),
                _image_resource: self.image_resource().clone(),
                _image: std::marker::PhantomData,
                _view: view,
            }
        )
    }
}

impl<D:DeviceSource, I:ImageSource, IR:ImageResourceSource> ImageViewSource for Arc<ImageView<D,I,IR>>{
    
}