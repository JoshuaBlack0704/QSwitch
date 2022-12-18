struct UBO
{
	uint count;
};

[[vk::binding(1,0)]]
cbuffer ubo
{
	UBO ubo;
}


[[vk::binding(0,0)]]
RWStructuredBuffer<uint> values;

[numthreads(16,1,1)]
void main(uint3 GlobalInvocationID : SV_DispatchThreadID, uint3 LocalInvocationID : SV_GroupThreadID)
{
	uint index = GlobalInvocationID.x;
	if (index > ubo.count)
	{
		return;
	}
	values[GlobalInvocationID.x] += 1;
}