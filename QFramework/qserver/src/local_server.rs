use std::{sync::Arc, net::SocketAddr, collections::HashMap, time::Duration};



use local_ip_address::local_ip;
use tokio::time::{self, timeout};
use tokio::{net::UdpSocket, runtime::Runtime, sync::{RwLock, RwLockReadGuard, RwLockWriteGuard}, time::sleep};

use crate::{KEEP_ALIVE_TIMEOUT, KEEP_ALIVE_BUDGET, async_timer};
use crate::{LocalServer, SocketPacket, TerminateSignal, message_exchange::MessageOp, MAX_MESSAGE_LENGTH, station::StationHeader};

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
    
    async fn udp_intake(server: Arc<LocalServer>){
        let lifetime = server.life.subscribe();
        loop {
            tokio::select! {
                _ = lifetime.terminated()=>{println!("Shutting down main udp listener for {}", server.local_address());break;}
                message = server.recieve()=>{
                    let op = MessageOp::Receive(message);
                    LocalServer::update_foreign_server(server.clone(), message.1).await;
                    tokio::spawn(Self::exchange(server.clone(), op));
                }
            }
        }
    }
    
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
    async fn keep_alive(server: Arc<LocalServer>, addr:SocketAddr){
        println!("Keep alive started for tgt {}", addr);
        // The first thing we need is an entry into the foreign servers list so we can be found
        let (tx, rx) = flume::bounded(1);
        {
            let mut writer = server.write_server().await;
            // Has one been added since we get the writer?
            if let Some(sender) = writer.get(&addr){
                let _ = sender.try_send(true);
                // If this is the case then there is some other keep alive task going so we 
                // can go ahead and cancel this one
                return;
            }
            else{
                // If not we add one and continue
                writer.insert(addr, tx);
            }
        }
        
        let mut keep_alive_budget = KEEP_ALIVE_BUDGET;
        
        while keep_alive_budget > 0{
            // Each loop we must count down the keep alive budget
            // If we receive an update it will be reset in the loop
            keep_alive_budget -= 1;
            
            // We need to check for an update
            if let Ok(_) = rx.try_recv(){
                keep_alive_budget = KEEP_ALIVE_BUDGET;
                println!("Keep alive maintained for tgt {}", addr);
            }
            
            // Now we send this cycle's keep alive message
            let header = bincode::serialize(&StationHeader::no_message()).unwrap();
            let op = MessageOp::Send(addr,false,header);
            let _ = Self::exchange(server.clone(), op).await;
            
            async_timer(KEEP_ALIVE_TIMEOUT).await;
        }
        // If we run out of keep alives we will need to remove the entry from the foreign servers list
        let mut writer = server.write_server().await;
        writer.remove(&addr);
        println!("Keep alive stopped for tgt {}", addr);
    }
}

/// State management functionality
impl LocalServer{
    pub(crate) async fn read_servers(&self) -> RwLockReadGuard<HashMap<SocketAddr, flume::Sender<bool>>> {
        self.foreign_servers.read().await
    }
    pub(crate) async fn read_exchanges(&self) -> RwLockReadGuard<HashMap<u64, Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>>> {
        self.message_exchanges.read().await
    }
    pub(crate) async fn read_stations(&self) -> RwLockReadGuard<HashMap<u32, HashMap<u64, flume::Sender<Vec<u8>>>>>  {
        self.stations.read().await
    }
    pub(crate) async fn write_server(&self) -> RwLockWriteGuard<HashMap<SocketAddr, flume::Sender<bool>>> {
        self.foreign_servers.write().await
    }
    pub(crate) async fn write_exchanges(&self) -> RwLockWriteGuard<HashMap<u64, Arc<(flume::Sender<SocketPacket>, flume::Receiver<SocketPacket>)>>> {
        self.message_exchanges.write().await
    }
    pub(crate) async fn write_stations(&self) -> RwLockWriteGuard<HashMap<u32, HashMap<u64, flume::Sender<Vec<u8>>>>> {
        self.stations.write().await
    }
    pub(crate) async fn update_foreign_server(server:Arc<LocalServer>, addr: SocketAddr){
        // First we see if one exisits
        let reader = server.read_servers().await;
        if let Some(sender) = reader.get(&addr){
            let _ = sender.try_send(true);
        }
        else{
            // If not we start a new keep alive task which will create and manage one
            tokio::spawn(Self::keep_alive(server.clone(), addr));
        }
    }
    pub fn connect_to_server(server: Arc<LocalServer>, addr: SocketAddr){
        server.runtime.block_on(Self::update_foreign_server(server.clone(), addr));
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
        let mut data = [0; MAX_MESSAGE_LENGTH];
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
        // println!("Socket {} sent {} bytes to {}", self.local_address(), data.len(), tgt);
        self.socket.send_to(&data, tgt).await.unwrap();
    }
    pub fn local_address(&self) -> SocketAddr {
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
