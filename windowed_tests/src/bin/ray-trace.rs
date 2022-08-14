use std::ffi::c_void;
use ash::{self, vk};
use qforce::core::{self, memory};


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
    let blas = [memory::BlasOutline::new_triangle(
        &v_data, 
        vk::Format::R32G32B32_SFLOAT, 
        &i_data, 
        None, 
        vk::GeometryFlagsKHR::empty(), 
        vk::BuildAccelerationStructureFlagsKHR::empty(), 
        null, 
        null, 
        null)
        ,
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
            memory::BlasOutline::new_triangle(
                &v_data, 
                vk::Format::R32G32B32_SFLOAT, 
                &i_data, 
                None, 
                vk::GeometryFlagsKHR::empty(), 
                vk::BuildAccelerationStructureFlagsKHR::empty(), 
                null, 
                null, 
                null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
        memory::BlasOutline::new_triangle(
            &v_data, 
            vk::Format::R32G32B32_SFLOAT, 
            &i_data, 
            None, 
            vk::GeometryFlagsKHR::empty(), 
            vk::BuildAccelerationStructureFlagsKHR::empty(), 
            null, 
            null, 
            null),
            memory::BlasOutline::new_triangle(
                &v_data, 
                vk::Format::R32G32B32_SFLOAT, 
                &i_data, 
                None, 
                vk::GeometryFlagsKHR::empty(), 
                vk::BuildAccelerationStructureFlagsKHR::empty(), 
                null, 
                null, 
                null)];
    
    let blas_store = memory::BlasStore::new_immediate(&engine, &blas);    
}