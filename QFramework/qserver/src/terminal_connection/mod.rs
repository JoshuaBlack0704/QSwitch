pub mod message_exchange;
use std::{net::SocketAddr, sync::Arc, mem::size_of};

use tokio::time::{Instant, sleep, Duration};

use crate::{TerminalConnection, SocketHandler, LiveState, TerminateSignal, Bytable, MAX_MESSAGE_LENGTH, CommGroup};

use self::message_exchange::{MessageOp, Fragment, Message};


#[repr(C)]
#[derive(Clone)]
pub(crate) enum TerminalMessageType{
    KeepAlive,
    CommGroup(u32)
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
            live_state: live_state.clone(),
            keep_alive_channel: keep_alive_channel.0,
            life });
        tokio::spawn(Self::keep_alive(live_state.clone(), tgt_addr, keep_alive_channel.1));
        terminal
        
    }
    async fn keep_alive(live_state: Arc<LiveState>, tgt_addr: SocketAddr, message_channel: flume::Receiver<Instant>){
        let mut no_response_budget = 10;
        let keep_alive = TerminalMessageType::KeepAlive;
        let mut keep_alive_data = vec![0u8;size_of::<TerminalMessageType>()];
        keep_alive.to_bytes(&mut keep_alive_data);
        
        let test = TerminalMessageType::CommGroup(1);
        let mut test_data = vec![0u8;size_of::<TerminalMessageType>()];
        test.to_bytes(&mut test_data);
        
        loop{
            tokio::select!{
                val = message_channel.recv_async()=>{
                    no_response_budget = 10;
                    if let Err(_) = val{
                        break;
                    }
                }
                _ = sleep(Duration::from_secs(TERMINAL_KEEPALIVE_WAIT))=>{
                    let terminal = LiveState::add_get_terminal(live_state.clone(), tgt_addr).await;
                    let op = MessageOp::Send(terminal.clone(), false, keep_alive_data.clone());
                    let test_op = MessageOp::Send(terminal.clone(), true, test_data.clone());
                    tokio::spawn(Self::message_exchange(terminal.live_state.clone(), op));
                    tokio::spawn(Self::message_exchange(terminal.live_state.clone(), test_op));
                    no_response_budget -= 1;
                    if no_response_budget <= 0{
                        LiveState::remove_terminal(terminal.live_state.clone(), terminal.tgt_addr).await;
                    }
                }
            }
        }
        println!("Disconnected from terminal {}", tgt_addr);
    }
    
    pub(crate) async fn process_message(live_state: Arc<LiveState>, terminal: Arc<TerminalConnection>, fragments: Vec<Option<Fragment>>){
        let data = Self::fragments_to_message(fragments);
        let header = TerminalMessageType::from_bytes(&data);
        match header{
            TerminalMessageType::KeepAlive => {
                println!("Terminal for {} received keep alive", terminal.tgt_addr);
                terminal.keep_alive_channel.send(Instant::now()).expect("Keep alive should not terminate before terminal connection is dropped");
            },
            TerminalMessageType::CommGroup(id) => {
                println!("Received message for comm group {}", id);
                let comm = LiveState::add_get_commgroup(live_state.clone(), id).await;
                let from_addr = terminal.tgt_addr; 
                tokio::spawn(CommGroup::process_message(comm, from_addr));
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