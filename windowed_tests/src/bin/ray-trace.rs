use std::ffi::CString;

use ash::vk::{self, Packed24_8};
use qforce::init::{self, WindowedEngine};
use qforce::init::{EngineInitOptions, SwapchainStore};
use qforce::memory::Allocator;
use qforce::ray_tracing::{
    Blas, RayTacingPipeline, RayTracingMemoryProfiles, ShaderTable, Tlas, TlasInstanceOutline,
    TriangleObjectGeometry,
};
use qforce::shader::Shader;
#[cfg(debug_assertions)]
fn get_vulkan_validate(options: &mut Vec<init::EngineInitOptions>) {
    println!("Validation Layers Active");
    let validation_features = [
        vk::ValidationFeatureEnableEXT::BEST_PRACTICES,
        vk::ValidationFeatureEnableEXT::GPU_ASSISTED,
        vk::ValidationFeatureEnableEXT::SYNCHRONIZATION_VALIDATION,
    ];
    options.push(init::EngineInitOptions::UseValidation(
        Some(validation_features.to_vec()),
        None,
    ));
    options.push(EngineInitOptions::UseDebugUtils);
}
#[cfg(not(debug_assertions))]
fn get_vulkan_validate(options: &mut Vec<init::EngineInitOptions>) {
    println!("Validation Layers Inactive");
}

#[repr(C)]
#[derive(Clone)]
pub struct Vertex {
    pos: [f32; 3],
}
fn main() {
    let (event_loop, engine);
    {
        match pretty_env_logger::try_init() {
            Ok(_) => {}
            Err(_) => {}
        };
        let features12 = vk::PhysicalDeviceVulkan12Features::builder()
            .buffer_device_address(true)
            .timeline_semaphore(true);
        let acc_features = vk::PhysicalDeviceAccelerationStructureFeaturesKHR::builder()
            .acceleration_structure(true);
        let ray_tracing_features =
            vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::builder().ray_tracing_pipeline(true);
        let acc_extension = ash::extensions::khr::AccelerationStructure::name().as_ptr();
        let ray_tracing = ash::extensions::khr::RayTracingPipeline::name().as_ptr();
        let def_host = ash::extensions::khr::DeferredHostOperations::name().as_ptr();

        let mut options = vec![
            EngineInitOptions::DeviceFeatures12(features12.build()),
            EngineInitOptions::DeviceFeaturesAccelerationStructure(acc_features.build()),
            EngineInitOptions::DeviceFeaturesRayTracing(ray_tracing_features.build()),
            EngineInitOptions::DeviceExtensions(vec![acc_extension, def_host, ray_tracing]),
        ];
        get_vulkan_validate(&mut options);
        (event_loop, engine) = WindowedEngine::init(&mut options);
    }

    let mut swapchain = SwapchainStore::new(
        &engine,
        &[init::CreateSwapchainOptions::ImageUsages(
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::COLOR_ATTACHMENT,
        )],
    );

    let mut width: u32 = swapchain.get_extent().width;
    let mut height: u32 = swapchain.get_extent().height;
    let mut extent = vk::Extent3D::builder()
        .width(width)
        .height(height)
        .depth(1)
        .build();

    let mut allocator = Allocator::new(&engine);
    let ray_tracing_profiles = RayTracingMemoryProfiles::new(&engine, &mut allocator);
    let v_data = [
        Vertex {
            pos: [0.0, 1.0, 0.0],
        }, //top
        Vertex {
            pos: [-1.0, -1.0, 0.5],
        }, //left
        Vertex {
            pos: [1.0, -1.0, 0.5],
        }, //right
        Vertex {
            pos: [0.0, -1.0, -0.5],
        }, //front
    ];
    let i_data = [
        3, 2, 0, //fr
        1, 0, 2, //back
        1, 3, 0, //fl
        1, 2, 3,
    ]; //bottom

    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_target_spirv(shaderc::SpirvVersion::V1_6);
    let ray_gen = Shader::new(
        &engine,
        String::from(
            r#"
    #version 460
    #extension GL_EXT_ray_tracing : require
    #extension GL_KHR_vulkan_glsl : enable

    layout(binding = 0, set = 0) uniform accelerationStructureEXT topLevelAS;
    layout(binding = 1, set = 0, rgba32f) uniform image2D image;

    struct hitPayload
    {
        bool hit;
        vec3 hitValue;
    };

    layout(location = 0) rayPayloadEXT hitPayload prd;

    void main() 
        {
            const vec2 pixelCenter = vec2(gl_LaunchIDEXT.xy) + vec2(0.5);
            const vec2 inUV = pixelCenter/vec2(gl_LaunchSizeEXT.xy);
            vec2 d = inUV * 2.0 - 1.0;
            vec4 origin    = vec4(0, 0, -1, 1);
            vec4 target    = vec4(d.x, -d.y, 0, 1);
            vec4 direction = vec4(normalize(target.xyz - origin.xyz), 0);
            uint  rayFlags = gl_RayFlagsOpaqueEXT;
            float tMin     = 0.001;
            float tMax     = 100000.0;
            traceRayEXT(topLevelAS, // acceleration structure
                rayFlags,       // rayFlags
                0xFF,           // cullMask
                0,              // sbtRecordOffset
                0,              // sbtRecordStride
                0,              // missIndex
                origin.xyz,     // ray origin
                tMin,           // ray min range
                direction.xyz,  // ray direction
                tMax,           // ray max range
                0               // payload (location = 0)
        );
            if (d.x > 0 && prd.hit){
                prd.hitValue = prd.hitValue * 0.5;
            }
            imageStore(image, ivec2(gl_LaunchIDEXT.xy), vec4(prd.hitValue,1.0));
        }

    "#,
        ),
        shaderc::ShaderKind::RayGeneration,
        "main",
        Some(&options),
    );
    let closest_hit = Shader::new(
        &engine,
        String::from(
            r#"
    #version 460
    #extension GL_EXT_ray_tracing : require
    #extension GL_EXT_nonuniform_qualifier : enable

    struct hitPayload
    {
        bool hit;
        vec3 hitValue;
    };

    layout(location = 0) rayPayloadInEXT hitPayload hitdata;
    hitAttributeEXT vec3 attribs;

    void main()
    {
        hitdata.hit = true;
        hitdata.hitValue = vec3(0.2, 0.5, 0.5);
    }"#,
        ),
        shaderc::ShaderKind::ClosestHit,
        "main",
        Some(&options),
    );
    let miss = Shader::new(
        &engine,
        String::from(
            r#"
    #version 460
    #extension GL_EXT_ray_tracing : require

    struct hitPayload
    {
        bool hit;
        vec3 hitValue;
    };

    layout(location = 0) rayPayloadInEXT hitPayload hitdata;

    void main()
    {
        hitdata.hit = false;
        hitdata.hitValue = vec3(0.0, 0.1, 0.3);
    }"#,
        ),
        shaderc::ShaderKind::Miss,
        "main",
        Some(&options),
    );
    let main = CString::new("main").unwrap();
    let sbt = ShaderTable {
        ray_gen: vec![ray_gen.get_stage(vk::ShaderStageFlags::RAYGEN_KHR, &main)],
        hit_groups: vec![(
            vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP,
            (
                Some(closest_hit.get_stage(vk::ShaderStageFlags::CLOSEST_HIT_KHR, &main)),
                None,
                None,
            ),
        )],
        misses: vec![miss.get_stage(vk::ShaderStageFlags::MISS_KHR, &main)],
    };
    let ray_pipeline = RayTacingPipeline::new(
        &engine,
        &sbt,
        &ray_tracing_profiles,
        &mut allocator,
        &[],
        &[],
    );

    let object_data = TriangleObjectGeometry::new(
        &engine,
        &ray_tracing_profiles,
        &mut allocator,
        &v_data,
        vk::Format::R32G32B32_SFLOAT,
        &i_data,
    );
    let blas_outlines = [object_data.get_blas_outline(1)];
    let blas = Blas::new(
        &engine,
        &ray_tracing_profiles,
        &mut allocator,
        &blas_outlines,
    );
    let transform = vk::TransformMatrixKHR {
        matrix: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0],
    };
    let default_instance = vk::AccelerationStructureInstanceKHR {
        transform,
        instance_custom_index_and_mask: Packed24_8::new(0, 0xff),
        instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(0, 0x00000002 as u8),
        acceleration_structure_reference: blas.get_blas_ref(),
    };
    let instance_buffer = Tlas::prepare_instance_memory(
        &engine,
        &ray_tracing_profiles,
        &mut allocator,
        100,
        Some(default_instance),
    );
    let instance_data = [TlasInstanceOutline {
        instance_data: vk::DeviceOrHostAddressConstKHR {
            device_address: instance_buffer.get_device_address(),
        },
        instance_count: 100,
        instance_count_overkill: 1,
        array_of_pointers: false,
    }];
    let _tlas = Tlas::new(
        &engine,
        &ray_tracing_profiles,
        &mut allocator,
        &instance_data,
    );
}
