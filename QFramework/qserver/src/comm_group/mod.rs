use std::{sync::Arc, net::SocketAddr};

use tokio::time::{sleep, Duration};

use crate::{CommGroup, LiveState};

// Comm groups are the binding system for comm ports
// Essentially a comm groups handles connections state identification and tracking, routing,
// and interaction with the network layer and terminal connections
// It does all of this so the comm port can just behave like a normal typed channel 

// A comm group instance is considered to be one single peice of a whole comm group
// When a new comm group instance is created it will take all public terminal connections
// And ask them for all of their live ports and populate its foreign ports list
// Then it will 
impl CommGroup{
    pub(crate) fn new(id: u32, live_state: Arc<LiveState>) -> Arc<CommGroup> {
        Arc::new(CommGroup{ id, live_state })
    }
    
    pub(crate) async fn process_message(comm: Arc<CommGroup>, from_addr: SocketAddr){
        sleep(Duration::from_secs(1)).await;
        println!("Processed message for comm group {}", comm.id);
        
    }
    
    
}
