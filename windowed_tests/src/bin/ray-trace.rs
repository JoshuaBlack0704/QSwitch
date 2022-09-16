use std::collections::HashMap;
use std::ffi::CString;

use ash::vk::{self, Packed24_8};
use glam::{Mat4, UVec3, Vec3};
use qforce::command::CommandPool;
use qforce::descriptor::{DescriptorSetOutline, DescriptorStack};
use qforce::init::{self, IEngine, WindowedEngine};
use qforce::init::{EngineInitOptions, SwapchainStore};
use qforce::memory::{
    AlignmentType, Allocator, AllocatorProfileStack, AllocatorProfileType, GeneralMemoryProfiles,
    ImageAllocatorProfile,
};
use qforce::ray_tracing::{
    Blas, RayTacingPipeline, RayTracingMemoryProfiles, ShaderTable, Tlas, TlasInstanceOutline,
    TriangleObjectGeometry,
};
use qforce::shader::Shader;
use qforce::sync::{Fence, Semaphore};
use qforce::IDisposable;
use time::Instant;
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
    nrm: [f32; 3],
    color: [f32; 3],
}
impl Vertex {
    pub fn combine(
        pos_data: &[[f32; 3]],
        normal_data: &[[f32; 3]],
        _color_data: &[[f32; 3]],
    ) -> Vec<Vertex> {
        assert_eq!(pos_data.len(), normal_data.len());
        //assert_eq!(pos_data.len(), color_data.len());

        pos_data
            .iter()
            .enumerate()
            .map(|(index, _)| Vertex {
                pos: pos_data[index],
                nrm: normal_data[index],
                color: [0.2, 0.5, 0.5],
            })
            .collect()
    }
}

#[repr(C)]
#[derive(Clone)]
pub struct CameraData {
    camera_translation: [f32; 4 * 4],
    camera_rotation: [f32; 4 * 4],
    light_pos: [f32; 4],
    light_color: [f32; 4],
    light_intensity: f32,
}
pub struct ShapeBuilder {
    vertices: Vec<Vec3>,
    indecies: Vec<UVec3>,
}
impl ShapeBuilder {
    pub fn new() -> ShapeBuilder {
        ShapeBuilder {
            vertices: vec![],
            indecies: vec![],
        }
    }
    pub fn add_primitive(&mut self, v1: Vec3, v2: Vec3, v3: Vec3, unique_vertices: bool) {
        let mut i1 = self.vertices.len();
        let mut i2 = self.vertices.len() + 1;
        let mut i3 = self.vertices.len() + 2;

        if !unique_vertices {
            match self.vertices.iter().enumerate().find(|(_, v)| **v == v1) {
                Some((index, _)) => {
                    i1 = index;
                }
                None => {
                    self.vertices.push(v1);
                }
            }
            match self.vertices.iter().enumerate().find(|(_, v)| **v == v2) {
                Some((index, _)) => {
                    i2 = index;
                }
                None => {
                    self.vertices.push(v2);
                }
            }
            match self.vertices.iter().enumerate().find(|(_, v)| **v == v3) {
                Some((index, _)) => {
                    i3 = index;
                }
                None => self.vertices.push(v3),
            }
        } else {
            self.vertices.push(v1);
            self.vertices.push(v2);
            self.vertices.push(v3);
        }

        let index_array = UVec3::new(i1 as u32, i2 as u32, i3 as u32);
        self.indecies.push(index_array);
    }
    pub fn get_pos_array(&self) -> Vec<[f32; 3]> {
        self.vertices.iter().map(|v| v.to_array()).collect()
    }
    pub fn get_normal_array(&self) -> Vec<[f32; 3]> {
        self.vertices
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let index = index as u32;
                let primatives = self
                    .indecies
                    .iter()
                    .filter(|p| p.to_array().contains(&index));
                let normals: Vec<Vec3> = primatives
                    .map(|p| {
                        let v1 = self.vertices[p.x as usize];
                        let v2 = self.vertices[p.y as usize];
                        let v3 = self.vertices[p.z as usize];
                        //(v1 - v3).cross(v3 - v2).normalize()
                        (v3 - v2).cross(v1 - v3).normalize()
                    })
                    .collect();

                let mut final_normal = Vec3::ZERO;
                for normal in normals.iter() {
                    final_normal += *normal;
                }
                final_normal.normalize().to_array()
            })
            .collect()
    }
    pub fn get_index_array(&self) -> Vec<u32> {
        let mut indecies = vec![];
        for index in self.indecies.iter() {
            indecies.push(index.x);
            indecies.push(index.y);
            indecies.push(index.z);
        }
        indecies
    }
}
pub struct ShaderStore {
    standard_ray_gen: Shader,
    standard_closest_hit: Shader,
    standard_miss: Shader,
    shadow_miss: Shader,
}
impl ShaderStore {
    pub fn new<T: IEngine>(engine: &T) -> ShaderStore {
        let mut options = shaderc::CompileOptions::new().unwrap();
        options.set_target_spirv(shaderc::SpirvVersion::V1_6);
        let ray_gen = Shader::new(
            engine,
            String::from(
                r#"
                #version 460
#extension GL_EXT_ray_tracing : require
#extension GL_KHR_vulkan_glsl : enable
        
layout(binding = 0, set = 0) uniform accelerationStructureEXT topLevelAS;
layout(binding = 1, set = 0, rgba32f) uniform image2D image;
layout(binding = 2, set = 0) uniform CameraData {
    mat4 translation;
    mat4 rotation;
    vec4 light_pos;
} camera;
struct hitPayload 
{
    
    vec4 hit_value;
    bool hit;
};

layout(location = 0) rayPayloadEXT hitPayload prd;

void main() 
{
    const vec2 pixelCenter = vec2(gl_LaunchIDEXT.xy) + vec2(0.5);
    const vec2 inUV = pixelCenter/vec2(gl_LaunchSizeEXT.xy);
    vec2 d = inUV * 2.0 - 1.0;
    vec4 origin    = vec4(0.0,0.0,-1.0,1.0);
    vec4 target    = vec4(origin.x + d.x, origin.y + -d.y, origin.z + 1, 1);
    vec4 direction = vec4(normalize(target.xyz - origin.xyz), 0);
    origin = camera.translation * origin;
    direction = camera.rotation * direction;
    uint  rayFlags = gl_RayFlagsOpaqueEXT;
    float tMin     = 0.001;
    float tMax     = 1000.0;
    
    //Sendoff
    traceRayEXT(
        topLevelAS, // acceleration structure
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
    imageStore(image, ivec2(gl_LaunchIDEXT.xy), prd.hit_value);
}

    "#,
            ),
            shaderc::ShaderKind::RayGeneration,
            "main",
            Some(&options),
        );
        let closest_hit = Shader::new(
            engine,
            String::from(
                r#"
                #version 460
#extension GL_EXT_ray_tracing : require
#extension GL_EXT_nonuniform_qualifier : enable
#extension GL_EXT_scalar_block_layout : enable
#extension GL_EXT_shader_explicit_arithmetic_types_int64 : require
#extension GL_EXT_buffer_reference2 : require

struct Vertex{
    vec3 pos;
    vec3 nrm;
    vec3 color;
};
struct ObjDesc{
    uint64_t vertex_address;
    uint64_t index_address;
};

layout(buffer_reference, scalar) buffer Vertices {Vertex v[]; }; // Positions of an object
layout(buffer_reference, scalar) buffer Indices {ivec3 i[]; };

layout(binding = 0, set = 0) uniform accelerationStructureEXT topLevelAS;
layout(binding = 2, set = 0) uniform CameraData {
    mat4 translation;
    mat4 rotation;
    vec4 light_pos;
    vec4 light_color;
    float light_intensity;
} camera;
layout(set = 0, binding = 3, scalar) buffer ObjDesc_ { ObjDesc i[]; } objDesc;

struct hitPayload
{
    vec4 hit_value;
    bool hit;
};

layout(location = 0) rayPayloadInEXT hitPayload hitdata;
layout(location = 1) rayPayloadEXT bool isShadowed;
hitAttributeEXT vec3 attribs;

void main()
{
    ObjDesc    objResource = objDesc.i[gl_InstanceCustomIndexEXT];
    Indices    indices     = Indices(objResource.index_address);
    Vertices   vertices    = Vertices(objResource.vertex_address);

    // Indices of the triangle
    ivec3 ind = indices.i[gl_PrimitiveID];
  
    // Vertex of the triangle
    Vertex v0 = vertices.v[ind.x];
    Vertex v1 = vertices.v[ind.y];
    Vertex v2 = vertices.v[ind.z];

    const vec3 barycentrics = vec3(1.0 - attribs.x - attribs.y, attribs.x, attribs.y);

    // Computing the coordinates of the hit position
    const vec3 pos      = v0.pos * barycentrics.x + v1.pos * barycentrics.y + v2.pos * barycentrics.z;
    const vec3 worldPos = vec3(gl_ObjectToWorldEXT * vec4(pos, 1.0));  // Transforming the position to world space
    // Computing the normal at hit position
    const vec3 nrm      = v0.nrm * barycentrics.x + v1.nrm * barycentrics.y + v2.nrm * barycentrics.z;
    const vec3 worldNrm = normalize(vec3(nrm * gl_WorldToObjectEXT));  // Transforming the normal to world space

    const vec3 reflectivity = v0.color * barycentrics.x + v1.color * barycentrics.y + v2.color * barycentrics.z;
    vec3 l_dir = (camera.light_pos.xyz - worldPos);
    float light_distance  = length(l_dir);
    l_dir = normalize(l_dir);
    float light_intensity = camera.light_intensity / (light_distance * light_distance);
    float factor = clamp(dot(worldNrm, -l_dir), 0.0, 1.0);

    vec3 color = (camera.light_color.xyz * light_intensity * factor) * reflectivity;

    vec4 ambience = vec4(reflectivity, 1.0) * camera.light_color * 0.1;
    isShadowed = true;
    if (factor > 0){
        uint  rayFlags = gl_RayFlagsTerminateOnFirstHitEXT | gl_RayFlagsOpaqueEXT | gl_RayFlagsSkipClosestHitShaderEXT;
        
        float tMin     = 0.001;
        float tMax     = light_distance;
        traceRayEXT(
            topLevelAS, // acceleration structure
            rayFlags,       // rayFlags
            0xFF,           // cullMask
            0,              // sbtRecordOffset
            0,              // sbtRecordStride
            1,              // missIndex
            worldPos,         // ray origin
            tMin,           // ray min range
            l_dir,      // ray direction
            tMax,           // ray max range
            1               // payload (location = 0)
        );
    }
    if (isShadowed){
        hitdata.hit_value = ambience;
    }
    else{
        hitdata.hit_value = vec4(color,1.0) + ambience;
    }
    hitdata.hit = true;
}
    "#,
            ),
            shaderc::ShaderKind::ClosestHit,
            "main",
            Some(&options),
        );
        let standard_miss = Shader::new(
            engine,
            String::from(
                r#"
                #version 460
                #extension GL_EXT_ray_tracing : require
                
                struct hitPayload
                {
                    vec4 hit_value;
                    bool hit;
                };
                
                layout(location = 0) rayPayloadInEXT hitPayload hitdata;
                
                void main()
                {
                    hitdata.hit_value = vec4(0.0, 0.1, 0.3, 1.0);
                    hitdata.hit = false;
                    
                }
    "#,
            ),
            shaderc::ShaderKind::Miss,
            "main",
            Some(&options),
        );
        let shadow_miss = Shader::new(
            engine,
            String::from(
                r#"
                #version 460
#extension GL_EXT_ray_tracing : require

layout(location = 1) rayPayloadInEXT bool isShadowed;

void main()
{
    isShadowed = false;
}
    "#,
            ),
            shaderc::ShaderKind::Miss,
            "main",
            Some(&options),
        );

        ShaderStore {
            standard_ray_gen: ray_gen,
            standard_closest_hit: closest_hit,
            standard_miss,
            shadow_miss,
        }
    }
    pub fn dispose(&mut self) {
        self.standard_ray_gen.dispose();
        self.standard_closest_hit.dispose();
        self.standard_miss.dispose();
        self.shadow_miss.dispose();
    }
}
pub struct CameraMovement {
    w_key: f32,
    s_key: f32,
    a_key: f32,
    d_key: f32,
    q_key: f32,
    e_key: f32,
    r_key: f32,
    f_key: f32,
    lshift_key: f32,
    lctrl_key: f32,
    camera_translation: Mat4,
    camera_rotation: Mat4,
    movement_speed: f32,
}
impl CameraMovement {
    pub fn new(movement_speed: f32) -> CameraMovement {
        CameraMovement {
            w_key: 0.0,
            s_key: 0.0,
            a_key: 0.0,
            d_key: 0.0,
            q_key: 0.0,
            e_key: 0.0,
            r_key: 0.0,
            f_key: 0.0,
            lshift_key: 0.0,
            lctrl_key: 0.0,
            camera_translation: Mat4::IDENTITY,
            camera_rotation: Mat4::IDENTITY,
            movement_speed,
        }
    }
    pub fn set_input(&mut self, input: winit::event::KeyboardInput) {
        match input.state {
            winit::event::ElementState::Pressed => match input.virtual_keycode {
                Some(code) => match code {
                    winit::event::VirtualKeyCode::W => {
                        self.w_key = 1.0;
                    }
                    winit::event::VirtualKeyCode::S => {
                        self.s_key = 1.0;
                    }
                    winit::event::VirtualKeyCode::A => {
                        self.a_key = 1.0;
                    }
                    winit::event::VirtualKeyCode::D => {
                        self.d_key = 1.0;
                    }
                    winit::event::VirtualKeyCode::LShift => {
                        self.lshift_key = 1.0;
                    }
                    winit::event::VirtualKeyCode::LControl => {
                        self.lctrl_key = 1.0;
                    }
                    winit::event::VirtualKeyCode::Q => {
                        self.q_key = 1.0;
                    }
                    winit::event::VirtualKeyCode::E => {
                        self.e_key = 1.0;
                    }
                    winit::event::VirtualKeyCode::R => {
                        self.r_key = 1.0;
                    }
                    winit::event::VirtualKeyCode::F => {
                        self.f_key = 1.0;
                    }
                    _ => {}
                },
                None => {}
            },
            winit::event::ElementState::Released => match input.virtual_keycode {
                Some(code) => match code {
                    winit::event::VirtualKeyCode::W => {
                        self.w_key = 0.0;
                    }
                    winit::event::VirtualKeyCode::S => {
                        self.s_key = 0.0;
                    }
                    winit::event::VirtualKeyCode::A => {
                        self.a_key = 0.0;
                    }
                    winit::event::VirtualKeyCode::D => {
                        self.d_key = 0.0;
                    }
                    winit::event::VirtualKeyCode::LShift => {
                        self.lshift_key = 0.0;
                    }
                    winit::event::VirtualKeyCode::LControl => {
                        self.lctrl_key = 0.0;
                    }
                    winit::event::VirtualKeyCode::Q => {
                        self.q_key = 0.0;
                    }
                    winit::event::VirtualKeyCode::E => {
                        self.e_key = 0.0;
                    }
                    winit::event::VirtualKeyCode::R => {
                        self.r_key = 0.0;
                    }
                    winit::event::VirtualKeyCode::F => {
                        self.f_key = 0.0;
                    }
                    _ => {}
                },
                None => {}
            },
        }
    }
    pub fn march_camera(&mut self, delta_time: f32, sim_time: f32) -> CameraData {
        let mut vector = Vec3::new(
            self.d_key - self.a_key,
            self.lshift_key - self.lctrl_key,
            self.w_key - self.s_key,
        )
        .normalize();
        vector *= self.movement_speed * delta_time;
        let y_angle = (self.e_key - self.q_key) * 2.0 * delta_time;
        let x_angle = (self.r_key - self.f_key) * 2.0 * delta_time;

        let mut camera_translation: [f32; 4 * 4] = [0.0; 4 * 4];
        let mut camera_rotation: [f32; 4 * 4] = [0.0; 4 * 4];

        if !vector.is_nan() {
            self.camera_translation = self.camera_translation
                * Mat4::from_translation(self.camera_rotation.transform_vector3(vector));
            self.camera_translation
                .write_cols_to_slice(&mut camera_translation);
        } else {
            self.camera_translation
                .write_cols_to_slice(&mut camera_translation);
        }
        if y_angle != 0.0 || x_angle != 0.0 {
            self.camera_rotation = self.camera_rotation
                * Mat4::from_rotation_y(y_angle)
                * Mat4::from_rotation_x(x_angle);
            self.camera_rotation
                .write_cols_to_slice(&mut camera_rotation);
        } else {
            self.camera_rotation
                .write_cols_to_slice(&mut camera_rotation);
        }

        let light_angle = sim_time / 5.0;
        let orbit_radius = 10.0;
        CameraData {
            camera_translation,
            camera_rotation,
            light_pos: [orbit_radius * light_angle.cos(), 0.0, 10.0 + orbit_radius * 2.0 * light_angle.sin(), 1.0],
            light_color: [1.0, 1.0, 1.0, 1.0],
            light_intensity: 70.0,
        }
    }
}

#[allow(unused, dead_code)]
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

    let render_image_profile = AllocatorProfileType::Image(ImageAllocatorProfile::new(
        vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC,
        vk::Format::B8G8R8A8_UNORM,
        vk::Extent3D::builder()
            .width(4000)
            .height(4000)
            .depth(1)
            .build(),
        &[],
    ));
    let mut allocator = Allocator::new(&engine);
    let general_profiles =
        GeneralMemoryProfiles::new(&mut allocator, 10 * 1024 * 1024, 100 * 1024 * 1024);
    let image_profile = AllocatorProfileStack::TargetImage(
        general_profiles.general_device_index,
        allocator.add_profile(render_image_profile),
    );
    let ray_tracing_profiles = RayTracingMemoryProfiles::new(&engine, &mut allocator);

    

    let mut shaders = ShaderStore::new(&engine);

    let main = CString::new("main").unwrap();
    let sbt = ShaderTable {
        ray_gen: vec![shaders
            .standard_ray_gen
            .get_stage(vk::ShaderStageFlags::RAYGEN_KHR, &main)],
        hit_groups: vec![(
            vk::RayTracingShaderGroupTypeKHR::TRIANGLES_HIT_GROUP,
            (
                Some(
                    shaders
                        .standard_closest_hit
                        .get_stage(vk::ShaderStageFlags::CLOSEST_HIT_KHR, &main),
                ),
                None,
                None,
            ),
        )],
        misses: vec![
            shaders
                .standard_miss
                .get_stage(vk::ShaderStageFlags::MISS_KHR, &main),
            shaders
                .shadow_miss
                .get_stage(vk::ShaderStageFlags::MISS_KHR, &main),
        ],
    };

    let mut shape_builder = ShapeBuilder::new();
    let p0 = Vec3::new(0.0, 1.0, 0.0);
    let p1 = Vec3::new(-1.0, -1.0, 0.5);
    let p2 = Vec3::new(1.0, -1.0, 0.5);
    let p3 = Vec3::new(0.0, -1.0, -0.5);
    shape_builder.add_primitive(p3, p2, p0, true);
    shape_builder.add_primitive(p1, p0, p2, true);
    shape_builder.add_primitive(p1, p3, p0, true);
    shape_builder.add_primitive(p1, p2, p3, true);

    let p_data = shape_builder.get_pos_array();
    let n_data = shape_builder.get_normal_array();
    let i_data = shape_builder.get_index_array();
    let v_data = Vertex::combine(&p_data, &n_data, &[]);

    let tetrahedron_data = TriangleObjectGeometry::new(
        &engine,
        &ray_tracing_profiles,
        &mut allocator,
        &v_data,
        vk::Format::R32G32B32_SFLOAT,
        &i_data,
    );

    let mut shape_builder = ShapeBuilder::new();
    let p0 = Vec3::new(-50.0, 0.0, -50.0);
    let p1 = Vec3::new(-50.0, 0.0, 50.0);
    let p2 = Vec3::new(50.0, 0.0, -50.0);
    let p3 = Vec3::new(50.0, 0.0, 50.0);
    shape_builder.add_primitive(p1, p2, p3, true);
    shape_builder.add_primitive(p1, p0, p3, true);

    let p_data = shape_builder.get_pos_array();
    let n_data = shape_builder.get_normal_array();
    let i_data = shape_builder.get_index_array();
    let v_data = Vertex::combine(&p_data, &n_data, &[]);

    let plane_data = TriangleObjectGeometry::new(
        &engine,
        &ray_tracing_profiles,
        &mut allocator,
        &v_data,
        vk::Format::R32G32B32_SFLOAT,
        &i_data,
    );

    let tetrahedron_blas_outlines = [tetrahedron_data.get_blas_outline(1)];
    let plane_blas_outlines = [plane_data.get_blas_outline(1)];
    let mut tetrahedron_blas = Blas::new(
        &engine,
        &ray_tracing_profiles,
        &mut allocator,
        &tetrahedron_blas_outlines,
    );
    let mut plane_blas = Blas::new(
        &engine,
        &ray_tracing_profiles,
        &mut allocator,
        &plane_blas_outlines,
    );
    let dimension = 5;
    let tetahedron_default_instance = vk::AccelerationStructureInstanceKHR {
        transform: vk::TransformMatrixKHR { matrix: [0.0; 12] },
        instance_custom_index_and_mask: Packed24_8::new(0, 0xff),
        instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(0, 0x00000001 as u8),
        acceleration_structure_reference: tetrahedron_blas.get_blas_ref(),
    };
    let mut tetrahedron_instances = vec![];
    for z in 0..dimension {
        for y in -dimension..dimension {
            for x in -dimension..dimension {
                let transform = vk::TransformMatrixKHR {
                    matrix: [
                        1.0,
                        0.0,
                        0.0,
                        (x * 3) as f32,
                        0.0,
                        1.0,
                        0.0,
                        (y * 3) as f32,
                        0.0,
                        0.0,
                        1.0,
                        (5 + z * 3) as f32,
                    ],
                };
                let mut instance = tetahedron_default_instance.clone();
                instance.transform = transform;
                tetrahedron_instances.push(instance);
            }
        }
    }
    let tetrahedron_instance_buffer = Tlas::prepare_instance_memory(
        &engine,
        &ray_tracing_profiles,
        &mut allocator,
        tetrahedron_instances.len(),
        Some(&tetrahedron_instances),
    );
    let plane_default_instance = vk::AccelerationStructureInstanceKHR {
        transform: vk::TransformMatrixKHR { matrix: [
            1.0,0.0,0.0, 0.0,
            0.0,1.0,0.0,-30.0,
            0.0,0.0,1.0, 0.0,
        ] },
        instance_custom_index_and_mask: Packed24_8::new(1, 0xff),
        instance_shader_binding_table_record_offset_and_flags: Packed24_8::new(0, 0x00000001 as u8),
        acceleration_structure_reference: plane_blas.get_blas_ref(),
    };
    let mut plane_instances = vec![plane_default_instance];
    let plane_instance_buffer = Tlas::prepare_instance_memory(
        &engine,
        &ray_tracing_profiles,
        &mut allocator,
        plane_instances.len(),
        Some(&plane_instances),
    );
    let instance_data = [TlasInstanceOutline {
        instance_data: vk::DeviceOrHostAddressConstKHR {
            device_address: tetrahedron_instance_buffer.get_device_address(),
        },
        instance_count: tetrahedron_instances.len() as u32,
        instance_count_overkill: 1,
        array_of_pointers: false,
    },
    TlasInstanceOutline {
        instance_data: vk::DeviceOrHostAddressConstKHR {
            device_address: plane_instance_buffer.get_device_address(),
        },
        instance_count: plane_instances.len() as u32,
        instance_count_overkill: 1,
        array_of_pointers: false,
    }];
    let mut tlas = Tlas::new(
        &engine,
        &ray_tracing_profiles,
        &mut allocator,
        &instance_data,
    );
    let queue_data = engine
        .get_queue_store()
        .get_queue(vk::QueueFlags::GRAPHICS | vk::QueueFlags::TRANSFER)
        .unwrap();
    let mut render_pool = CommandPool::new(
        &engine,
        vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_data.1)
            .build(),
    );
    let render_cmd = render_pool.get_command_buffers(
        vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .build(),
    )[0];
    let mut transfer_pool = CommandPool::new(
        &engine,
        vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_data.1)
            .build(),
    );
    let transfer_cmd = transfer_pool.get_command_buffers(
        vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .build(),
    )[0];

    let mut render_target = allocator.get_image_resources(
        &image_profile,
        vk::ImageAspectFlags::COLOR,
        0,
        1,
        0,
        1,
        vk::ImageViewType::TYPE_2D,
        vk::Format::B8G8R8A8_UNORM,
        &[],
    );
    render_target.internal_transition(&engine, vk::ImageLayout::GENERAL);

    let camera_data_stage = allocator.get_buffer_region::<CameraData>(
        &general_profiles.host_storage,
        1,
        &AlignmentType::Free,
        &[],
    );
    let camera_data_mem = allocator.get_buffer_region::<CameraData>(
        &general_profiles.device_uniform,
        1,
        &AlignmentType::Free,
        &[],
    );

    let object_shader_data = [tetrahedron_data.get_shader_data(), plane_data.get_shader_data()];
    let object_shader_data_mem = allocator.get_buffer_region_from_slice(
        &general_profiles.host_storage,
        &general_profiles.device_storage,
        &object_shader_data,
        &AlignmentType::Free,
        &[],
    );

    let mut d_outline = DescriptorSetOutline::new(&engine, &[]);
    let tlas_binding = d_outline.add_binding(
        vk::DescriptorType::ACCELERATION_STRUCTURE_KHR,
        1,
        vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR,
    );
    let rander_target_binding = d_outline.add_binding(
        vk::DescriptorType::STORAGE_IMAGE,
        1,
        vk::ShaderStageFlags::RAYGEN_KHR,
    );
    let camera_data_binding = d_outline.add_binding(
        vk::DescriptorType::UNIFORM_BUFFER,
        1,
        vk::ShaderStageFlags::RAYGEN_KHR | vk::ShaderStageFlags::CLOSEST_HIT_KHR,
    );
    let object_shader_data_binding = d_outline.add_binding(
        vk::DescriptorType::STORAGE_BUFFER,
        1,
        vk::ShaderStageFlags::CLOSEST_HIT_KHR,
    );

    let mut d_stack = DescriptorStack::new(&engine);
    let render_set = d_stack.add_outline(d_outline);
    d_stack.create_sets(&[]);
    let mut set = d_stack.get_set(render_set);
    let mut write_requests = [
        (0, 0, tlas.get_write()),
        (1, 0, render_target.get_write(None)),
        (2, 0, camera_data_mem.get_write()),
        (3, 0, object_shader_data_mem.get_write()),
    ];
    set.write(&mut write_requests);

    let mut ray_pipeline = RayTacingPipeline::new(
        &engine,
        &sbt,
        &ray_tracing_profiles,
        &mut allocator,
        &[set.get_layout()],
        &[],
    );

    let mut render_loop_fence = Fence::new(&engine, true);
    let mut render_semaphore = Semaphore::new(&engine);
    let mut transfer_semaphore = Semaphore::new(&engine);
    let mut image_aquire_semaphore = Semaphore::new(&engine);
    let mut running = true;
    let mut instant = Box::new(Instant::now());
    let mut delta_time = 0.0;
    let mut camera = CameraMovement::new(30.0);
    let mut sim_time = 0.0;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = winit::event_loop::ControlFlow::Poll;
        match event {
            winit::event::Event::WindowEvent { window_id, event } => {
                match event {
                    winit::event::WindowEvent::Resized(_) => {
                        if !running {
                            return;
                        }
                        render_loop_fence.wait();
                        render_pool.reset();
                        swapchain = SwapchainStore::new(
                            &engine,
                            &[
                                init::CreateSwapchainOptions::OldSwapchain(&swapchain),
                                init::CreateSwapchainOptions::ImageUsages(
                                    vk::ImageUsageFlags::TRANSFER_DST
                                        | vk::ImageUsageFlags::COLOR_ATTACHMENT,
                                ),
                            ],
                        );
                        //Here we need to record our ray ray_trace command as well as get a new image resource
                        width = swapchain.get_extent().width;
                        height = swapchain.get_extent().height;
                        extent = vk::Extent3D::builder()
                            .width(width)
                            .height(height)
                            .depth(1)
                            .build();

                        let device = engine.get_device();
                        let ray_loader = ash::extensions::khr::RayTracingPipeline::new(
                            &engine.get_instance(),
                            &device,
                        );
                        let (ray_gen_address, miss_address, hit_address) =
                            ray_pipeline.sbt_addresses;
                        unsafe {
                            device.begin_command_buffer(
                                render_cmd,
                                &vk::CommandBufferBeginInfo::builder().build(),
                            );

                            camera_data_stage.copy_to_region(render_cmd, &camera_data_mem);
                            let memory_barrier = [vk::MemoryBarrier::builder()
                                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                                .dst_access_mask(vk::AccessFlags::SHADER_READ)
                                .build()];
                            device.cmd_pipeline_barrier(
                                render_cmd,
                                vk::PipelineStageFlags::TRANSFER,
                                vk::PipelineStageFlags::RAY_TRACING_SHADER_KHR,
                                vk::DependencyFlags::empty(),
                                &memory_barrier,
                                &[],
                                &[],
                            );

                            device.cmd_bind_pipeline(
                                render_cmd,
                                vk::PipelineBindPoint::RAY_TRACING_KHR,
                                ray_pipeline.get_pipeline(),
                            );
                            device.cmd_bind_descriptor_sets(
                                render_cmd,
                                vk::PipelineBindPoint::RAY_TRACING_KHR,
                                ray_pipeline.get_pipeline_layout(),
                                0,
                                &[set.get_set()],
                                &[],
                            );
                            ray_loader.cmd_trace_rays(
                                render_cmd,
                                &ray_gen_address,
                                &miss_address,
                                &hit_address,
                                &vk::StridedDeviceAddressRegionKHR::default(),
                                width,
                                height,
                                1,
                            );
                            device
                                .end_command_buffer(render_cmd)
                                .expect("Could not end command buffer");
                            render_target
                                .set_target_extent(extent, vk::Offset3D::builder().build());
                        }
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        *control_flow = winit::event_loop::ControlFlow::Exit;
                        running = false;
                        unsafe {
                            engine
                                .get_device()
                                .device_wait_idle()
                                .expect("Could not stop device")
                        };
                        render_pool.dispose();
                        transfer_pool.dispose();
                        ray_pipeline.dispose();
                        tlas.dispose();
                        tetrahedron_blas.dispose();
                        plane_blas.dispose();
                        allocator.dispose();
                        d_stack.dispose();
                        render_loop_fence.dispose();
                        render_semaphore.dispose();
                        transfer_semaphore.dispose();
                        image_aquire_semaphore.dispose();
                        render_target.dispose();
                        shaders.dispose();
                    }
                    winit::event::WindowEvent::KeyboardInput {
                        device_id,
                        input,
                        is_synthetic,
                    } => {
                        camera.set_input(input);
                    }
                    _ => {}
                }
            }
            winit::event::Event::MainEventsCleared => {
                if !running {
                    return;
                }
                render_loop_fence.wait_reset();
                delta_time = instant.elapsed().as_seconds_f32();
                sim_time += delta_time;
                println!("Frame time {:.3} ms", delta_time * 1000.0);
                *instant = Instant::now();
                transfer_pool.reset();
                let swapchains = [swapchain.get_swapchain()];
                let (image_index, present_target) = swapchain.get_next_image(
                    u64::MAX,
                    Some(image_aquire_semaphore.semaphore),
                    None,
                );
                let image_index = [image_index];
                let render_wait_semaphores = [image_aquire_semaphore.semaphore];
                let render_wait_stage = [vk::PipelineStageFlags::RAY_TRACING_SHADER_KHR];
                let render_signal = [render_semaphore.semaphore];
                let transfer_wait_semaphores = [render_semaphore.semaphore];
                let transfer_wait_stage = [vk::PipelineStageFlags::TRANSFER];
                let transfer_signal = [transfer_semaphore.semaphore];

                let render_cmds = [render_cmd];
                let transfer_cmds = [transfer_cmd];
                let render_submit = vk::SubmitInfo::builder()
                    .command_buffers(&render_cmds)
                    .wait_semaphores(&render_wait_semaphores)
                    .wait_dst_stage_mask(&render_wait_stage)
                    .signal_semaphores(&render_signal);
                let transfer_submit = vk::SubmitInfo::builder()
                    .command_buffers(&transfer_cmds)
                    .wait_semaphores(&transfer_wait_semaphores)
                    .wait_dst_stage_mask(&transfer_wait_stage)
                    .signal_semaphores(&transfer_signal);
                let present_info = vk::PresentInfoKHR::builder()
                    .image_indices(&image_index)
                    .swapchains(&swapchains)
                    .wait_semaphores(&transfer_signal);
                let submits = [render_submit.build(), transfer_submit.build()];

                unsafe {
                    let device = engine.get_device();
                    device
                        .begin_command_buffer(
                            transfer_cmd,
                            &vk::CommandBufferBeginInfo::builder()
                                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
                                .build(),
                        )
                        .expect("Could not begin command buffer");

                    let present_target_to_transfer = present_target.transition(
                        vk::AccessFlags::NONE,
                        vk::AccessFlags::MEMORY_WRITE,
                        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                    );
                    let present_target_to_transfer = present_target_to_transfer.0;

                    let transfer_transitions = [present_target_to_transfer];

                    device.cmd_pipeline_barrier(
                        transfer_cmd,
                        vk::PipelineStageFlags::TOP_OF_PIPE,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &transfer_transitions,
                    );
                    render_target.copy_to_image(transfer_cmd, &present_target);
                    let present_target_to_preset = present_target.transition(
                        vk::AccessFlags::MEMORY_WRITE,
                        vk::AccessFlags::NONE,
                        vk::ImageLayout::PRESENT_SRC_KHR,
                    );
                    let present_target_to_present = present_target_to_preset.0;

                    let reset_transitions = [present_target_to_present];
                    device.cmd_pipeline_barrier(
                        transfer_cmd,
                        vk::PipelineStageFlags::TRANSFER,
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                        vk::DependencyFlags::empty(),
                        &[],
                        &[],
                        &reset_transitions,
                    );
                    device
                        .end_command_buffer(transfer_cmd)
                        .expect("Could not end command buffer");
                    let camera_data = camera.march_camera(delta_time, sim_time);
                    allocator.copy_from_ram(&camera_data, 1, &camera_data_stage);
                    device.queue_submit(queue_data.0, &submits, render_loop_fence.get_fence());
                    swapchain.present(queue_data.0, image_index[0], &transfer_signal);
                }
            }
            _ => {}
        }
    });
}
