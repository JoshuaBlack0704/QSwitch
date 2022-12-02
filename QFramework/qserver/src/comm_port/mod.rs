// The comm port will be the defact way in which users will
// communicate in a cluster
// The main peices of a comm port are its channel and id
// The channel is to identify a set of ports that will talk to each other and 
// all channels will be provided by users
// The id is the unique identifier for that particular port that will be
// used for routing

/// A comm port network will need to self manage state so each comm port on each
/// terminal knows all other comm ports on all other terminals
/// To do this when a new terminal connections is established, the first thing they do is 
/// Inform the foreign terminal of all of its comm ports. Of course this goes
/// both ways.
/// Further, any time a port is created or destroyed a terminal will notifiy all other temrinals
/// of the change
use std::{sync::Arc, mem::size_of};

use rand::{thread_rng, Rng};

use crate::{CommPort, LiveState, TerminalConnection, Bytable};

pub(super) type Id = u64;
pub(super) type Channel = u32;

#[derive(Clone)]
#[repr(C)]
pub(crate) struct CommPortHeader{
    from_id: Id,
    to_id: Id,
    channel: Channel,
    ping: bool,
}

impl CommPort{
    pub(crate) fn new(channel: u32, live_state: Arc<LiveState>) -> Arc<CommPort> {
        // On creation a CommPort needs to ping all other open CommPorts of the same channel
        // Since this is a special operation we can employ a dedicated method from a terminal connection
        // to ensure that our ping gets delived to all ports of a channel of a terminal 
        // This ping is sent to all known terminals
        
        // We need to generate this port's id
        let id = thread_rng().gen::<u64>();
        
        let ping_header = CommPortHeader{ from_id: id, to_id: 0, channel, ping: true };
        let mut data = vec![0u8;size_of::<CommPortHeader>()];
        ping_header.to_bytes(&mut data);
        
        let 
        
        // Now we need to send this header to all known terminals
        
        
        Arc::new(CommPort{ id, channel, live_state })
    }
    // Since we can't store type information inside of the CommPort we will
    // need to have an api which handles type transformations before we reach
    // internally
    
    
}

impl LiveState{
    
}

impl TerminalConnection{
    /// This function sends all live state port data to a foreign terminal
    /// Remeber, a terminal connections struct represents a foreign terminal
    async fn broadcast_port_data_to(&self){
        // We will need to compile a list of all ports
        let known_ports:Vec<(Id, Channel)> = vec![];
        
        
    }
    //Here we set the method to handle Comm Port communicaton
    async fn handle_comm_port(data: Vec<u8>){}
}
