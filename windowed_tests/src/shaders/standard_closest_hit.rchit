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
