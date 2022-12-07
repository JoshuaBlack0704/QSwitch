use std::{sync::Arc, mem::size_of, net::SocketAddr};
use tokio::time::{Duration, timeout};

use flume::RecvError;
use rand::{thread_rng, Rng};

use crate::{LocalServer, SocketPacket, MAX_MESSAGE_LENGTH, Serializable, Station};
pub(crate) type Fragment = (usize, [u8; MAX_MESSAGE_LENGTH]);
pub(crate) type Message = Vec<u8>;

#[repr(C)]
#[derive(Clone)]
///Will be auto pasted onto any and all message fragments sent
pub(crate) struct MessageExchangeHeader{
    /// A randomly generated value that will be used by both the sender and reciever to identify
    /// live message exchanges
    exchange_id: u64,
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
pub(crate) enum MessageExchangeError{
    NoConfirmation,
    Failed
}

#[derive(Clone)]
pub(crate) enum MessageOp{
    Send(SocketAddr, bool, Message),
    Receive(SocketPacket),
}

const SEND_TIMEOUT_TIME:u64= 100;
const SEND_TIMEOUT_CYCLES:usize = 10;
const RECEIVE_TIMEOUT_TIME:u64= 16;
const RECEIVE_TIMEOUT_CYCLES:usize = 10;
const MESSAGE_COMPLETE_TIMEOUT:u64= 1000;

/// The Message exchange functionality using the station analogy
impl LocalServer{
    /// This the two-way async process for sending and reciving messages
    /// This function will operate as both the send and receive.
    /// In the case of send it will send a set of fragments and then
    /// become a task and listen for any naks if required
    /// In the case of receive it will become a task and listen for receives
    /// In the case that a message_exchange for a particular message_id is already open it will
    /// just send a message to that message_exchange's channel and exit
    
    /// The ack protocol is as follows:
    /// A message needs to be sent, so a new Send message_exchange is spun up
    /// The send message exhange then blasts the whole message at once to the target
    /// The send side then listens for a message complete or retransmit message from the receiver
    /// If no message is received after some timeout the send side will request an update
    /// If the send side does not hear back from the receiver after so many cycles of it's update branch
    /// it will stop
    /// When a upd_listener gets a packet on its socket it will immidiately launch this task at the receive branch
    /// If a channel for the received packet's message_id already exists we just send it along the channel.
    /// NOTE This channel might have been created by the send side which means the receive side take care of it for the send branch
    /// If the channel does not exist the there MUST not be a send task or receive task so the the running task becomes the new 
    /// receive task and creates a channel
    /// The receive task will then start listening on the channel for another fragment
    /// If the total number of required fragments have not been received and we have reached
    /// some timeout value then the task will create re-transmit headers for all of the missing fragments
    /// and send back to the send side.
    /// If too many timout branches are reached then the message is dropped
    /// Once all of the fragments have been received the receive side will start a message processing
    /// task, passing ownership to the new task.
    /// Then it will send a message complete header to the send side and wait for awhile to make sure the send side 
    /// does not request for an update, which if it does the task will resend the message complete
    
    /// If reliability is not required then the send side will blast all of the fragments and then exit
    /// the recieve side will listen for new packets, but if a timeout is reached and
    /// it does not have all of the fragments then the message is dropped.
    /// When the receive side gets the last requiered fragment it will immediately start
    /// the messsage processing task and then exit since it does not need to notify the 
    /// send side
    
    pub(crate) async fn exchange(server: Arc<LocalServer>, operation: MessageOp) -> Result<bool, MessageExchangeError>{
        // Operation has to cases: Send, Receive
        match operation{
            MessageOp::Send(addr, mut nak, message) => {
                // The first thing we do in send is generate the exchange's id
                let exchange_id = thread_rng().gen::<u64>();
                // println!("Starting send request with id {}", exchange_id);
                if addr == server.local_address(){
                    // We are sending messages over the loopback, we will dont need to nak
                    // Doing so would produce a conflict of live state due to the send case
                    // and receive case using the same channel
                    nak = false;
                }
                // Then we must break our message into fragments
                let fragements = Self::message_to_fragments(exchange_id, nak, &message);
                // Now we just need to send all of our data
                for fragment in fragements.iter(){
                    server.send(addr, &fragment.1[0..fragment.0]).await;
                }
                // With all of our data in transit, we enter nak space
                // If we dont have a requested nak we can exit here
                if !nak{
                    return Ok(true); 
                }
                // However, if we do have a nak then we must listen for retransmit requests
                // This firstly involves creating an exchange entry in the servers exchange map
                let channel = Arc::new(flume::unbounded());
                {
                    // We use a scope here to drop the writer
                    let mut writer = server.write_exchanges().await;
                    if let Some(_) = writer.insert(exchange_id, channel.clone()){
                        // We should only ever create one entry with a given id
                        println!("We have duplicated an exchange entry when setting up a send operation")
                    }
                }
                println!("Created new active exchange from id {}", exchange_id);
                
                // Now we wait for any retransmit requests
                let mut timeout_budget = SEND_TIMEOUT_CYCLES;
                loop{
                    if let Ok(packet) = timeout(Duration::from_millis(SEND_TIMEOUT_TIME), channel.1.recv_async()).await{
                        if let Ok(packet) = packet{
                            if Self::retransmit_request(server.clone() ,exchange_id, &fragements, packet).await {
                                // Now that the exchange is complete we can remove it from existence
                                server.remove_exchange(exchange_id).await;
                                return Ok(true);
                            }
                        }
                        
                    }
                    else{
                        timeout_budget -= 1;
                        // We timeout enough times we consider the message status as unknown
                        if timeout_budget == 0{
                            // Now that the exchange is complete we can remove it from existence
                            server.remove_exchange(exchange_id).await;
                            return Err(MessageExchangeError::NoConfirmation);
                        }
                        // However, we will attempt to contact the receive side and ask for an update
                        let header:[u8; size_of::<MessageExchangeHeader>()] = MessageExchangeHeader::message_complete(exchange_id, nak).into();
                        server.send(addr, &header).await;
                    }
                }
            },
            MessageOp::Receive(packet) => {
                // When we receive a packet from the main upd listener
                // This is the branch that will be called.
                // It is responsible for routing messages to either an active 
                // sender for receiver. If there is none, it will become an active 
                // receiver
                
                
                // Our first step is to try to route the packet
                let header = MessageExchangeHeader::from_bytes(&packet.2);
                // println!("Received message for exchange {}", header.exchange_id);
                let (new, channel) = server.get_or_add_exchange(header.exchange_id).await;
                // We can go ahead and push the message on the channel cause no matter what
                // it will be handeled. By this task or an already running one
                let _ = channel.0.send(packet);
                if !new{
                    // If all that was needed was rounting then we can go ahead and exit
                    return Ok(true);
                }
                
                // Since we have a header, we know the message structure which we can prepare
                // memory for
                let mut fragments:Vec<Option<Fragment>> = vec![None; header.fragment_count as usize];
                let mut remaining_timeouts = RECEIVE_TIMEOUT_CYCLES;
                
                
                // Now, we can begin the receive operation and begin to peice the message together
                loop{
                    if let Ok(packet) = timeout(Duration::from_millis(RECEIVE_TIMEOUT_TIME), channel.1.recv_async()).await{
                        if let Ok(packet) = packet{
                            if Self::receive_fragment(server.clone(), header.exchange_id, packet, &mut fragments, channel.clone()).await{
                                // Now that the exchange is complete we can remove it from existence
                                server.remove_exchange(header.exchange_id).await;
                                return Ok(true);
                            }
                        }
                    }
                    else{
                        // If we have nak, we need to request retransmits
                        if header.nak{
                            for request in Self::prepare_retransmits(header.exchange_id, &fragments).iter(){
                                server.send(packet.1, request.as_slice()).await;
                            }
                        }
                        
                        // If we timeout too many times then we drop the message
                        remaining_timeouts -= 1;
                        println!("Receive timeout budget {}", remaining_timeouts);
                        if remaining_timeouts <= 0{
                            // Now that the exchange is complete we can remove it from existence
                            server.remove_exchange(header.exchange_id).await;
                            return Err(MessageExchangeError::Failed);
                        }
                        
                    }
                }
            },
        }
    }
    async fn receive_fragment(server: Arc<LocalServer>, exchange_id: u64, packet: SocketPacket, fragments: &mut [Option<Fragment>], channel: Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>) -> bool{
        // The receive case can get two message types: A fragment or an update request
        // A fragment is the send case sending the message data
        // An update request is the send case asking what the current state of the receive case is
        // and if it either needs more data or is complete
        
        // So lets handle both cases
        // If the channel has as error will will tell the recive case to close
        let header = MessageExchangeHeader::from_bytes(&packet.2);
        // We need to see what type of message this is
        if header.message_complete{
            println!("Receive side for exchange {} asked for state update", exchange_id);
            // Remember, if the send side sends a message_complete then it is asking for a state update
            // So we send any retransmits we have
            for request in Self::prepare_retransmits(exchange_id, fragments).iter(){
                server.send(packet.1, request.as_slice()).await;
            }
            return false;
        }
        
        // If this is not a state update than this is a new fragement
        // If we get a duplicate fragment then we just overwrite what we already have 
        let index = header.fragment_index;
        // println!("Receive side for exchange {} got fragment {} of {}", exchange_id, index + 1, header.fragment_count);
        
        fragments[index as usize] = Some((header.fragment_data as usize, packet.2));
        // Now that we have gotten a new fragment we should check to see if we need to
        // enter the message complete stage of the receive case
        // In this stage we will package the message and send it off
        // as well as notifiy the send case of completion and wait for any update requests it might send
        
        let requests = Self::prepare_retransmits(exchange_id, fragments);
        if requests.len() == 0 {
            // We have the complete message
            // This means we can peice the message together
            if let Ok(message) = Self::fragments_to_message(fragments){
                // println!("Exchange {} assembled", exchange_id);
                // And send it off
                tokio::spawn(Station::route_message(server.clone(), message));
                
                // Now we wait for awhile and respond to any send case communication with a message complete
                // If we have nak of course
                if header.nak{
                    loop{
                        match timeout(Duration::from_millis(MESSAGE_COMPLETE_TIMEOUT), channel.1.recv_async()).await{
                            Ok(_) => {
                                let header:[u8; size_of::<MessageExchangeHeader>()] = MessageExchangeHeader::message_complete(exchange_id, true).into();
                                server.send(packet.1, &header).await;
                            },
                            Err(_) => {
                                // If we have waited long enough we will assume that the send case has closed
                                return true;
                            },
                        }
                    }
                }
                // If no nak then we just consider this exchange complete
                return true;
            }
            else{
                // This should be an impossible case
                return true;
            }
        }
        return false;
    }
    fn fragments_to_message(fragments: &[Option<Fragment>]) -> Result<Vec<u8>, MessageExchangeError>{
        let mut message = Vec::with_capacity(fragments.len() * MAX_MESSAGE_LENGTH);
        for fragment in fragments{
            if let Some((len, data)) = fragment{
                // Note we dont look at the beginning of the data because the exchange header is still in there
                message.extend_from_slice(&data[size_of::<MessageExchangeHeader>()..len + size_of::<MessageExchangeHeader>()]);
            }
            else{
                // This is a critical flaw in the program design
                return Err(MessageExchangeError::Failed);
            }
        }
        Ok(message)
    }
    
    fn prepare_retransmits(message_id: u64, fragments: &[Option<Fragment>]) -> Vec<[u8; size_of::<MessageExchangeHeader>()]> {
            let mut headers = Vec::with_capacity(fragments.len());
            for (index, fragment) in fragments.iter().enumerate(){
                if let None = fragment{
                    let header = MessageExchangeHeader{ 
                        exchange_id: message_id,
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
     /// This function works on the send case and will process any message the send case receives
    /// It returns a bool which signifies if the send case can shutdown
    async fn retransmit_request(server: Arc<LocalServer>, exchange_id: u64, fragments: &Vec<Fragment>, packet: SocketPacket) -> bool{
        // The send case can get either a retransmit request or a message complete
        // message
        // The former specifies what fragment to resend, the latter is technically optional
        // and lets the send side know it can close
        let header = MessageExchangeHeader::from_bytes(&packet.2);
        if header.exchange_id != exchange_id{
            println!("Message from exchange {} landed in exchange {}", header.exchange_id, exchange_id);
            return false;
        }
        // If the message complete flag is on in the send case channel that idicated the receive side has
        // all of the element
        if header.message_complete{
            return true;
        }
        
        // If not, then this is a retransmit request and we must send the requested fragment
        // Not the receive side will send back the index it needs
        let requested_fragment = &fragments[header.fragment_index as usize];
        let requested_data = &requested_fragment.1[0..requested_fragment.0];
        server.send(packet.1, requested_data).await;
        
        return false;
    }
    fn message_to_fragments(exchange_id: u64, nak:bool, message: &Message) -> Vec<Fragment> {
        let data_size = MAX_MESSAGE_LENGTH - size_of::<MessageExchangeHeader>();
        let mut fragments:Vec<Fragment> = Vec::with_capacity(message.len()/data_size + 1);
        let chunks = message.chunks(data_size);
        let total_chunks = chunks.len() as u32;
        
        for (index, chunk) in chunks.enumerate(){
            let header = MessageExchangeHeader{ 
                exchange_id,
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




impl MessageExchangeHeader{
    fn message_complete(message_id: u64, nak: bool) -> MessageExchangeHeader {
        MessageExchangeHeader{ 
        exchange_id: message_id,
        fragment_count: 0,
        fragment_index: 0,
        fragment_data: 0,
        nak,
        message_complete: true }
    }
}
    
 
impl LocalServer{
    async fn add_unique_exchange(&self, exchange_id: u64) -> Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)> {
        let mut exchanges = self.write_exchanges().await;
        let channel = Arc::new(flume::unbounded());
        if let Some(_) = exchanges.insert(exchange_id, channel.clone()){
            panic!("Message exchange is not unique");
        }
        channel
        
    }
    async fn remove_exchange(&self, exchange_id: u64){
        let mut exchange = self.write_exchanges().await;
        exchange.remove(&exchange_id);
    }
    /// This function returns if we had to add the exchange
    /// True: new exchange added
    /// False: retrived pre existing
    async fn get_or_add_exchange(&self, exchange_id: u64) -> (bool, Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>) {
        // Here we need to notify the caller if we added or got the exchange channel
        
        {
            // First we try to read a pre-exising terminal map
            let reader = self.read_exchanges().await;
            if let Some(message_channel) = reader.get(&exchange_id){
                return (false, message_channel.clone());
            }
        }
        
        // If no pre-exising termnials are found we grab a writer and add a new one
        let mut writer = self.write_exchanges().await;
        // A terminal may have been added since we dropped the reader
        if let Some(message_channel) = writer.get(&exchange_id){
            return (false, message_channel.clone());
        }
        
        let message_channel = Arc::new(flume::unbounded());
        
        if let Some(_) = writer.insert(exchange_id, message_channel.clone()){
            println!("Adding pre-exisiting message_id");
        }
        
        (true, message_channel)
        
    }
}