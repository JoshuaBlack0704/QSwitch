use std::{ffi::{CStr, CString}, sync::Arc};

use ash::vk;
use log::{debug, info};
use raw_window_handle::RawDisplayHandle;

use crate::Instance;

pub trait InstanceSettingsProvider{
    fn app_info(&self) -> vk::ApplicationInfo;
    fn use_validation_layers(&self) -> bool;
    fn use_debug(&self) -> bool;
    fn use_window_extensions(&self) -> Option<Vec<*const i8>>;
    fn validation_enables(&self) -> Option<&[vk::ValidationFeatureEnableEXT]>;
    fn validation_disables(&self) -> Option<&[vk::ValidationFeatureDisableEXT]>;
}

pub trait InstanceProvider{
    fn instance(&self) -> &ash::Instance;
    fn entry(&self) -> &ash::Entry;
}

pub struct SettingsProvider{
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
}

impl Instance{
    pub fn new<S: InstanceSettingsProvider>(settings: &S) -> Arc<Instance> {
        // The beginning of our new vulkan system
        let entry = ash::Entry::linked();
        let app_info = settings.app_info();
        
        let mut validation_features = vk::ValidationFeaturesEXT::builder();
        let mut layer_names = vec![];
        let mut extension_names = vec![]; 
        
        if settings.use_validation_layers(){
            let name = unsafe{CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0")};
            layer_names.push(name.as_ptr());
            info!("Validation layers requested");
            
            if let Some(enables) = settings.validation_enables(){
                debug!("Non standard validation enableds requested");
                validation_features = validation_features.enabled_validation_features(enables);
            }
            if let Some(disables) = settings.validation_disables(){
                debug!("Non standard validation disables requested");
                validation_features = validation_features.disabled_validation_features(disables);
            }
        }
        
        if settings.use_debug(){
            extension_names.push(ash::extensions::ext::DebugUtils::name().as_ptr());
            debug!("Debug system requested");
        }
        
        if let Some(names) = settings.use_window_extensions(){
            extension_names.extend_from_slice(&names);
            debug!("Window extensions requested");
        }
        
        let mut inst_cinfo = vk::InstanceCreateInfo::builder();
        inst_cinfo = inst_cinfo.application_info(&app_info);
        inst_cinfo = inst_cinfo.enabled_extension_names(&extension_names);
        inst_cinfo = inst_cinfo.enabled_layer_names(&layer_names);
        inst_cinfo = inst_cinfo.push_next(&mut validation_features);
        
        let instance = unsafe{entry.create_instance(&inst_cinfo, None)}.expect("Could not create instance");
        info!("Created instance {:?}", instance.handle());
        
        Arc::new(Instance{ entry, instance })
        
    }
}

impl InstanceProvider for Instance{
    fn instance(&self) -> &ash::Instance {
        &self.instance
    }

    fn entry(&self) -> &ash::Entry {
        &self.entry
    }
}

impl Drop for Instance{
    fn drop(&mut self) {
        debug!("Destroyed instance {:?}", self.instance.handle());
        unsafe{
            self.instance.destroy_instance(None);
        }
    }
}

impl SettingsProvider{
        pub fn new(
        app_name: CString,
        engine_name: CString,
        app_version: u32,
        engine_version: u32,
        api_version: u32,
        use_validation: bool,
        validation_enables: Option<Vec<vk::ValidationFeatureEnableEXT>>,
        validation_disables: Option<Vec<vk::ValidationFeatureDisableEXT>>,
        use_debug: bool,
    ) -> SettingsProvider {
        
        SettingsProvider{ 
            app_name,
            engine_name,
            app_version,
            engine_version,
            api_version,
            use_validation,
            validation_enables,
            validation_disables,
            use_debug,
            window_extensions: None, }
        
    }
    
    pub fn use_window_extensions(&mut self, display: RawDisplayHandle){
        self.window_extensions = Some(ash_window::enumerate_required_extensions(display).unwrap().to_vec());
    }
}

#[cfg(debug_assertions)]
fn validate() -> bool {
    true
}

#[cfg(not(debug_assertions))]
fn validate() -> bool{
    false
}

impl Default for SettingsProvider{
    fn default() -> Self {
        
        Self::new(
            CString::new("App").unwrap(), 
            CString::new("Engine").unwrap(),
            0, 0, vk::API_VERSION_1_3,
            validate(), None, None, false
        )
    }
}

impl InstanceSettingsProvider for SettingsProvider{
    fn app_info(&self) -> vk::ApplicationInfo {
        vk::ApplicationInfo::builder()
        .api_version(self.api_version)
        .application_name(&self.app_name)
        .engine_name(&self.engine_name)
        .application_version(self.app_version)
        .engine_version(self.engine_version)
        .build()
    }

    fn use_validation_layers(&self) -> bool {
        self.use_validation
    }

    fn use_debug(&self) -> bool {
        self.use_debug
    }

    fn use_window_extensions(&self) -> Option<Vec<*const i8>> {
        self.window_extensions.clone()
    }

    fn validation_enables(&self) -> Option<&[vk::ValidationFeatureEnableEXT]> {
        if let Some(enables) = &self.validation_enables{
            return Some(enables);
        }
        None
    }

    fn validation_disables(&self) -> Option<&[vk::ValidationFeatureDisableEXT]> {
        if let Some(disables) = &self.validation_disables{
            return Some(disables);
        }
        None
    }
}