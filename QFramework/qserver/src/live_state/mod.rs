use std::{collections::HashMap, sync::Arc, mem::size_of, net::SocketAddr};

use tokio::{sync::RwLock, time::{sleep, Duration}};
use rand::{self, thread_rng, Rng};


use super::{LiveState, Bytable, SocketPacket, TerminalConnection, SocketHandler, MAX_MESSAGE_LENGTH, TerminateSignal};
pub(crate) type Fragment = (usize, [u8; MAX_MESSAGE_LENGTH]);
pub(crate) type Message = Vec<u8>;

#[derive(Clone)]
pub(crate) enum MessageOp{
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
    pub(crate) async fn add_get_terminal(live_state: Arc<Self>, terminal_addr: SocketAddr) -> Arc<TerminalConnection> {
        {
            // First we try to read a pre-exising terminal map
            let reader = live_state.terminals.read().await;
            if let Some(terminal) = reader.get(&terminal_addr){
                return terminal.clone();
            }
        }
        
        // If no pre-exising termnials are found we grab a writer and add a new one
        let mut writer = live_state.terminals.write().await;
        // A terminal may have been added since we dropped the reader
        if let Some(terminal) = writer.get(&terminal_addr){
            return terminal.clone();
        }
        
        let terminal = TerminalConnection::new(terminal_addr, live_state.socket.clone(), live_state.clone(), live_state.discoverable);
        
        if let Some(_) = writer.insert(terminal_addr, terminal.clone()){
            println!("Adding pre-exisiting terminal");
        }
        
        terminal
    }
    pub(crate) async fn remove_terminal(live_state: Arc<Self>, terminal_addr: SocketAddr){
        println!("Removing terminal {}", terminal_addr);
        let mut writer = live_state.terminals.write().await;
        if let None = writer.remove(&terminal_addr){
            println!("Trying to remove non existing terminal from live state");
            
        }
        
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
        
        if let Some(_) = writer.insert(message_id, message_channel.clone()){
            println!("Adding pre-exisiting message_id");
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
        
        if let Some(_) = writer.insert(message_id, message_channel.clone()){
            println!("Adding pre-exisiting message_id");
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

const SEND_TIMEOUT_TIME:u64= 100;
const SEND_TIMEOUT_CYCLES:usize = 10;
const RECEIVE_TIMEOUT_TIME:u64= 16;
const RECEIVE_TIMEOUT_CYCLES:usize = 10;
const MESSAGE_COMPLETE_TIMEOUT:u64= 1000;
impl TerminalConnection{
    /// This the two-way async process for sending and reciving messages
    pub async fn message_exchange(live_state: Arc<LiveState>, operation: MessageOp){
        
        
        // This function will operate as both the send and receive.
        // In the case of send it will send a set of fragments and then
        // become a task and listen for any naks if required
        // In the case of receive it will become a task and listen for receives
        // In the case that a message_exchange for a particular message_id is already open it will
        // just send a message to that message_exchange's channel and exit
        
        // The ack protocol is as follows:
        // A message needs to be sent, so a new Send message_exchange is spun up
        // The send message exhange then blasts the whole message at once to the target
        // The send side then listens for a message complete or retransmit message from the receiver
        // If no message is received after some timeout the send side will request an update
        // If the send side does not hear back from the receiver after so many cycles of it's update branch
        // it will stop
        // TODO: Propigate result of send op up the call stack
        // When a upd_listener gets a packet on its socket it will immidiately launch this task at the receive branch
        // If a channel for the received packet's message_id already exists we just send it along the channel.
        // NOTE This channel might have been created by the send side which means the receive side take care of it for the send branch
        // If the channel does not exist the there MUST not be a send task or receive task so the the running task becomes the new 
        // receive task and creates a channel
        // The receive task will then start listening on the channel for another fragment
        // If the total number of required fragments have not been received and we have reached
        // some timeout value then the task will create re-transmit headers for all of the missing fragments
        // and send back to the send side.
        // If too many timout branches are reached then the message is dropped
        // Once all of the fragments have been received the receive side will start a message processing
        // task, passing ownership to the new task.
        // Then it will send a message complete header to the send side and wait for awhile to make sure the send side 
        // does not request for an update, which if it does the task will resend the message complete
        
        // If reliability is not required then the send side will blast all of the fragments and then exit
        // the recieve side will listen for new packets, but if a timeout is reached and
        // it does not have all of the fragments then the message is dropped.
        // When the receive side gets the last requiered fragment it will immediately start
        // the messsage processing task and then exit since it does not need to notify the 
        // send side
        
        match operation{
            MessageOp::Send(terminal, nak, message) => {
                let message_id = thread_rng().gen::<u64>();
                println!("Received send request with message_id: {}", message_id);
                let fragments = Self::message_to_fragments(message_id, nak, &message);
                // In the send case we will always need to generate a new message_map entry
                let message_channel = LiveState::add_get_message(live_state.clone(), message_id).await;
                // Here we send the initial blast of message fragments
                for fragment in fragments.iter(){
                    terminal.socket.send(terminal.tgt_addr.clone(), &fragment.1[0..fragment.0]).await;
                }
                // If we arent nak then we can just close this send side since a receiver will never send message back
                if !nak{
                    return;
                }
                let mut cycle_budget = SEND_TIMEOUT_CYCLES;
                
                loop{
                    // Remeber, whichever branch of the select that loses get canceled
                    // so the timeout will start over if some message arrives in the channel
                    tokio::select!{
                        val = message_channel.1.recv_async()=>{
                            if let Ok(packet) = val{
                            // A message sent back from the receive branch is a retransmit request unless the message_complete
                            // bool is true
                                let (complete, retransmit_index) = Self::send_side_process(packet);
                                if !complete{
                                    // Resending the requested fragment 
                                    println!("Received retransmit request from {} for fragment {} of message {}", packet.1, retransmit_index, message_id);
                                    let retransmit_fragment = &fragments[retransmit_index];
                                    terminal.socket.send(terminal.tgt_addr.clone(), &retransmit_fragment.1[0..retransmit_fragment.0]).await;
                                    
                                }
                                else{
                                    // If the receiver side has told us it has everything then we are done
                                    println!("Message complete received for message {}", message_id);
                                    break;
                                }
                            }
                        }
                        _ = sleep(Duration::from_millis(SEND_TIMEOUT_TIME))=>{
                            // If this completes we need to ask the receive time for an update
                            // aka. If the message complete bool is true in the receive side's RECPTION
                            // the the receive side will interpret it as an update request
                            
                            println!("Asking receive side for update on message {}", message_id);
                            let header = MessageExchangeHeader{ 
                                message_id,
                                fragment_count: 0,
                                fragment_index: 0,
                                fragment_data: 0,
                                nak,
                                message_complete: true };
                            let mut data = vec![0; size_of::<MessageExchangeHeader>()];
                            header.to_bytes(data.as_mut_slice());
                            terminal.socket.send(terminal.tgt_addr.clone(), data.as_slice()).await;
                            cycle_budget -= 1;
                            if cycle_budget <= 0{
                                // If we run out of cycles we assume the tgt has become unreachable
                                break;
                            }
                        }
                    }
                }
                LiveState::remove_message(live_state.clone(), message_id).await;
                
                
            },
            MessageOp::Receive(packet) => {
                // Imediately we need to see if there is already another channel open for it
                let header = MessageExchangeHeader::from_bytes(&packet.2);
                println!("Received packet from {} for message {}", packet.1, header.message_id);
                let (is_first, message_channel) = LiveState::first_get_message(live_state.clone(), header.message_id).await;
                // This will be handeled later
                message_channel.0.send(packet).unwrap();
                if !is_first{
                    //If a different task is already handling this message_id then we can exit
                    return;
                }
                
                //We need to build the message structure
                let terminal = LiveState::add_get_terminal(live_state.clone(), packet.1).await;
                let mut fragments:Vec<Option<Fragment>> = vec![None; header.fragment_count as usize];
                let mut remaining_timeouts = RECEIVE_TIMEOUT_CYCLES;
                
                loop{
                    tokio::select!{
                        val = message_channel.1.recv_async()=>{
                            // If a receive channel gets a packet it can either be a new fragment or an
                            // update request from a send side timeout
                            if let Ok(packet) = val{
                                let header = MessageExchangeHeader::from_bytes(&packet.2);
                                if header.nak{
                                    if header.message_complete{
                                        // Remeber, message complete send from the send side means it is requestin an update
                                        // which would be either a retransmit request or message complete which would be handled by a
                                        // seperate task
                                        for request in Self::prepare_retransmits(header.message_id, &fragments).iter(){
                                            terminal.socket.send(terminal.tgt_addr, request.as_slice()).await;
                                        }
                                    }
                                    else{
                                        // If we got a new fragment we add it to the fragment vector
                                        // if this fragmest is a duplicate we just override the exising fragment
                                        fragments[header.fragment_index as usize] = Some((packet.0, packet.2));
                                    }
                                }
                                // If we dont have nak then the send function will never send a message complete
                                // this means any message we get will be a new fragment
                                else{
                                    // If we got a new fragment we add it to the fragment vector
                                    // if this fragmest is a duplicate we just override the exising fragment
                                    fragments[header.fragment_index as usize] = Some((packet.0, packet.2));
                                }
                                
                                // Now that we may have received a new packet we must check to see if we have all the packets
                                // if we don't no problem. If we do we need to start the message procceser.
                                // If we have nak we need to send message complete by starting the receive complete task
                                let still_needed = Self::prepare_retransmits(header.message_id, &fragments);
                                if still_needed.len() == 0{
                                    // We have completed the message
                                    // Now we need to pass on the final message
                                    tokio::spawn(Self::process_message(live_state.clone(), terminal.clone(), fragments));
                                    if header.nak{
                                        Self::message_complete(header.message_id, live_state.clone(), terminal.clone(), message_channel).await;
                                    }
                                    break;
                                }
                            }
                        }
                        _ = sleep(Duration::from_millis(RECEIVE_TIMEOUT_TIME))=>{
                            // If we have nak, we need to request retransmits
                            if header.nak{
                                for request in Self::prepare_retransmits(header.message_id, &fragments).iter(){
                                    terminal.socket.send(terminal.tgt_addr, request.as_slice()).await;
                                }
                            }
                            
                            // If we timeout too many times then we drop the message
                            remaining_timeouts -= 1;
                            if remaining_timeouts <= 0{
                                break;
                            }
                        }
                    }
                    
                }
                
                LiveState::remove_message(live_state.clone(), header.message_id).await;
                
            },
        };
        
    }
    async fn message_complete(message_id: u64, live_state: Arc<LiveState>, terminal: Arc<TerminalConnection>, channel: Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>){
        // To complete the message we must send the message_complete header and 
        // then wait to make sure the sender got it
        let header = MessageExchangeHeader{ 
            message_id,
            fragment_count: 0,
            fragment_index: 0,
            fragment_data: 0,
            nak: true,
            message_complete: true };
        let mut data = vec![0u8; size_of::<MessageExchangeHeader>()];
        header.to_bytes(data.as_mut_slice());
        terminal.socket.send(terminal.tgt_addr, &data).await;
        loop{
            // We just need to listen for any communication from the send side and resend our message_complete if we get any
            tokio::select!{
                val = channel.1.recv_async()=>{
                    terminal.socket.send(terminal.tgt_addr, &data).await;
                }
                _ = sleep(Duration::from_millis(MESSAGE_COMPLETE_TIMEOUT))=>{
                    // If we dont get any messages from the send side for this long we assume we are done
                    break;
                }
            }
        }
        
        
        
    }
    fn send_side_process(packet: SocketPacket) -> (bool, usize) {
        let header = MessageExchangeHeader::from_bytes(&packet.2);
        let complete = header.message_complete;
        let fragment_index = header.fragment_index as usize;
        (complete, fragment_index)
    }
    fn prepare_retransmits(message_id: u64, fragments: &[Option<Fragment>]) -> Vec<[u8; size_of::<MessageExchangeHeader>()]> {
        let mut headers = Vec::with_capacity(fragments.len());
        for (index, fragment) in fragments.iter().enumerate(){
            if let None = fragment{
                let header = MessageExchangeHeader{ 
                    message_id,
                    fragment_count: 0,
                    fragment_index: index as u32,
                    fragment_data: 0,
                    nak: true,
                    message_complete: false };
                let mut header_data = [0u8; size_of::<MessageExchangeHeader>()];
                header.to_bytes(&mut header_data);
                headers.push(header_data);
            }
        }
        headers
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
    fn message_to_fragments(message_id: u64, nak:bool, message: &Message) -> Vec<Fragment> {
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
    
}
