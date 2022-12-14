use std::{sync::Arc, net::SocketAddr, collections::{HashMap, VecDeque, HashSet}};
use serde::{Serialize, Deserialize};

use station::StationId;
use tokio::{runtime::Runtime, net::UdpSocket, sync::RwLock, time::{Duration,sleep}};

mod local_server;
mod station;
mod message_exchange;


pub(crate) const MAX_MESSAGE_LENGTH: usize = 1024;
pub(crate) const KEEP_ALIVE_TIMEOUT: u64 = 500;
pub(crate) const KEEP_ALIVE_BUDGET: usize = 3;
pub(crate) const NO_MESSAGE_CHANNEL:u32 = u32::MAX;
pub(crate) const PING_CHANNEL:u32 = u32::MAX - 1;
pub(crate) const SERVER_CHANNEL:u32 = u32::MAX - 2;
pub(crate) const NO_DELIVER_CHANNEL:u32 = u32::MAX - 3;
pub(crate) type SocketPacket = (usize, SocketAddr, [u8; MAX_MESSAGE_LENGTH]);

pub trait StationOperable{
    fn to_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Self;
}
#[derive(Clone, Serialize, Deserialize)]
pub enum ServerInternalComm{
    // When you ping you send a bool that determines discoverability
    Ping(bool),
    KeepAlive(bool),
    AddrDownload(Vec<SocketAddr>),
}

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
    keep_alive_tasks: RwLock<HashMap<SocketAddr, flume::Sender<bool>>>,
    /// The state of all live message exchanges
    message_exchanges: RwLock<HashMap<u64, Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>>>,
    /// The state of all known comm ports
    stations: RwLock<HashMap<station::StationChannel, HashMap<station::StationId, flume::Sender<(SocketAddr, Vec<u8>)>>>>,    
    /// Server Communication Station ID
    internal_station_channel: Option<flume::Sender<(SocketAddr, Vec<u8>)>>,
}

/// The CommPort struct represents a channel for users to push data to a live CommGroup for transfer.
pub struct Station<T: StationOperable> {
    id: u64,
    channel: u32,
    server: Arc<LocalServer>,
    intake: (flume::Sender<(SocketAddr, Vec<u8>)>, flume::Receiver<(SocketAddr, Vec<u8>)>),
    known_stations: HashMap<station::StationId, SocketAddr>,
    message_queue: VecDeque<(SocketAddr,Vec<u8>)>,
    object: Option<T>,
}


#[derive(Clone)]
struct TerminateSignal {
    channel: (flume::Sender<bool>, flume::Receiver<bool>),
}

pub(crate) async fn async_timer(timeout: u64){
    sleep(Duration::from_millis(timeout)).await;
}
