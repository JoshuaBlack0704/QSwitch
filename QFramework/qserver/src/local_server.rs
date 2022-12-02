use std::{sync::Arc, net::SocketAddr, collections::HashMap, time::Duration};

use local_ip_address::local_ip;
use tokio::{net::UdpSocket, runtime::Runtime, sync::{RwLock, RwLockReadGuard, RwLockWriteGuard}, time::sleep};

use crate::{LocalServer, SocketPacket, TerminateSignal, ForeignServer};

impl LocalServer{
    ///
    pub fn new(
        target_socket: Option<SocketAddr>,
        discoverable: bool,
        target_runtime: Option<Arc<Runtime>>,
    ) -> Arc<LocalServer>{
        let target_socket = match target_socket {
            Some(a) => a,
            None => SocketAddr::new(local_ip().unwrap(), 0),
        };

        let target_runtime = match target_runtime {
            Some(r) => r,
            None => Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap(),
            ),
        };
        let socket = Self::new_socket(target_socket, target_runtime.clone());
        let life = TerminateSignal::new();
        let foreign_servers = RwLock::new(HashMap::new());
        let message_exchanges = RwLock::new(HashMap::new());
        let stations = RwLock::new(HashMap::new());
        
        println!(
            "Started Cluster Terminal on {}",
            socket.local_addr().unwrap()
        );
        
        let server = Arc::new(LocalServer{ 
            runtime: target_runtime.clone(),
            discoverable,
            socket,
            life,
            foreign_servers,
            message_exchanges,
            stations });

        target_runtime.spawn(Self::udp_intake(server.clone()));
        server
    }
    
    async fn udp_intake(server: Arc<LocalServer>){}
    
    pub fn get_runtime(&self) -> Arc<Runtime> {
        self.runtime.clone()
    }
    pub fn idle_async(&self){
        self.runtime.block_on(Self::idle());
    }
    async fn idle(){
        loop{
            sleep(Duration::from_millis(1000*5)).await;
        }
    }
    
}

/// State management functionality
impl LocalServer{
    pub(crate) async fn read_servers(&self) -> RwLockReadGuard<HashMap<SocketAddr, Arc<ForeignServer>>> {
        self.foreign_servers.read().await
    }
    pub(crate) async fn read_exchanges(&self) -> RwLockReadGuard<HashMap<u64, Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>>> {
        self.message_exchanges.read().await
    }
    pub(crate) async fn read_stations(&self) -> RwLockReadGuard<HashMap<(u64, u32), Arc<crate::Station>>> {
        self.stations.read().await
    }
    pub(crate) async fn write_server(&self) -> RwLockWriteGuard<HashMap<SocketAddr, Arc<ForeignServer>>> {
        self.foreign_servers.write().await
    }
    pub(crate) async fn write_exchanges(&self) -> RwLockWriteGuard<HashMap<u64, Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>>> {
        self.message_exchanges.write().await
    }
    pub(crate) async fn write_stations(&self) -> RwLockWriteGuard<HashMap<(u64, u32), Arc<crate::Station>>> {
        self.stations.write().await
    }
    pub(crate) async fn get_foreign_server(&self, address: SocketAddr) -> Arc<ForeignServer> {
        let reader = self.read_servers().await;
        reader.get(&address).expect("Not such known foreign server").clone()
    }
}

/// Socket functionality
impl LocalServer{
    /// Creates a new SocketHandler
    /// # Arguments
    /// * `socket_addr` - The address socket will be bound to
    /// * `rt` - The runtime used to bind the socket
    fn new_socket(socket_addr: SocketAddr, rt: Arc<Runtime>) -> UdpSocket {
        rt.block_on(UdpSocket::bind(socket_addr)).unwrap()
    }
    /// Async waits to receive a viable message
    /// Erroed messages are just dropped
    async fn recieve(&self) -> SocketPacket{    
        let mut data = [0; 1024];
        loop {
            if let Ok((len, addr)) = self.socket.recv_from(&mut data).await {
                return (len, addr, data);
            }
        }
    }
    /// * `tgt` - The target address
    /// * `data` - A vector of bytes. Needs to be a vector to help with lifetime issues
    /// Async sends a message to the `tgt`
    pub(crate) async fn send(&self, tgt: SocketAddr, data: &[u8]) {
        println!("Socket {} send {} bytes to {}", self.local_address(), data.len(), tgt);
        self.socket.send_to(&data, tgt).await.unwrap();
    }
    pub(crate) fn local_address(&self) -> SocketAddr {
        self.socket.local_addr().unwrap()
    }
    
}

impl TerminateSignal {
    /// Creates a new Terminate Signal
    pub fn new() -> TerminateSignal {
        TerminateSignal {
            channel: flume::bounded(1),
        }
    }
    /// Creates a new child of the terminate signal that will be notified
    pub fn subscribe(&self) -> TerminateSignal {
        let rx = self.channel.1.clone();
        let tx = flume::bounded(1).0;
        TerminateSignal { channel: (tx, rx) }
    }
    /// What a child can wait on to be notified of parent drop
    pub async fn terminated(&self) {
        let _ = self.channel.1.recv_async().await;
    }
}
