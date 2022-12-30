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

float3 verticies[3] = {
    float3(0.5, 0.0, 0.0),
    float3(0.0, 0.5, 0.0),
    float3(-0.5, 0.0, 0.0),
};

VSOutput main(VSInput input, uint index : SV_VERTEXID){
    VSOutput  output = (VSOutput)0;
    output.color = float3(1.0,1.0,1.0);

    output.pos = float4(verticies[index].xyz, 1.0);
    return output;
}