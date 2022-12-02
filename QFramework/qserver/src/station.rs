pub(crate) type StationId = u64;
pub(crate) type StationChannel = u32;
#[derive(Clone)]
#[repr(C)]
pub(crate) struct StationHeader{
    from_id: StationId,
    to_id: StationId,
    channel: StationChannel,
    ping: bool,
}
