    #version 460
    #extension GL_EXT_ray_tracing : require

    layout(location = 1) rayPayloadInEXT bool shadowed;

    void main()
    {
        hitdata.hit_value = vec4(0.0, 0.1, 0.3, 1.0);
        hitdata.hit = false;
    }

