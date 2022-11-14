use std::{collections::HashMap, sync::Arc, mem::size_of, net::SocketAddr};

use tokio::{sync::RwLock, time::{sleep, Duration}};
use rand::{self, thread_rng, Rng};


use super::{LiveState, Bytable, SocketPacket, TerminalConnection, SocketHandler, MAX_MESSAGE_LENGTH};
type Fragment = (usize, [u8; MAX_MESSAGE_LENGTH]);
type Message = Vec<u8>;

enum MessageOp{
    Send(Arc<TerminalConnection>, bool, Message),
    Receive(SocketPacket),
}

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
    nak: bool,
    /// This is send by the receiver in case they need a rebroadcast
    resend: bool,
}

impl LiveState{
    
    ///Creates a new terminal map
    pub fn new(socket: SocketHandler, discoverable: bool) -> Arc<LiveState> {
        Arc::new(LiveState{ 
            terminals: RwLock::new(HashMap::new()), 
            message_map: RwLock::new(HashMap::new()),
            socket,
            discoverable, })
    }
    /// If present, get a pre-existing terminal connection. Adding one if no pre-existing are found
    async fn add_get_terminal(terminal_map: Arc<Self>, terminal_addr: SocketAddr) -> Arc<TerminalConnection> {
        {
            // First we try to read a pre-exising terminal map
            let reader = terminal_map.terminals.read().await;
            if let Some(terminal) = reader.get(&terminal_addr){
                return terminal.clone();
            }
        }
        
        // If no pre-exising termnials are found we grab a writer and add a new one
        let mut writer = terminal_map.terminals.write().await;
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
    async fn add_get_message(live_state: Arc<Self>, message_id: u64) -> Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)> {
        {
            // First we try to read a pre-exising terminal map
            let reader = live_state.message_map.read().await;
            if let Some(message_channel) = reader.get(&message_id){
                return message_channel.clone();
            }
        }
        
        // If no pre-exising termnials are found we grab a writer and add a new one
        let mut writer = live_state.message_map.write().await;
        // A terminal may have been added since we dropped the reader
        if let Some(message_channel) = writer.get(&message_id){
            return message_channel.clone();
        }
        
        let message_channel = Arc::new(flume::unbounded());
        
        if let None = writer.insert(message_id, message_channel.clone()){
            println!("Adding pre-exisiting terminal");
        }
        
        message_channel
        
    }
    /// This function will return an bool specifing if the returned channel was already created
    /// Returns (is_unique, channel)
    async fn first_get_message(live_state: Arc<LiveState>, message_id: u64) -> (bool, Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>) {
        {
            // First we try to read a pre-exising terminal map
            let reader = live_state.message_map.read().await;
            if let Some(message_channel) = reader.get(&message_id){
                return (false, message_channel.clone());
            }
        }
        
        // If no pre-exising termnials are found we grab a writer and add a new one
        let mut writer = live_state.message_map.write().await;
        // A terminal may have been added since we dropped the reader
        if let Some(message_channel) = writer.get(&message_id){
            return (false, message_channel.clone());
        }
        
        let message_channel = Arc::new(flume::unbounded());
        
        if let None = writer.insert(message_id, message_channel.clone()){
            println!("Adding pre-exisiting terminal");
        }
        
        (true, message_channel)
        
    }
    async fn remove_message(live_state: Arc<LiveState>, message_id: u64){
        let mut writer = live_state.message_map.write().await;
        writer.remove(&message_id);
    }
    async fn try_get_message(live_state: Arc<LiveState>, messaged_id: u64) -> Option<Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>> {
        let reader = live_state.message_map.read().await;
        if let Some(channel) = reader.get(&messaged_id){
            Some(channel.clone())
        }
        else{
            None
        }
    }
}


impl TerminalConnection{
    /// This the two-way async process for sending and reciving messages
    async fn message_exchange(live_state: Arc<LiveState>, operation: MessageOp){
        
        
        // This function will operate as both the send and receive.
        // In the case of send it will send a set of fragments and then
        // become a task and listen for any naks if required
        // In the case of receive it will become a task and listen for receives
        // In the case that a message_exchange for a particular message_id is already open it will
        // just send a message to that message_exchange's channel and exit
        
        match operation{
            MessageOp::Send(terminal, nak, message) => {
                let message_id = thread_rng().gen::<u64>();
                let fragments = Self::message_to_fragments(message_id, nak, &message);
                
                // In the send case we will always need to generate a new message_map entry
                let message_channel = LiveState::add_get_message(live_state.clone(), message_id).await;
                // Here we send the initial blast of message fragments
                for fragment in fragments.iter(){
                    terminal.socket.send(terminal.tgt_addr.clone(), &fragment.1[0..fragment.0]).await;
                }
                
                // If we did not request an nak then we are done
                if nak {
                    // To support nak we will need to start listening for receptions on the channel
                    // We also need to keep time, as with nak the target could not send a response because they got the message
                    // So we need to wait for a decent time for a response
                    let mut accumulator = 0;
                    // We will wait for one whole second
                    let time_out = 1000;
                    
                    loop {
                        tokio::select!{
                            // A message channel created by the send case will always lead here when sent from the main udp listener
                            // thus specializing this function
                            val = message_channel.1.recv_async()=>{
                                if let Ok(packet) = val{
                                    // If we get a packet delivered to a send case it will be to request a re-transmit
                                    let header = MessageExchangeHeader::from_bytes(packet.2.as_slice());
                                    let needed_fragment = &fragments[header.fragment_id as usize];
                                    terminal.socket.send(terminal.tgt_addr, &needed_fragment.1[0..needed_fragment.0]).await;
                                }
                                accumulator = 0;
                            
                            }
                            _ = sleep(Duration::from_millis(100)) => {
                                // We periodically add to the timer
                                // We the timer is too high we shut down the send
                                accumulator += 100;
                                if accumulator > time_out{
                                    break;
                                }
                            }
                        }
                    }
                }
                
                // The send process is now completed
                // Now we must destroy the active message_exchange Hashmap entry
                LiveState::remove_message(live_state.clone(), message_id).await;
                
            },
            MessageOp::Receive(packet) => {
                let header = MessageExchangeHeader::from_bytes(&packet.2);
                
                let (is_first, message_channel) = LiveState::first_get_message(live_state.clone(), header.message_id).await;
                if !is_first{
                    // This channel might have been created by the send case or receive case
                    let _ = message_channel.0.send(packet);
                    // Once we have sent the packet we can exit this task
                    return;
                }
                // TODO: we need to handle the one fragement case
                
                // If we are here we will begin the receive routine
                let 
                loop {
                    
                }
                
            },
        };
        
    }
    fn message_to_fragments(message_id: u64, nak:bool, message: &Message) -> Vec<Fragment> {
        let data_size = MAX_MESSAGE_LENGTH - size_of::<MessageExchangeHeader>();
        let mut fragments:Vec<Fragment> = Vec::with_capacity(message.len()/data_size + 1);
        let chunks = message.chunks(data_size);
        let total_chunks = chunks.len() as u32;
        
        for (index, chunk) in chunks.enumerate(){
            let header = MessageExchangeHeader{ 
                message_id,
                total_fragments: total_chunks,
                fragment_id: index as u32,
                fragment_data: chunk.len() as u32,
                nak,
                resend: false, };
            
            let mut fragment = (size_of::<MessageExchangeHeader>() + chunk.len(), [0; MAX_MESSAGE_LENGTH]);
            let header_space = &mut fragment.1[0..size_of::<MessageExchangeHeader>()];
            header.to_bytes(header_space);
            let data_space = &mut fragment.1[size_of::<MessageExchangeHeader>()..];
            for (index, byte) in chunk.iter().enumerate(){
                data_space[index] = *byte;
            }
            fragments.push(fragment);
        }
        
        fragments
        
    }
    // fn fragments_to_message(fragments: Vec<Fragment>) -> Message{
        
    // }
    
        
}
