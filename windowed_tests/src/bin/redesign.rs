use qforce::init;
use ash::vk;

fn main(){

        let validation_features = [vk::ValidationFeatureEnableEXT::BEST_PRACTICES];

        match pretty_env_logger::try_init(){
            Ok(_) => {},
            Err(_) => {},
        };

        let device_extension_names_raw = [
            ash::extensions::khr::AccelerationStructure::name().as_ptr(),
            ash::extensions::khr::DeferredHostOperations::name().as_ptr(),
            ash::extensions::khr::RayTracingPipeline::name().as_ptr(),
        ];
        let ray_features = vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder().ray_tracing_pipeline(true).build();
        let acc_features = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder().acceleration_structure(true).build();
        let features12 = vk::PhysicalDeviceVulkan12Features::builder().timeline_semaphore(true).buffer_device_address(true).build();


        let mut options = vec![
            init::EngineInitOptions::UseValidation(Some(&validation_features), None),
             init::EngineInitOptions::UseDebugUtils,
             init::EngineInitOptions::DeviceExtensions(&device_extension_names_raw),
             init::EngineInitOptions::DeviceFeatures12(features12),
             init::EngineInitOptions::DeviceFeaturesRayTracing(ray_features),
             init::EngineInitOptions::DeviceFeaturesAccelerationStructure(acc_features)];
        let engine = init::WindowedEngine::init(&mut options);
}