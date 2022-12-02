use std::sync::Arc;

use crate::{ForeignServer, LocalServer, message_exchange::Fragment};

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
    pub(crate) async fn process_message(fserver: Arc<ForeignServer>, fragments: Vec<Option<Fragment>>){}
}

impl LocalServer{
}