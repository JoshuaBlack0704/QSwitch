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
    fragment_count: u32,
    /// This fragments index in the array of split fragments
    fragment_index: u32,
    /// The number of bytes this fragment contains
    fragment_data: u32,
    /// Is this message reliable
    nak: bool,
    /// This is send by the receiver in case they need a rebroadcast
    message_complete: bool,
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
                                    if header.message_complete {
                                        break;
                                    }
                                    else{
                                        let needed_fragment = &fragments[header.fragment_index as usize];
                                        terminal.socket.send(terminal.tgt_addr, &needed_fragment.1[0..needed_fragment.0]).await;
                                    }
                                }
                                else{
                                    println!("Unexpected channel close");
                                    break;
                                }
                            
                            }
                            _ = sleep(Duration::from_millis(time_out)) => {
                                // This branch will cancel and restart if we get a nak retrasmit request
                                break;
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
                let _ = message_channel.0.send(packet);
                if !is_first{
                    // This channel might have been created by the send case or receive case
                    // Once we have sent the packet we can exit this task
                    return;
                }
                // If we are here we will begin the receive routine
                let mut fragment_check = vec![false; header.fragment_count as usize]; 
                let mut fragments:Vec<Fragment> = Vec::with_capacity(header.fragment_count as usize);
                // The amount of time we will wait for a packet from the send side before doing a nak retransmit pass
                let nak_timeout = 16;
                // The amount of times we will try to contact an unrespondive send side
                let mut nak_attemts = 5;
                
                loop{
                    tokio::select!{
                        val = message_channel.1.recv_async()=>{
                            if let Ok(packet) = val{
                                if header.nak{
                                    if Self::nak_packet_process(live_state.clone(), packet, &mut fragment_check, &mut fragments).await{
                                        // If we are here we have recieved the last fragment in the message
                                    }
                                }
                                else{
                                    if Self::packet_process(live_state.clone(), packet, &mut fragment_check, &mut fragments).await{
                                        // If we are here we have recieved the last fragment in the message
                                        
                                    }
                                }
                            }
                            else{
                                println!("Unexpected channel close");
                                break;
                            }
                        }
                        _ = sleep(Duration::from_millis(nak_timeout))=>{
                            if header.nak{
                                let terminal = LiveState::add_get_terminal(live_state.clone(), packet.1).await;
                                Self::nak_pass(terminal, packet.clone(), &mut fragment_check, &mut fragments).await;
                                nak_attemts -= 1;
                                if nak_attemts <= 0{
                                    break;
                                }
                            }
                            else{
                                // If we run out of time here without a completed message then we just drop it
                                break;
                            }
                        }
                    }
                }
                
            },
        };
        
    }
    async fn nak_packet_process(live_state: Arc<LiveState>,packet: SocketPacket, packet_check: &mut [bool], fragments: &mut Vec<Fragment>) -> bool{
        
        let header = MessageExchangeHeader::from_bytes(&packet.2);
        if header.message_complete{
            // If the receive branch gets a header that has the message_complete flag on then it means the send side is requesting the nak
            // status of the message because it has not received a message_complete or nak message in awhile
            let terminal = LiveState::add_get_terminal(live_state.clone(), packet.1).await;
            Self::nak_pass(terminal, packet, packet_check, fragments).await;
            false
        }
        else{
            // Else, it is the send side giving us a new packet
            
            if !packet_check[header.fragment_index as usize]{
                // If this packet is not a duplicate
                packet_check[header.fragment_index as usize] = true;
                let fragment = (packet.0, packet.2);
                fragments.push(fragment);
            }
            
            // With the new packet added we check if we have all packets
            if fragments.len() == header.fragment_count as usize{
                // If we have all of the fragments then we will launch the message proccess
                let terminal = LiveState::add_get_terminal(live_state.clone(), packet.1).await;
                // We do a nak pass here to send the send side a message_complete signal
                Self::nak_pass(terminal, packet, packet_check, fragments).await;
                return true;
            }
            false
        }
        
    }
    async fn packet_process(live_state: Arc<LiveState>, packet: SocketPacket, packet_check: &mut [bool], fragments: &mut Vec<Fragment>) -> bool{
        true
    }
    async fn nak_pass(live_state: Arc<TerminalConnection>, packet: SocketPacket, packet_check: &mut [bool], fragments: &mut Vec<Fragment>){}
    fn message_to_fragments(
        message_id: u64, nak:bool, message: &Message) -> Vec<Fragment> {
        let data_size = MAX_MESSAGE_LENGTH - size_of::<MessageExchangeHeader>();
        let mut fragments:Vec<Fragment> = Vec::with_capacity(message.len()/data_size + 1);
        let chunks = message.chunks(data_size);
        let total_chunks = chunks.len() as u32;
        
        for (index, chunk) in chunks.enumerate(){
            let header = MessageExchangeHeader{ 
                message_id,
                fragment_count: total_chunks,
                fragment_index: index as u32,
                fragment_data: chunk.len() as u32,
                nak,
                message_complete: false, };
            
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
    async fn process_message(live_state: Arc<LiveState>, terminal: Arc<TerminalConnection>, fragments: Vec<Fragment>){}
}
