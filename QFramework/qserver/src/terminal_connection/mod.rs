
use std::{net::SocketAddr, sync::Arc};

use super::{TerminalConnection, SocketHandler, TerminalMap, TerminateSignal};

impl TerminalConnection{
    pub fn new(tgt_addr: SocketAddr, socket: SocketHandler, terminal_map: Arc<TerminalMap>, discoverable: bool) -> Arc<TerminalConnection >{
        let keep_alive_channel = flume::unbounded();
        let life = TerminateSignal::new();
        println!("Creating new terminal connection for target {}", tgt_addr);
        Arc::new(TerminalConnection{ 
            discoverable,
            tgt_addr,
            socket,
            terminal_map,
            keep_alive_channel: keep_alive_channel.0,
            life })
    }
}