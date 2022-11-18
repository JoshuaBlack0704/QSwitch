use std::{sync::Arc, collections::HashMap, net::SocketAddr};

use tokio::sync::RwLock;

use crate::{LiveState, SocketHandler, TerminalConnection, SocketPacket};



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
    pub(crate) async fn add_get_message(live_state: Arc<Self>, message_id: u64) -> Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)> {
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
    pub(crate) async fn first_get_message(live_state: Arc<LiveState>, message_id: u64) -> (bool, Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>) {
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
    pub(crate) async fn remove_message(live_state: Arc<LiveState>, message_id: u64){
        let mut writer = live_state.message_map.write().await;
        writer.remove(&message_id);
    }
}


