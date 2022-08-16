use std::ffi::c_void;
use ash::{self, vk};
use qforce::core::{self, memory};
use cgmath;


#[cfg(debug_assertions)]
    fn get_vulkan_validate() -> bool{
        println!("Validation Layers Active");
        true
    }
    #[cfg(not(debug_assertions))]
    fn get_vulkan_validate() -> bool {
        println!("Validation Layers Inactive");
        false
    }


#[repr(C)]
    #[derive(Clone)]
    pub struct Vertex{
        pos: [f32; 3],
    }

    
fn main(){
    let _err = pretty_env_logger::try_init();
    let engine = core::WindowlessEngine::init(get_vulkan_validate());
    let v_data = [
        Vertex{pos: [ 1.0,-1.0,0.0]},
        Vertex{pos: [ 0.0, 1.0,0.0]},
        Vertex{pos: [-1.0,-1.0,0.0]},
    ];
    let null = 0 as *const c_void;
    let i_data = [1,2,3];
    let objects = [core::ray_tracing::acceleration_structures::ObjectOutline{ 
        vertex_data: v_data.to_vec(), 
        vertex_format: vk::Format::R32G32B32_SFLOAT, 
        index_data: i_data.to_vec(), 
        inital_pos_data: vec![cgmath::vec4(0.0, 0.0, 1.0, 0.0)],
        sbt_hit_group_offset: 0, }];
    let store = core::ray_tracing::acceleration_structures::ObjectStore::new(&engine, &objects);

    let tlas = core::ray_tracing::acceleration_structures::Tlas::new_immediate::<core::WindowlessEngine,Vertex>(&engine, store.0.get_instance_count(), store.0.get_instance_address());
}