use std::sync::Arc;

use ash::vk;

use crate::device::DeviceProvider;

use self::layout::PipelineLayoutProvider;

pub mod layout;
pub struct Layout<D:DeviceProvider>{
    device: Arc<D>,
    layout: vk::PipelineLayout,
}

pub mod compute;
pub struct Compute<D:DeviceProvider, L:PipelineLayoutProvider>{
    device: Arc<D>,
    layout: Arc<L>,
    pipeline: vk::Pipeline,
}

pub mod graphics;

pub mod raytracing;