use std::{sync::Arc, net::SocketAddr, collections::HashMap};

use tokio::{runtime::Runtime, net::UdpSocket, sync::RwLock, time::Instant};

//QServer public definitions
mod cluster_terminal;
mod comm_group;
mod comm_port;






/// The main struct of the QServer library
/// This struct will initialize the async system and either connect to, or start, a cluster

pub struct ClusterTerminal {
    /// The tokio runtime used to run the network system
    /// Can either be given or built internally
    rt: Arc<Runtime>,
    ///Is this terminal a public terminal. Can conneting terminals discover it
    public: bool,
    ///The line of communication with the active network main task
    socket: SocketHandler,
    ///The signal to terminate the async system
    network_terminate: TerminateSignal,
    /// The map used to store all connected terminals
    terminal_map: TerminalAddressMap,
}

/// The CommGroup struct is how a user of a ClusterTerminal would send data over the cluster
/// All of the logic required to converte user data types to bytes and resolve targets is included
pub struct CommGroup {}
/// The CommPort struct represents a channel for users to push data to a live CommGroup for transfer.
pub struct CommPort {}

#[derive(Clone)]
pub struct SocketHandler {
    socket: Arc<UdpSocket>,
}
//Represents a connection to another machine
//A target of messages
//Implicitly carries lifetime information, so can't be cloned
pub struct TerminalConnection {
    is_public: bool,
    addr: SocketAddr,
    socket: SocketHandler,
    terminal_map: TerminalAddressMap,
    keep_alive_time: Instant,
    life: Arc<TerminateSignal>,
}
#[derive(Clone)]
pub struct TerminateSignal {
    channel: (flume::Sender<bool>, flume::Receiver<bool>),
}
#[derive(Clone)]
pub struct TerminalAddressMap {
    active_connections: Arc<RwLock<HashMap<SocketAddr, Arc<RwLock<TerminalConnection>>>>>,
}
