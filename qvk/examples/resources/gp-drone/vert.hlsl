struct VSInput{
    [[vk::location(0)]] float3 pos : POSITION0;
    [[vk::location(1)]] float3 norm : NORMAL0;
    [[vk::location(2)]] float3 color : COLOR0;
};

struct VSOutput{
    float4 pos : SV_POSITION;
    [[vk::location(0)]] float3 normal : NORMAL0;
    [[vk::location(1)]] float3 color: COLOR0;
    [[vk::location(2)]] float3 l_dir: POSITION0;
};

struct UBO{
    float4x4 projection;
    float4x4 view;
    uint object_count;
    uint target_count;
    float delta_time;
    uint frame;
};

struct InstanceData{
    float4x4 mvp_matrix;
    float4 target_pos;
    float4 current_pos;
};

[[vk::binding(0,0)]]
cbuffer ubo {UBO ubo;}

[[vk::binding(1,0)]]
RWStructuredBuffer<InstanceData> i_data;

VSOutput main(VSInput input, int id : SV_INSTANCEID){
    float3 light_pos = float3(-1,1,-1);
    float3 light_dir = light_pos - input.pos;
    light_dir = normalize(light_dir);


    VSOutput  output = (VSOutput)0;
    output.color = input.color;
    output.normal = input.norm;
    output.l_dir = light_dir;
    output.pos = mul(i_data[id].mvp_matrix, float4(input.pos.xyz, 1.0));
    //output.pos = mul(ubo.projection, mul(ubo.view, float4(input.pos.xyz, 1.0)));
    return output;
}