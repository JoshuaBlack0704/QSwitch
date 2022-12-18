[[vk::binding(0,0)]]
RWBuffer<uint> data;

struct UBO
{
	uint count;
};

[[vk::binding(0,1)]]
cbuffer ubo
{
	UBO ubo;
}

[numthreads(16)]
void main(uint3 GlobalInvocationID : SV_DispatchThreadID, uint3 LocalInvocationID : SV_GroupThreadID)
{
	uint index = GlobalInvocationID.x;
	if (index > ubo.count)
	{
		return;
	}
	data[GlobalInvocationID.x] += 1;
}