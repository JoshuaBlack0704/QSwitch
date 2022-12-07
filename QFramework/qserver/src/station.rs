use std::sync::Arc;
use serde::{Serialize, Deserialize};

use crate::{Station, LocalServer, Serializable, NO_MESSAGE_CHANNEL, PING_CHANNEL};

pub(crate) type StationId = u64;
pub(crate) type StationChannel = u32;
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StationHeader{
    from_id: StationId,
    to_id: StationId,
    channel: StationChannel,
}

impl Station{
    /// The entry point for station messages. Is used from a receive exchange task
    pub(crate) async fn route_message(server: Arc<LocalServer>, message: Vec<u8>){
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
                    let _ = station.send(message);
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
            if let Some(channel) = stations.get(&(header.to_id as u32)){
                // Now this is a ping, so we need to notify all stations on this channel
                for station in channel.values(){
                    // The idea is that each station will get this ping
                    // thus adding the sender to its internal list of stations
                    // Then it will send a no message back to the source of the ping
                    // letting the source know of its existence
                    let _ = station.send(message.clone());
                }
            }
            return;
        }
        
        
        // Lastly, for any other arbitrary channel, we pass the message along as normal
        // according to its channel and id
        if let Some(channel) = stations.get(&header.channel){
            if let Some(station) = channel.get(&header.to_id){
                let _ = station.send(message);
            }
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
                
                
                
