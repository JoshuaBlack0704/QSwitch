use local_ip_address::local_ip;
use std::{net::SocketAddr, sync::Arc};

use tokio::net::UdpSocket;
use tokio::runtime::Runtime;



/// # The general purpose implementations of Cluster Terminal
///
/// 'ClusterTerminal' is the main entry point to starting or connecting to a cluster.
/// It provides the starting api and the neccesary functions to create communication layers
use super::{ClusterTerminal, SocketPacket, SocketHandler, TerminalMap, TerminateSignal, MAX_MESSAGE_LENGTH};

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
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .unwrap(),
            ),
        };
        let socket = SocketHandler::new(target_socket, target_runtime.clone());
        let network_terminate = TerminateSignal::new();
        let terminal_map = TerminalMap::new(discoverable);

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
            terminal_map,
        }
    }

    ///The main udp async task
    /// * `terminal_map` - The map of live terminal connections being maintained
    /// * `terminate_signal` - The signal used to stop the async task when the Cluster Terminal is dropped
    /// * `socket` - The bound SocketHandler that will provide the async socket tasks
    async fn udp_listener(
        terminal_map: Arc<TerminalMap>,
        terminate_signal: TerminateSignal,
        socket: SocketHandler,
    ) {
        loop {
            tokio::select! {
                _ = terminate_signal.terminated()=>{println!("Shutting down main udp listener for {}", socket.socket.local_addr().unwrap());break;}
                message = socket.recieve()=>{
                    
                }
            }
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
    pub async fn send(&self, tgt: SocketAddr, data: Vec<u8>) {
        self.socket.send_to(&data, tgt).await.unwrap();
    }
    pub fn local_address(&self) -> SocketAddr {
        self.socket.local_addr().unwrap()
    }
}

impl TerminateSignal {
    /// Creates a new Terminate Signal
    fn new() -> TerminateSignal {
        TerminateSignal {
            channel: flume::bounded(1),
        }
    }
    /// Creates a new child of the terminate signal that will be notified
    fn subscribe(&self) -> TerminateSignal {
        let rx = self.channel.1.clone();
        let tx = flume::bounded(1).0;
        TerminateSignal { channel: (tx, rx) }
    }
    /// What a child can wait on to be notified of parent drop
    async fn terminated(&self) {
        let _ = self.channel.1.recv_async().await;
    }
}
