    #version 460
    #extension GL_EXT_ray_tracing : require

    struct hitPayload
    {
        vec4 hit_pos;
        vec4 hit_value;
        bool hit;
    };

    layout(location = 0) rayPayloadInEXT hitPayload hitdata;

    void main()
    {
        hitdata.hit_value = vec4(0.0, 0.1, 0.3, 1.0);
        hitdata.hit = false;
        
    }


