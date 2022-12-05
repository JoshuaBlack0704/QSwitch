use std::sync::Arc;

use crate::{Station, LocalServer, Bytable};

pub(crate) const NO_MESSAGE_CHANNEL:u32 = u32::MAX;
pub(crate) type StationId = u64;
pub(crate) type StationChannel = u32;
#[derive(Clone)]
#[repr(C)]
pub(crate) struct StationHeader{
    from_id: StationId,
    to_id: StationId,
    channel: StationChannel,
}

impl Station{
    pub(crate) async fn route_message(server: Arc<LocalServer>, message: Vec<u8>){
        let header = StationHeader::from_bytes(&message);
        if header.channel == NO_MESSAGE_CHANNEL{
            return;
        }
    }
}
impl StationHeader{
    pub(crate) fn no_message() -> StationHeader {
        StationHeader{ 
            from_id: 0,
            to_id: 0,
            channel: NO_MESSAGE_CHANNEL }
    }
}
                
                
                
