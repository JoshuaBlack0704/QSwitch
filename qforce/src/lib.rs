pub mod engine {
    use std::sync::Arc;

    use ash::{self, vk};
    use qvk::init::{
        self, IVulkanInit, IWindowedVulkanInit, Initializer, VulkanInitOptions, WindowedInitalizer,
    };

    #[cfg(debug_assertions)]
    fn get_vulkan_validate(options: &mut Vec<init::VulkanInitOptions>) {
        println!("Validation Layers Active");
        let validation_features = [
            vk::ValidationFeatureEnableEXT::BEST_PRACTICES,
            vk::ValidationFeatureEnableEXT::GPU_ASSISTED,
            vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION,
        ];
        options.push(init::VulkanInitOptions::UseValidation(
            Some(validation_features.to_vec()),
            None,
        ));
        options.push(VulkanInitOptions::UseDebugUtils);
    }
    #[cfg(not(debug_assertions))]
    fn get_vulkan_validate(options: &mut Vec<init::VulkanInitOptions>) {
        println!("Validation Layers Inactive");
    }

    pub struct Engine<U> {
        init: Arc<U>,
    }
    pub fn new() -> Engine<Initializer> {
        let mut options = vec![];
        get_vulkan_validate(&mut options);
        let init = Initializer::init(&mut options, None);
        Engine { init: init.0 }
    }
    pub fn new_windowed() -> (winit::event_loop::EventLoop<()>, Engine<WindowedInitalizer>) {
        let mut options = vec![];
        get_vulkan_validate(&mut options);
        let init = WindowedInitalizer::init(&mut options);

        (init.0, Engine { init: init.1 })
    }

    impl<I: IVulkanInit> Engine<I> {
        pub fn hello(&self) {
            println!("Hello");
        }
    }
    impl<I: IWindowedVulkanInit + IVulkanInit> Engine<I> {
        pub fn hello_window(&self) {
            println!("Hello window");
        }
    }
}
pub mod data {
    use rand::Rng;

    #[derive(Clone)]
    pub struct Vector {
        pub x: f32,
        pub y: f32,
        pub z: f32,
    }
    impl Vector {
        pub fn random_cube<R: Rng>(bounds: f32, rng: &mut R) -> Vector {
            let x = rng.gen_range(0.0..=bounds);
            let y = rng.gen_range(0.0..=bounds);
            let z = rng.gen_range(0.0..=bounds);
            Vector { x, y, z }
        }
    }
}
