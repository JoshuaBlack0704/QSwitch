use local_ip_address::local_ip;
use std::{net::SocketAddr, sync::Arc};

use tokio::{net::UdpSocket, time::sleep};
use tokio::runtime::Runtime;
use tokio::time::Duration;



use crate::{TerminalConnection, live_state::MessageOp};

/// # The general purpose implementations of Cluster Terminal
///
/// 'ClusterTerminal' is the main entry point to starting or connecting to a cluster.
/// It provides the starting api and the neccesary functions to create communication layers
use super::{ClusterTerminal, SocketPacket, SocketHandler, LiveState, TerminateSignal};

impl ClusterTerminal {
    /// * `target_socket` - To provide user defined address. Will otherwise use the system network address and random port
    /// * `discoverable` - Will the address of this terminal be shared wihtout its consent
    /// * `target_runtime` - Allows passing external runtimes to start the network systems on. Will create one internally if None.
    ///
    /// Creates a new `ClusterTerminal`
    pub fn new(
        target_socket: Option<SocketAddr>,
        discoverable: bool,
        target_runtime: Option<Arc<Runtime>>,
    ) -> ClusterTerminal {
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
        let socket = SocketHandler::new(target_socket, target_runtime.clone());
        let network_terminate = TerminateSignal::new();
        let terminal_map = LiveState::new(socket.clone(), discoverable);

        target_runtime.spawn(Self::udp_listener(
            terminal_map.clone(),
            network_terminate.subscribe(),
            socket.clone(),
        ));

        println!(
            "Started Cluster Terminal on {}",
            socket.socket.local_addr().unwrap()
        );

        ClusterTerminal {
            rt: target_runtime,
            discoverable,
            socket,
            network_terminate,
            live_state: terminal_map,
        }
    }

    ///The main udp async task
    /// * `terminal_map` - The map of live terminal connections being maintained
    /// * `terminate_signal` - The signal used to stop the async task when the Cluster Terminal is dropped
    /// * `socket` - The bound SocketHandler that will provide the async socket tasks
    async fn udp_listener(
        live_state: Arc<LiveState>,
        terminate_signal: TerminateSignal,
        socket: SocketHandler,
    ) {
        loop {
            tokio::select! {
                _ = terminate_signal.terminated()=>{println!("Shutting down main udp listener for {}", socket.socket.local_addr().unwrap());break;}
                message = socket.recieve()=>{
                    let op = MessageOp::Receive(message);
                    tokio::spawn(TerminalConnection::message_exchange(live_state.clone(), op));
                }
            }
        }
    }
    pub fn connect_to(&self, tgt: SocketAddr){
        let _ = self.rt.block_on(LiveState::add_get_terminal(self.live_state.clone(), tgt));
    }
    pub fn get_addr(&self) -> SocketAddr {
        self.socket.local_address()
    }
    pub fn get_runtime(&self) -> Arc<Runtime> {
        self.rt.clone()
    }
    pub fn idle_async(&self){
        self.rt.block_on(Self::idle());
    }
    async fn idle(){
        loop{
            sleep(Duration::from_millis(1000*5)).await;
        }
    }
}

impl SocketHandler {
    /// Creates a new SocketHandler
    /// # Arguments
    /// * `socket_addr` - The address socket will be bound to
    /// * `rt` - The runtime used to bind the socket
    fn new(socket_addr: SocketAddr, rt: Arc<Runtime>) -> SocketHandler {
        let socket = rt.block_on(UdpSocket::bind(socket_addr)).unwrap();
        SocketHandler {
            socket: Arc::new(socket),
        }
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
    pub async fn send(&self, tgt: SocketAddr, data: &[u8]) {
        println!("Socket {} send {} bytes to {}", self.local_address(), data.len(), tgt);
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
