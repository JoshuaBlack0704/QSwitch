use std::{collections::HashMap, sync::Arc, mem::size_of};

use tokio::sync::RwLock;


use super::{TerminalMap, SocketPacket, TerminalConnection};

#[repr(C)]
#[derive(Clone)]
///Will be auto pasted onto any and all message fragments sent
struct MessageExchangeHeader{
    /// A randomly generated value that will be used by both the sender and reciever to identify
    /// live message exchanges
    message_id: u64,
    /// The total number of fragements this message is comprised of
    total_fragments: u32,
    /// This fragments index in the array of split fragments
    fragment_id: u32,
    /// The number of bytes this fragment contains
    fragment_data: u32,
    /// Is this message reliable
    nak: bool
}

impl TerminalMap{
    
    ///Creates a new terminal map
    pub fn new(discoverable: bool) -> Arc<TerminalMap> {
        Arc::new(TerminalMap{ 
            active_connections: RwLock::new(HashMap::new()), 
            message_map: RwLock::new(HashMap::new()),
            discoverable })
    }
    async fn add_get(terminal_map: Arc<Self>){
        
    }
}

impl TerminalConnection{
    /// Will process a live single or multi-fragment message from an external source
    pub async fn process_message(terminal_map: Arc<TerminalMap>, packet: SocketPacket){
        // The first step is to pull out the Message Exchange Header and get the target TerminalConnection
        let header = Self::pull_header::<MessageExchangeHeader>(&packet);
        
        //Now we will check if this is a one fragment message
        if header.total_fragments == 1{
            
        }
        
    }
    pub async fn send_message(){}
    pub fn pull_header<T: Clone>(packet: &SocketPacket) -> T {
        let data = &packet.2 as *const u8;
        unsafe{std::slice::from_raw_parts(data as *const T, 1)[0].clone()}
    }
    pub fn apply_header<T:Clone>(header: T, data: &mut [u8]){
        //Assuming enough data has been left for the header
        let header = &header as *const T;
        let bytes = unsafe{std::slice::from_raw_parts(header as *const u8, size_of::<T>())};
        assert!(data.len() >= bytes.len());
        
        for (index, byte) in bytes.iter().enumerate(){
            data[index] = *byte;
        }
    }
        
}
