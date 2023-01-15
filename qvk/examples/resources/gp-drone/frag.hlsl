struct PsInput{
  [[vk::location(0)]] float3 normal : NORMAL0;
  [[vk::location(1)]] float3 color : COLOR0;
  [[vk::location(2)]] float3 l_dir: POSITION0;
};


float4 main(PsInput input) : SV_TARGET
{
  float factor = clamp(dot(input.normal, input.l_dir), 0.1, 1);

  return float4(input.color, 1.0) * factor;
}