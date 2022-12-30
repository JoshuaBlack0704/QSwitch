struct VSInput{
    [[vk::location(0)]] float3 pos: POSITION0;
    [[vk::location(1)]] float3 color: COLOR0;
};

struct UBO{
    float4x4 projection;
};

[[vk::binding(0,0)]]
cbuffer ubo {UBO ubo;}

struct VSOutput{
    float4 pos : SV_POSITION;
    [[vk::location(0)]] float3 color: COLOR0;
};

VSOutput main(VSInput input){
    VSOutput  output = (VSOutput)0;
    output.color = input.color;
    output.pos = float4(input.pos.xyz, 1.0);
    return output;
}