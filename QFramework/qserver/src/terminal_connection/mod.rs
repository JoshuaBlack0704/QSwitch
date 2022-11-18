
use std::{net::SocketAddr, sync::Arc, mem::size_of};

use tokio::time::{Instant, sleep, Duration};


use crate::{live_state::{Fragment, Message}, MAX_MESSAGE_LENGTH, Bytable};

use super::{TerminalConnection,live_state::MessageOp, SocketHandler, LiveState, TerminateSignal};

#[repr(C)]
#[derive(Clone)]
pub(crate) enum TerminalMessageType{
    KeepAlive,
}

const TERMINAL_KEEPALIVE_WAIT:u64 = 10;


impl TerminalConnection{
    pub(crate) fn new(tgt_addr: SocketAddr, socket: SocketHandler, live_state: Arc<LiveState>, discoverable: bool) -> Arc<TerminalConnection >{
        let keep_alive_channel = flume::unbounded();
        let life = TerminateSignal::new();
        println!("Creating new terminal connection for target {}", tgt_addr);
        let terminal = Arc::new(TerminalConnection{ 
            discoverable,
            tgt_addr,
            socket,
            live_state,
            keep_alive_channel: keep_alive_channel.0,
            life });
        tokio::spawn(Self::keep_alive(terminal.clone(), keep_alive_channel.1));
        terminal
        
    }
    async fn keep_alive(terminal: Arc<TerminalConnection>, message_channel: flume::Receiver<Instant>){
        let mut no_response_budget = 10;
        let keep_alive = TerminalMessageType::KeepAlive;
        let mut keep_alive_data = vec![0u8;size_of::<TerminalMessageType>()];
        keep_alive.to_bytes(&mut keep_alive_data);
        let op = MessageOp::Send(terminal.clone(), false, keep_alive_data);
        
        tokio::spawn(Self::message_exchange(terminal.live_state.clone(), op.clone()));
        loop{
            tokio::select!{
                val = message_channel.recv_async()=>{
                    no_response_budget = 10;
                    if let Err(_) = val{
                        break;
                    }
                }
                _ = sleep(Duration::from_secs(TERMINAL_KEEPALIVE_WAIT))=>{
                    tokio::spawn(Self::message_exchange(terminal.live_state.clone(), op.clone()));
                    no_response_budget -= 1;
                    if no_response_budget <= 0{
                        println!("Disconneted from terminal {}", terminal.socket.local_address());
                        LiveState::remove_terminal(terminal.live_state.clone(), terminal.socket.local_address()).await;
                        break;
                    }
                }
            }
        }
    }
    
    pub(crate) async fn process_message(live_state: Arc<LiveState>, terminal: Arc<TerminalConnection>, fragments: Vec<Option<Fragment>>){
        let data = Self::fragments_to_message(fragments);
        let header = TerminalMessageType::from_bytes(&data);
        match header{
            TerminalMessageType::KeepAlive => {
                println!("Terminal for {} received keep alive", terminal.socket.local_address());
                terminal.keep_alive_channel.send(Instant::now()).expect("Keep alive should not terminate before terminal connection is dropped");
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