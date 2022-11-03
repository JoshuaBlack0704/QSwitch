use std::{collections::HashMap, mem::size_of, net::SocketAddr, sync::Arc};

use log::debug;
use tokio::{
    net::{ToSocketAddrs, UdpSocket},
    runtime::Runtime,
};

//The server architecture must be game agnostic. That is it must only provide
//network communication functionalty and systems. The Quniverse and QSwitch will use these
//systems and functionality to "make a game"
#[async_trait::async_trait]
pub trait ILifetimeTree<T: ILifetimeTree<T, C>, C: ILifetimeTreeController> {
    fn child_from_tree(&self) -> T;
    fn new_tree() -> C;
    async fn shutdown(&self);
}
#[async_trait::async_trait]
pub trait ILifetimeTreeController {
    async fn shutdown(self);
}

pub struct LifetimeTreeController {
    pub tree: LifetimeTree,
    tree_control: (flume::Sender<bool>, flume::Receiver<bool>),
}
pub struct LifetimeTree {
    tree: (flume::Sender<bool>, flume::Receiver<bool>),
}
#[async_trait::async_trait]
impl ILifetimeTree<LifetimeTree, LifetimeTreeController> for LifetimeTree {
    fn child_from_tree(&self) -> LifetimeTree {
        LifetimeTree {
            tree: self.tree.clone(),
        }
    }

    fn new_tree() -> LifetimeTreeController {
        let uptree = flume::bounded(1);
        let downtree = flume::bounded(1);
        let tree = LifetimeTree {
            tree: (downtree.0, uptree.1),
        };
        let controller = LifetimeTreeController {
            tree,
            tree_control: (uptree.0, downtree.1),
        };
        controller
    }

    async fn shutdown(&self) {
        let _ = self.tree.1.recv_async().await;
    }
}
#[async_trait::async_trait]
impl ILifetimeTreeController for LifetimeTreeController {
    async fn shutdown(self) {
        drop(self.tree);
        drop(self.tree_control.0);
        let _ = self.tree_control.1.recv_async().await;
    }
}
//Will use a load and fire system where you first prime a network channel with data and then send it all at once
//internally the network channel will store all of the data as a BytesMut so no types will need to be given as they will
//all be transformed into bytes
//this also means that a network channel can provide a message size based on its staged cache
//Since all data will be sent as sized messages the network channel should
//keep track of all the different messages it has so that a use can iterate each message
//Lastly the Network channel should contain a method to "chunk" data from a message so
//a use can do things like pull a standard descriptive enum from the front of a message
//Network channels need to keep the protocal they use opaque as both upd and tcp might be used
//This means a network channel should be in a trait
#[derive(Clone)]
pub struct DoubleChannel<T: Clone> {
    //Up:    Sender -   Receiver
    //          |          |
    //Down:  Receiver - Sender
    tx: flume::Sender<T>,
    rx: flume::Receiver<T>,
}

impl<T: Clone> DoubleChannel<T> {
    pub fn new() -> (DoubleChannel<T>, DoubleChannel<T>) {
        let left = flume::unbounded();
        let right = flume::unbounded();
        let end1 = DoubleChannel {
            tx: left.0,
            rx: right.1,
        };
        let end2 = DoubleChannel {
            rx: left.1,
            tx: right.0,
        };
        (end1, end2)
    }
    pub fn tx(&self) -> &flume::Sender<T> {
        &self.tx
    }
    pub fn rx(&self) -> &flume::Receiver<T> {
        &self.rx
    }
}
#[derive(Clone)]
pub enum ServiceMessage {
    NewUdpLink(DoubleChannel<(usize, [u8; 500])>),
    InitiateUdpLink(SocketAddr),
}
pub struct UdpServiceListener {
    lt: LifetimeTreeController,
    link: DoubleChannel<ServiceMessage>,
    socket: Arc<UdpSocket>,
}
impl UdpServiceListener {
    pub fn start<A: ToSocketAddrs + Clone + Copy>(
        bound_address: A,
        rt: &Runtime,
    ) -> UdpServiceListener {
        let channel = DoubleChannel::new();
        let ltc = LifetimeTree::new_tree();
        let lt = ltc.tree.child_from_tree();

        let socket = Arc::new(
            rt.block_on(UdpSocket::bind(bound_address))
                .expect("Could not bind udp socket"),
        );
        rt.spawn(Self::service(
            socket.clone(),
            channel.0.clone(),
            lt.child_from_tree(),
        ));
        UdpServiceListener {
            lt: ltc,
            link: channel.1,
            socket: socket.clone(),
        }
    }
    pub fn stop(self, rt: &Runtime) {
        rt.block_on(self.lt.shutdown());
    }
    pub fn initiate_udp_link(&self, addr: SocketAddr) {
        self.link
            .tx()
            .send(ServiceMessage::InitiateUdpLink(addr))
            .expect("Main udp service not running");
    }
    pub fn get_local_addr(&self) -> Result<SocketAddr, std::io::Error> {
        self.socket.local_addr()
    }
    pub fn get_new_link(&self) -> Option<DoubleChannel<(usize, [u8; 500])>> {
        for msg in self.link.rx().recv() {
            if let ServiceMessage::NewUdpLink(a) = msg {
                return Some(a);
            }
        }
        None
    }
    async fn service(
        socket: Arc<UdpSocket>,
        link: DoubleChannel<ServiceMessage>,
        lt: LifetimeTree,
    ) {
        let mut conn_map = HashMap::new();
        let mut data: [u8; 500] = [0; 500];
        println!("Starting upd service on {:?}", socket.local_addr());
        loop {
            tokio::select! {
                _ = lt.shutdown()=>{
                    println!("Shuting down main Udp Service");
                    break;
                }
                val = socket.recv_from(&mut data)=>{
                   match val {
                        Ok((len, addr)) => {
                            Self::forward_message(socket.clone(), &data, len, addr, &mut conn_map, &link ,&lt);
                        },
                        Err(_) => {
                            Self::handle_message_error();
                        },
                    }
                }
                val = link.rx().recv_async()=>{
                    if let Ok(msg) = val{
                        match msg{
                            ServiceMessage::NewUdpLink(_) => panic!("Should note be receiveing this message here"),
                            ServiceMessage::InitiateUdpLink(a) => Self::forward_message(socket.clone(), &data, 1, a, &mut conn_map, &link, &lt),
                        }
                    }
                }
            }
        }
    }
    fn forward_message(
        socket: Arc<UdpSocket>,
        data: &[u8; 500],
        len: usize,
        addr: SocketAddr,
        conn_map: &mut HashMap<SocketAddr, flume::Sender<(usize, [u8; 500])>>,
        service_link: &DoubleChannel<ServiceMessage>,
        lt: &LifetimeTree,
    ) {
        if let Some(link) = conn_map.get(&addr) {
            if let Err(_) = link.send((len, data.clone())) {
                conn_map.remove(&addr);
            }
        } else {
            let (tx, rx) = flume::unbounded();
            tx.send((len, data.clone())).unwrap();
            let (l, r) = DoubleChannel::new();
            tokio::spawn(Self::start_upd_link(
                socket.clone(),
                rx,
                addr.clone(),
                l,
                lt.child_from_tree(),
            ));
            conn_map.insert(addr, tx);
            service_link
                .tx()
                .send(ServiceMessage::NewUdpLink(r))
                .expect("Service link is broken");
        }
    }
    fn handle_message_error() {}
    async fn start_upd_link(
        socket: Arc<UdpSocket>,
        link: flume::Receiver<(usize, [u8; 500])>,
        addr: SocketAddr,
        dst: DoubleChannel<(usize, [u8; 500])>,
        lt: LifetimeTree,
    ) {
        println!("Starting new upd link with addr {}", addr);
        loop {
            tokio::select! {
                _ = lt.shutdown()=>{
                    break;
                }
                val = link.recv_async()=>{
                    if let Ok((len,bytes)) = val{
                        println!("Message from {}: {:?}",addr, &bytes[..len]);
                        dst.tx().send((len,bytes)).expect("Udp link has no dst");
                    }
                }
                val = dst.rx().recv_async()=>{
                    if let Ok((len, bytes)) = val{
                        println!("Sending data to {}: {:?}",addr, &bytes[..len]);
                        socket.send_to(&bytes[..len], addr).await.expect("Could not send udp packet");
                    }
                }
            }
        }
        println!("Shuting down Udp connection to {}", addr);
    }
}

//Revision 2
//The cluster terminal is the main interface to a connected terminal.
//The cluster terminal will be responisible for managing connections to other cluster
//terminals. The cluster terminal will mantain connection state and perform keep alive
//operations for udp connection channels. The connection terminal will also be
//the entry point for udp inflow and will route messages to the desired destination,
//likey a CommGroup. The ClusterTerminal, as of now, will also provide automatic
//ACK systems should a message request them.

//The CommGroup represents a logical set of communicaton listeners. Essentially
//the CommGroup will contain an arbitratry number of ports that can connect and
//disconnect at will. These ports serve as logical routing targets for CommGroup
//messages.

//Upon receiving a network packet the ClusterTerminal will look at the packets header.
//Which will be [u32][u32][bool][u32][u32][u8; 500] or [messageID][packetIndex][ACK][CommGroupID][portID][u8; 500].
//The messageID represents a WHOLE message, which may or may not be split into multiple packets.
//The messageID is randomly generated by the sender and if it is not recognized a new
//async task is spawned to handle reassembling a whole message as well as ACK handlling
//if requested. If the messageID is already known and active then the packet is just sent
//to that active message handler. The packetIndex is index of the current packet. ACK indicates
//that the sender would like to initialize an ACK dialog.
//CommGroupID is the local destination CommGroup and portID is the destination port within
//that local CommGroup. The message is delivered to the CommGroup as (u32,Vec<u8>) or (portID,data)
//after the message has been assembled and, if requested, ACK'ed.
//From there the message is passed to the port, if possible, rebuilt into a target data type, and then
//made availble to the end of the port, likley a channel, for a user to intake.

//For an AI CommGroup, there would be a port for every AI. Should AI1 wish to talk to AI2 it would
//grab, if known, the AI2 CommPort (which behaves, to the AI, like a regular channel) and send a particular
//data type on that CommPort. This message would be delivered to the CommGroup. Next the CommGroup would
//look to see if port AI2 is a local port. If it is, instead of converting the data type to bytes
//it will just pass it directly to port AI2's channel, completely avoiding the network layer. If port AI2 is
//on a different cluster node then the CommGroup will transform the message into bytes and
//prepare a network transfer by converting the target port into [u32][u32][u32][bool][Vec<u8>]
//or [dstClusterTerminal][CommGroupID][portID][ACK][data] then pass it to the
//Cluster Terminal which will convert dstClusterTerminal into an addr,
//Turn the vec of bytes into a vec of Packets of the form
//[messageID][packetIndex][ACK][CommGroupID][portID][u8; 500], and then send the packets; handling any
//ACK dialog if requested.

//The main network system of the Cluster. By connecting multiple
//of these together over a network we can build out a distributed
//network cluster that, when combined with CommGroups is opaque to a user.
pub struct ClusterTerminal {
    //The main runtime used by all network systems
    rt: Arc<Runtime>,
    //The line of communication with the active network main task
    network_service: DoubleChannel<TerminalMessage>,
    //the set of open comm groups
    //NOTE: If a terminal creates a new comgroup it will randomly generate a new ID
    //instead of selecting the next index in case another terminal
    //is simultaneoulsy creating anothe commgroup. It will then
    //validate that there are no other commgroups using that id, regen+repeat
    //if there is. Then it will start an new commgroup dialog with the
    //other terminals so they also add them to their sets.
    comm_groups: HashMap<u32,CommGroup>,
}
#[derive(Clone)]
//This is the base message that all terminals send to each other.
//This is also use for internal Terminal messages such as the shutdown command 
pub enum TerminalMessage {
    NewNode,
    KeepAlive,
    Terminate,
}
pub struct CommGroup {}
pub struct CommPort {}
