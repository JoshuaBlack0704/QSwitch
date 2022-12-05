use std::{sync::Arc, net::SocketAddr, collections::HashMap};

use tokio::{runtime::Runtime, net::UdpSocket, sync::RwLock, time::{Duration,sleep}};

mod bytable;
mod local_server;
mod foreign_server;
mod station;
mod message_exchange;


const MAX_MESSAGE_LENGTH: usize = 1024;
pub(crate) type SocketPacket = (usize, SocketAddr, [u8; MAX_MESSAGE_LENGTH]);


pub trait Bytable{
    fn to_bytes(&self, dst: &mut [u8]);       
    fn from_bytes(src: &[u8]) -> Self;
}

pub(crate) const SERVER_PING_CHANNEL:u32 = u32::MAX;

/// The main struct of the QServer library
/// This struct will initialize the async system and either connect to, or start, a cluster
pub struct LocalServer{
    /// The tokio runtime we will be using
    runtime: Arc<Runtime>,
    /// Can connections be established by contacting this server
    /// True: yes
    /// False: no
    discoverable: bool,
    /// The upd socket
    socket: UdpSocket,
    /// This is used to shutdown any tasks that the Server spawns
    life: TerminateSignal,
    /// The state of all known servers
    foreign_servers: RwLock<HashMap<SocketAddr, flume::Sender<bool>>>,
    /// The state of all live message exchanges
    message_exchanges: RwLock<HashMap<u64, Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>>>,
    /// The state of all known comm ports
    stations: RwLock<HashMap<station::StationChannel, HashMap<station::StationId, flume::Sender<Vec<u8>>>>>,    
}

/// The CommPort struct represents a channel for users to push data to a live CommGroup for transfer.
pub struct Station {
    id: u64,
    channel: u32,
    server: Arc<LocalServer>,
}

#[derive(Clone)]
struct TerminateSignal {
    channel: (flume::Sender<bool>, flume::Receiver<bool>),
}

pub(crate) async fn async_timer(timeout: u64){
    sleep(Duration::from_millis(timeout)).await;
}
