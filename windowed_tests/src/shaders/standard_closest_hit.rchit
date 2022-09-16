    #version 460
    #extension GL_EXT_ray_tracing : require
    #extension GL_EXT_nonuniform_qualifier : enable

    struct hitPayload
    {
        vec4 hit_pos;
        vec4 hit_value;
        bool hit;
    };

    layout(location = 0) rayPayloadInEXT hitPayload hitdata;
    hitAttributeEXT vec3 attribs;

    void main()
    {
        hitdata.hit_pos = vec4(gl_WorldRayOriginEXT + (gl_WorldRayDirectionEXT * gl_HitTEXT),1.0);
        hitdata.hit_value = vec4(0.2, 0.5, 0.5,1.0);
        hitdata.hit = true;
    }
