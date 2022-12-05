use std::sync::Arc;
use crate::{ForeignServer, LocalServer, message_exchange::{Fragment, Message}, MAX_MESSAGE_LENGTH, Bytable};

#[repr(C)]
#[derive(Clone)]
pub(crate) enum ServerMessageType{
    KeepAlive,
    CommGroup(u32)
}

impl ForeignServer{
    pub(crate) async fn send_to(&self, data: &[u8]){
        self.local_server.send(self.address, data).await;
    }
    /// Essentially, 
    pub(crate) async fn process_message(fserver: Arc<ForeignServer>, fragments: Vec<Option<Fragment>>){
        let data = Self::fragments_to_message(fragments);
        let header = ServerMessageType::from_bytes(&data);
        match header{
            ServerMessageType::KeepAlive => {
                println!("Server for {} received keep alive", fserver.address);
                // fserver.keep_alive_channel.send(Instant::now()).expect("Keep alive should not terminate before terminal connection is dropped");
            },
            ServerMessageType::CommGroup(id) => {
                println!("Received message for comm group {}", id);
            },
        }
        
    }
    fn fragments_to_message(fragments: Vec<Option<Fragment>>) -> Message{
        let mut message_data:Vec<u8> = Vec::with_capacity(fragments.len() * MAX_MESSAGE_LENGTH);        
        for fragment in fragments.iter(){
            let (len, data) = fragment.expect("Should have either complete message or dropped it");
            message_data.extend_from_slice(&data[0..len]);
        }
        message_data
        

    }
}

