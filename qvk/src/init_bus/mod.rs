use std::ffi::CString;

use ash::vk;

pub mod instance;
pub enum InstanceExtension{
    
}
pub trait InstanceSource{
    fn get_instance(&self) -> &ash::Instance;
    
}
pub struct InstanceBuilder{
    pub app_name: CString,
    pub engine_name: CString,
    pub app_version: u32,
    pub engine_version: u32,
    pub api_version: u32,
    pub use_validation: bool,
    pub validation_enables: Option<Vec<vk::ValidationFeatureEnableEXT>>,
    pub validation_disables: Option<Vec<vk::ValidationFeatureDisableEXT>>,
    pub use_debug: bool,
    pub window_extensions: Option<Vec<*const i8>>,
    pub instance_extensions: Vec<InstanceExtension>,
}
pub struct Instance{
    entry: ash::Entry,
    instance: ash::Instance,
}