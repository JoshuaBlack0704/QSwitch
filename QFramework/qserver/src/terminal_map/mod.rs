use std::{collections::HashMap, sync::Arc, mem::size_of, net::SocketAddr};

use tokio::sync::RwLock;
use rand::{self, thread_rng, Rng};


use super::{TerminalMap, SocketPacket, TerminalConnection, SocketHandler, MAX_MESSAGE_LENGTH};

#[repr(C)]
#[derive(Clone)]
///Will be auto pasted onto any and all message fragments sent
pub struct MessageExchangeHeader{
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
    pub fn new(socket: SocketHandler, discoverable: bool) -> Arc<TerminalMap> {
        Arc::new(TerminalMap{ 
            active_connections: RwLock::new(HashMap::new()), 
            message_map: RwLock::new(HashMap::new()),
            socket,
            discoverable, })
    }
    /// If present, get a pre-existing terminal connection. Adding one if no pre-existing are found
    async fn add_get_terminal(terminal_map: Arc<Self>, terminal_addr: SocketAddr) -> Arc<TerminalConnection> {
        {
            // First we try to read a pre-exising terminal map
            let reader = terminal_map.active_connections.read().await;
            if let Some(terminal) = reader.get(&terminal_addr){
                return terminal.clone();
            }
        }
        
        // If no pre-exising termnials are found we grab a writer and add a new one
        let mut writer = terminal_map.active_connections.write().await;
        // A terminal may have been added since we dropped the reader
        if let Some(terminal) = writer.get(&terminal_addr){
            return terminal.clone();
        }
        
        let terminal = TerminalConnection::new(terminal_addr, terminal_map.socket.clone(), terminal_map.clone(), terminal_map.discoverable);
        
        if let None = writer.insert(terminal_addr, terminal.clone()){
            println!("Adding pre-exisiting terminal");
        }
        
        terminal
    }
}

impl TerminalConnection{
    /// Will process a live single or multi-fragment message from an external source
    pub async fn process_message(terminal_map: Arc<TerminalMap>, packet: SocketPacket){
        // The first step is to pull out the Message Exchange Header and get the target TerminalConnection
        let header = Self::pull_header::<MessageExchangeHeader>(&packet);
        let terminal = TerminalMap::add_get_terminal(terminal_map.clone(), packet.1).await;
        
        //Now we will check if this is a one fragment message
        if header.total_fragments == 1{
            println!("Received message with id {}", header.message_id);
            Self::receive_message(terminal, packet.2[size_of::<MessageExchangeHeader>()..packet.2.len()].to_vec()).await;
        }
        
    }
    pub async fn send_message(terminal: Arc<TerminalConnection>, message: Vec<u8>){
        let header = MessageExchangeHeader{ 
            message_id: thread_rng().gen::<u64>(),
            total_fragments: 1,
            fragment_id: 0,
            fragment_data: message.len() as u32,
            nak: false };
        
        let mut fragment:[u8;MAX_MESSAGE_LENGTH] = [0; MAX_MESSAGE_LENGTH];
        
        Self::apply_header(header, fragment.as_mut_slice());
        let data = &mut fragment[size_of::<MessageExchangeHeader>()..MAX_MESSAGE_LENGTH];
        
        let index = 0;
        while index < data.len() && index < message.len(){
            data[index] = message[index];
        }
        
    }
    async fn receive_message(terminal: Arc<TerminalConnection>, message: Vec<u8>){}
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
