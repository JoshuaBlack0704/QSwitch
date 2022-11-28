use std::{sync::Arc, net::SocketAddr, collections::HashMap};

use tokio::{runtime::Runtime, net::UdpSocket, sync::RwLock, time::Instant};

mod cluster_terminal;
mod comm_port;
mod live_state;
mod terminal_connection;
mod bytable;




pub trait Bytable{
    fn to_bytes(&self, dst: &mut [u8]);       
    fn from_bytes(src: &[u8]) -> Self;
}

/// The main struct of the QServer library
/// This struct will initialize the async system and either connect to, or start, a cluster
/// TODO: Merge ClusterTerminal and terminal address map

pub struct ClusterTerminal {
    /// The tokio runtime used to run the network system
    /// Can either be given or built internally
    rt: Arc<Runtime>,
    ///Is this terminal a public terminal. Can conneting terminals discover it
    discoverable: bool,
    ///The line of communication with the active network main task
    socket: SocketHandler,
    ///The signal to terminate the async system
    network_terminate: TerminateSignal,
    /// The map used to store all connected terminals
    live_state: Arc<LiveState>,

}

/// The CommPort struct represents a channel for users to push data to a live CommGroup for transfer.
pub struct CommPort {
    id: u64,
    channel: u32,
    live_state: Arc<LiveState>,
}

#[derive(Clone)]
struct SocketHandler {
    socket: Arc<UdpSocket>,
}
//Represents a connection to another machine
//A target of messages
//Implicitly carries lifetime information, so can't be cloned
pub(crate) struct TerminalConnection {
    discoverable: bool,
    tgt_addr: SocketAddr,
    socket: SocketHandler,
    live_state: Arc<LiveState>,
    keep_alive_channel: flume::Sender<Instant>,
    life: TerminateSignal,
}
#[derive(Clone)]
struct TerminateSignal {
    channel: (flume::Sender<bool>, flume::Receiver<bool>),
}
struct LiveState {
    terminals: RwLock<HashMap<SocketAddr, Arc<TerminalConnection>>>,
    message_map: RwLock<HashMap<u64, Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>>>,
    comm_port_map: RwLock<HashMap<(u64, u32), Arc<CommPort>>>,
    socket: SocketHandler,
    discoverable: bool,
}

const MAX_MESSAGE_LENGTH: usize = 1024;
type SocketPacket = (usize, SocketAddr, [u8; MAX_MESSAGE_LENGTH]);
