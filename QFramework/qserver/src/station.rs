use std::{sync::Arc, net::SocketAddr, mem::size_of, collections::{HashMap, VecDeque}};

use rand::{thread_rng, Rng};
use serde::{Serialize, Deserialize};

use crate::{Station, LocalServer, NO_MESSAGE_CHANNEL, PING_CHANNEL, StationOperable, message_exchange::MessageOp, SERVER_CHANNEL};

pub(crate) type StationId = u64;
pub(crate) type StationChannel = u32;
pub type StationReturn<T> = (SocketAddr, StationId, T);
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StationHeader{
    from_id: StationId,
    to_id: StationId,
    channel: StationChannel,
}
pub enum StationSendError{
    AckFailure,
    UnknownStation,
}
pub(crate) fn make_header(channel: StationChannel, from_id: StationId, to_id:StationId) -> StationHeader {
    StationHeader{ from_id, to_id, channel }
}
/// The entry point for station messages. Is used from a receive exchange task
pub(crate) async fn route_message(server: Arc<LocalServer>, source:SocketAddr, message: Vec<u8>){
    let header: StationHeader = bincode::deserialize(&message).unwrap();
    let stations = server.read_stations().await;
    
    
    // The message can be some channel or it can be a no message channel
    // The no message channel applies to all channels and routing takes place 
    // with just the station id
    if header.channel == NO_MESSAGE_CHANNEL{
        println!("Got no message");
        // We need a list of all stations
        for channel in stations.values(){
            if let Some(station) = channel.get(&header.to_id){
                let _ = station.send((source,message));
                break;
            }
        }
        return;
    }
    
    // Next is if we have a ping channel message
    // This is a new station telling all other stations on a particular channel that
    // exsits
    // If this is the case, the to_id is actually the channel the sending station is on
    
    if header.channel == PING_CHANNEL{
        if let Some(channel) = stations.get(&(header.to_id as StationChannel)){
            // Now this is a ping, so we need to notify all stations on this channel
            for station in channel.values(){
                // The idea is that each station will get this ping
                // thus adding the sender to its internal list of stations
                // Then it will send a no message back to the source of the ping
                // letting the source know of its existence
                let _ = station.send((source,message.clone()));
            }
        }
        return;
    }
    
    // Since stations don't know other stations until server contact has been made
    // through the station protocol
    // We need to specifiy a different protocol for server communication
    // Essentially we just find the single station on our own
    if header.channel == SERVER_CHANNEL{
        if let Some(channel) = stations.get(&header.channel){
            if let Some(station) = channel.get(&server.internal_station_id){
                let _ = station.send((source, message));
            }
        }
        return;
    }
    
    
    // Lastly, for any other arbitrary channel, we pass the message along as normal
    // according to its channel and id
    if let Some(channel) = stations.get(&header.channel){
        if let Some(station) = channel.get(&header.to_id){
            let _ = station.send((source,message));
        }
    }
}

impl<T:StationOperable> Station<T>{
    pub async fn new_async(server: Arc<LocalServer>, channel: StationChannel, external_id: Option<StationId>) -> Station<T> {
        // We need an id
        let id:StationId;
        if let Some(eid) = external_id{
            id = eid;
        }
        else{
            id = thread_rng().gen::<StationId>();
        }
        let intake_channel = flume::unbounded();
        let station:Station<T> = Station{ 
            id,
            channel,
            server: server.clone(),
            intake: intake_channel.1,
            known_stations: HashMap::new(),
            message_queue: VecDeque::new(),
            object: None };
        
        let mut addrs:Vec<SocketAddr>;
        {
            // When we make a new station we must ping for others in all known servers
            let servers = server.read_servers().await;
            addrs = servers.keys().map(|addr| *addr).collect();
        }
        addrs.push(server.local_address());
        
        // Now we must ping all of the servers
        for addr in addrs{
            station.ping(addr);
        }
        
        station
    }
    pub fn new(server: Arc<LocalServer>, channel: StationChannel, external_id: Option<StationId>) -> Station<T> {
        server.runtime.block_on(Self::new_async(server.clone(), channel, external_id))
    }
    fn ping(&self, tgt_server: SocketAddr){
        // We need the ping header
        let header = StationHeader{ 
            from_id: self.id,
            // Remember, for ping messages the to_id member is for the channel
            to_id: self.channel as u64,
            channel: PING_CHANNEL };
        let header = bincode::serialize(&header).unwrap();
        
        // Then we prepare the message
        let op = MessageOp::Send(tgt_server, true, header);
        
        // Then send
        // We use spawn here because we might be pinging a huge number of servers
        tokio::spawn(LocalServer::exchange(self.server.clone(), op));
        
    }
    pub async fn send(&mut self, tgt:StationId, nak: bool, object: &T) -> Result<bool, StationSendError>{
        // First we ensure our interal state is up to date
        self.queue_intake().await;
        
        // Before we spend any more cpu time with allocations, let make sure 
        // we know of tgt
        if let Some(tgt_addr) = self.known_stations.get(&tgt){
            // Then we need to break the object into bytes
            let data = object.to_bytes();
        
            // Then we need to prepare a header in bytes
            let header = StationHeader{ 
                from_id: self.id,
                to_id: tgt,
                channel: self.channel };
            let mut header = bincode::serialize(&header).unwrap();
        
            // Then fuse
            header.extend_from_slice(&data);
        
            // Then send
            let op = MessageOp::Send(*tgt_addr, nak, header);
        
            return match LocalServer::exchange(self.server.clone(), op).await{
                Ok(_) => Ok(true),
                Err(_) => Err(StationSendError::AckFailure),
            };
        } 
        else {return Err(StationSendError::UnknownStation);};
    }
    pub async fn receive(&mut self) -> Option<StationReturn<T>>{
        //First we need to update internal state
        self.queue_intake().await;
        // Then we need to try pull the first message
        let Some((source, message)) = self.message_queue.pop_front() else {return None};
        // Then we seperate our data
        let Ok(header) = bincode::deserialize::<StationHeader>(&message) else {return None};
        let data = &message[size_of::<StationHeader>()..size_of::<T>()+size_of::<StationHeader>()];
        let object = T::from_bytes(&data);
        
        Some((source, header.from_id, object))
    }
    pub async fn listen(&mut self) -> Option<StationReturn<T>>{
        // here we wait till something arrives at the station
        self.wait_intake().await;
        // Then we see if its a message that matters
        if let Some((source, message)) = self.message_queue.pop_front(){
            // Then we seperate our data
            let Ok(header) = bincode::deserialize::<StationHeader>(&message) else {return None};
            let data = &message[size_of::<StationHeader>()..size_of::<T>()+size_of::<StationHeader>()];
            let object = T::from_bytes(&data);
        
            return Some((source, header.from_id, object));
        }
        
        None
    }
    pub async fn receive_all(&mut self) -> Vec<StationReturn<T>> {
        let mut objects = vec![];
        //First we need to update internal state
        self.queue_intake().await;
        // Then we need to iterate through all messages
        for (source, message) in self.message_queue.drain(..){
            // Then we seperate our data
            let Ok(header):Result<StationHeader, _> = bincode::deserialize(&message) else {println!("Message queue drained at error"); return objects;};
            let data = &message[size_of::<StationHeader>()..size_of::<T>()+size_of::<StationHeader>()];
            let object = T::from_bytes(&data);
            objects.push((source, header.from_id, object));
        }
        objects
        
        
    }
    async fn wait_intake(&mut self){
        let intake = self.intake.recv_async().await.unwrap();
        self.intake(intake).await;
    }
    async fn intake(&mut self, intake: (SocketAddr, Vec<u8>)){
        let (source, message) = intake;
        // First we pull the header
        let header:StationHeader = bincode::deserialize(&message).unwrap();
        
        // For all messages we just add stations we don't know
        if let None = self.known_stations.get(&header.from_id){
            println!("Station {} discoverd station {} at {}", self.id, header.from_id, source);
            let _ = self.known_stations.insert(header.from_id, source);
            // If this is server communication from a new server we need to send back a no message
            if header.channel == SERVER_CHANNEL{
                let header = StationHeader{ 
                    from_id: self.id,
                    to_id: header.from_id,
                    channel: NO_MESSAGE_CHANNEL }; 
                let header = bincode::serialize(&header).unwrap();
                let op = MessageOp::Send(source, true, header);
                let _ = LocalServer::exchange(self.server.clone(), op).await;
            }
        }
        
        // If this is a no message then we will just forget about it
        if header.channel == NO_MESSAGE_CHANNEL{
            return;
        }
        
        // If this is a ping message we need to send back a no message
        if header.channel == PING_CHANNEL{
            let header = StationHeader{ 
                from_id: self.id,
                to_id: header.from_id,
                channel: NO_MESSAGE_CHANNEL }; 
            let header = bincode::serialize(&header).unwrap();
            let op = MessageOp::Send(source, true, header);
            let _ = LocalServer::exchange(self.server.clone(), op).await;
            
            return;
        }
        // If the message is normal we add it to the message queue
        self.message_queue.push_back((source,message));
    }
    /// This function is responsible for seperating internal messages and messages meant for the user
    async fn queue_intake(&mut self){
        let intakes:Vec<(SocketAddr, Vec<u8>)> = self.intake.try_iter().collect();
        for intake in intakes{
            self.intake(intake).await;
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
                
                
                
