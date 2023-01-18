use std::{ffi::{CString, CStr}, sync::Arc};

use ash::vk::{self, InstanceCreateInfoBuilder};
use log::{info, debug};
use qcom::bus::{Bus, BusElement, BusTransaction};
use raw_window_handle::RawDisplayHandle;

use crate::bus::{QvkBus, QvkBusMessage};

use super::{InstanceBuilder, InstanceExtension, Instance, InstanceSource};

impl InstanceBuilder{
    pub fn app_name(mut self, name: CString) -> InstanceBuilder {
        self.app_name = name;
        self
    }
    pub fn engine_name(mut self, name: CString) -> InstanceBuilder {
        self.engine_name = name;
        self
    }
    pub fn app_version(mut self, version: u32) -> InstanceBuilder {
        self.app_version = version;
        self
    }
    pub fn engine_version(mut self, version: u32) -> InstanceBuilder {
        self.engine_version = version;
        self
    }
    pub fn api_version(mut self, version: u32) -> InstanceBuilder {
        self.api_version = version;
        self
    }
    pub fn use_validation(mut self, validation: bool) -> InstanceBuilder {
        self.use_validation = validation;
        self
    }
    pub fn add_validation_enable(mut self, enable: vk::ValidationFeatureEnableEXT) -> InstanceBuilder {
        self.validation_enables.get_or_insert(vec![]).push(enable);
        self
    }
    pub fn add_validation_disable(mut self, disable: vk::ValidationFeatureDisableEXT) -> InstanceBuilder {
        self.validation_disables.get_or_insert(vec![]).push(disable);
        self
    }
    pub fn use_debug(mut self, debug: bool) -> InstanceBuilder {
        self.use_debug = debug;
        self
    }
    pub fn window_extensions(mut self, display: RawDisplayHandle) -> InstanceBuilder {
        self.window_extensions = Some(ash_window::enumerate_required_extensions(display).unwrap().to_vec());
        self
    }
    pub fn add_extension(&mut self, ext: InstanceExtension){
        self.instance_extensions.push(ext);
    }
    fn app_info(&self) -> vk::ApplicationInfo {
        vk::ApplicationInfo::builder()
            .api_version(self.api_version)
            .application_name(&self.app_name)
            .engine_name(&self.engine_name)
            .application_version(self.app_version)
            .engine_version(self.engine_version)
            .build()
    }
    pub fn build(mut self, qvk_bus: &Arc<QvkBus>) {
        let entry = ash::Entry::linked();
        let app_info = self.app_info();
        
        let mut validation_features = vk::ValidationFeaturesEXT::builder();
        let mut layer_names = vec![];
        let mut extension_names = vec![];

        if self.use_validation {
            let name =
                unsafe { CStr::from_bytes_with_nul_unchecked(b"VK_LAYER_KHRONOS_validation\0") };
            layer_names.push(name.as_ptr());
            info!("Validation layers requested");

            if let Some(enables) = &self.validation_enables {
                debug!("Non standard validation enableds requested");
                validation_features = validation_features.enabled_validation_features(enables);
            }
            if let Some(disables) = &self.validation_disables {
                debug!("Non standard validation disables requested");
                validation_features = validation_features.disabled_validation_features(disables);
            }
        }

        if self.use_debug {
            extension_names.push(ash::extensions::ext::DebugUtils::name().as_ptr());
            debug!("Debug system requested");
        }

        if let Some(names) = self.window_extensions{
            extension_names.extend_from_slice(&names);
            debug!("Window extensions requested");
        }

        let mut info = vk::InstanceCreateInfo::builder();
        info = info.application_info(&app_info);
        info = info.enabled_extension_names(&extension_names);
        info = info.enabled_layer_names(&layer_names);
        for ext in self.instance_extensions.iter_mut(){
            info = ext.push(info);
            
        }
        info = info.push_next(&mut validation_features);

        let instance =
            unsafe { entry.create_instance(&info, None) }.expect("Could not create instance");
        let _ = qvk_bus.broadcast(crate::bus::QvkBusMessage::InstanceHandle(instance.handle()));

        let instance = Arc::new(Instance { entry, instance });
        qvk_bus.bind_instance(Arc::new(instance));
    }
    
}
impl InstanceExtension{
    fn push(&mut self, mut _builder: InstanceCreateInfoBuilder) -> InstanceCreateInfoBuilder {
        todo!()
    }
}

#[cfg(debug_assertions)]
fn validate() -> bool{
    true
}
#[cfg(not(debug_assertions))]
fn validate() -> bool{
    false
}

impl Default for InstanceBuilder{
    fn default() -> Self {
        
        Self{
            app_name: CString::new("Default").unwrap(),
            engine_name: CString::new("Default").unwrap(),
            app_version: 0,
            engine_version: 0,
            api_version: vk::API_VERSION_1_3,
            use_validation: validate(),
            validation_enables: None,
            validation_disables: None,
            use_debug: false,
            window_extensions: None,
            instance_extensions: vec![],
        }
    }
}

impl InstanceSource for Instance{
    fn get_instance(&self) -> &ash::Instance {
        &self.instance
    }
}

impl BusElement<QvkBusMessage> for Arc<Instance>{
    fn accepts_transaction(&self, _src: &dyn Bus<QvkBusMessage>, transaction: &qcom::bus::BusTransaction<QvkBusMessage>) -> bool {
        if let BusTransaction::InProgress(msg) = transaction{
            if let QvkBusMessage::Instance(_) = msg{
                return true;
            }
            return false;
        }
        false
    }

    fn handle_transaction(&self, _src: &dyn Bus<QvkBusMessage>, transaction: &mut qcom::bus::BusTransaction<QvkBusMessage>) -> Option<QvkBusMessage> {
        
        let msg = match transaction{
            BusTransaction::InProgress(msg) => msg,
            _ => panic!("Instance cannot handle finished transactions"),
        };

        let instance = match msg{
            QvkBusMessage::GetInstance => {self.clone()},
            _ => panic!("Instance can only handle GetInstance messages")
        };


        Some(QvkBusMessage::Instance(instance))
    }
}