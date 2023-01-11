struct VSInput{
    [[vk::location(0)]] float3 pos : POSITION0;
    [[vk::location(1)]] float3 norm : NORMAL0;
    [[vk::location(2)]] float3 color : COLOR0;
};

struct UBO{
    float4x4 projection;
    float4x4 view;
};

[[vk::binding(0,0)]]
cbuffer ubo {UBO ubo;}

struct VSOutput{
    float4 pos : SV_POSITION;
    [[vk::location(0)]] float3 color: COLOR0;
};

VSOutput main(VSInput input, int id : SV_INSTANCEID){
    float3 light_pos = float3(-1,1,-1);
    float3 light_dir = light_pos - input.pos;
    light_dir = normalize(light_dir);
    float factor = clamp(dot(input.norm, light_dir), 0.1, 1);


    VSOutput  output = (VSOutput)0;
    output.color = input.color * factor;
    output.pos = mul(ubo.projection, mul(ubo.view, float4(input.pos.xyz, 1.0)));
    return output;
}