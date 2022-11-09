use std::{
    collections::HashMap, mem::size_of, net::SocketAddr, slice::from_raw_parts, sync::Arc,
    time::Duration,
};

use tokio::{
    net::UdpSocket,
    runtime::Runtime,
    sync::RwLock,
    time::{sleep, Instant},
};

use rand::{thread_rng, Rng};

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
pub trait Transferable {
    fn to_transfer(&self) -> &[u8];
}
pub struct ClusterTerminal {
    //The main runtime used by all network systems
    rt: Arc<Runtime>,
    public: bool,
    //The line of communication with the active network main task
    socket: SocketHandler,
    network_terminate: TerminateSignal,
    terminal_map: TerminalAddressMap,
    //the set of open comm groups
    //NOTE: If a terminal creates a new comgroup it will randomly generate a new ID
    //instead of selecting the next index in case another terminal
    //is simultaneoulsy creating another commgroup. It will then
    //validate that there are no other commgroups using that id, regen+repeat
    //if there is. Then it will start an new commgroup dialog with the
    //other terminals so they also add them to their sets.
}
#[derive(Clone, Debug)]
#[repr(C)]
//This is the base message that all terminals send to each other.
//This is also used for internal Terminal messages such as the shutdown command
pub enum TerminalMessage {
    //Bool tells if this is a public terminal
    KeepAlive(bool),
    //
    ClusterNodeAddr(SocketAddr),
}
pub struct CommGroup {}
pub struct CommPort {}
#[derive(Clone)]
pub struct TerminateSignal {
    channel: (flume::Sender<bool>, flume::Receiver<bool>),
}
#[derive(Clone)]
pub struct TerminalAddressMap {
    active_connections: Arc<RwLock<HashMap<SocketAddr, Arc<RwLock<TerminalConnection>>>>>,
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
pub struct SocketHandler {
    socket: Arc<UdpSocket>,
}
type SocketMessage = (usize, [u8; 1024], SocketAddr);
const MESSAGE_FRAGMENT_DATA_SIZE: usize = 1024 - 8 - 8 - 8 - 4 - 4 - 4;

#[derive(Clone)]
#[repr(C)]
//The Message Fragment is the base unit of communication for the network layer
struct MessageFragment {
    //The total length of this message fragment
    data_length: u64,
    //The combined length of all fragments
    total_message_length: u64,
    //A randomly generated values for message stream identification
    message_id: u64,
    //Total fragments that make up this ONE message
    fragment_total: u32,
    //This fragments id
    fragment_id: u32,
    //Does this message need to be reliable
    nak: bool,
    //The data
    data: [u8; MESSAGE_FRAGMENT_DATA_SIZE],
}
struct MessageIdMap {
    live_ids: Arc<
        RwLock<HashMap<u64, Arc<(flume::Sender<SocketMessage>, flume::Receiver<SocketMessage>)>>>,
    >,
}

impl MessageIdMap {
    fn new() -> MessageIdMap {
        MessageIdMap {
            live_ids: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}
impl MessageFragment {
    fn from_data(data: &[u8]) -> MessageFragment {
        let ptr = data.as_ptr();
        assert!(size_of::<Self>() <= data.len());
        unsafe { from_raw_parts(ptr as *const MessageFragment, 1)[0].clone() }
    }
    fn to_data(&self) -> Vec<u8> {
        let data = self as *const Self;
        unsafe { from_raw_parts(data as *const u8, size_of::<Self>()) }.to_vec()
    }
}
impl ClusterTerminal {
    //Starts the network system and returns the ClusterTerminal object.
    //Which is essentially an interface to the running network async tasks.
    pub fn new(socket_addr: SocketAddr, public: bool) -> Self {
        let rt = Arc::new(Runtime::new().unwrap());
        let socket = SocketHandler::new(socket_addr, rt.clone());
        let root_terminate = TerminateSignal::new();
        let terminal_map = TerminalAddressMap::new();
        rt.spawn(Self::udp_listener(
            terminal_map.clone(),
            socket.clone(),
            root_terminate.subscribe(),
            public,
        ));
        println!(
            "Starting new public:{} cluster terminal on address {}",
            public,
            socket.local_address()
        );
        ClusterTerminal {
            rt,
            socket,
            network_terminate: root_terminate,
            terminal_map,
            public,
        }
    }
    pub fn address(&self) -> SocketAddr {
        self.socket.local_address()
    }
    async fn udp_listener(
        terminal_map: TerminalAddressMap,
        socket: SocketHandler,
        terminate: TerminateSignal,
        public: bool,
    ) {
        //Upon receiving a message
        loop {
            tokio::select! {
                _ = terminate.terminated()=>{println!("Terminating udp listener");break;}
                mesg = socket.receive()=>{
                    tokio::spawn(TerminalConnection::receive(terminal_map.clone(), mesg.clone(), socket.clone(), public));
                }
            }
        }
    }
    //Will attempt to join a cluster. Whether we want to be discoverable to other machines is given by public
    pub fn join_cluster(&self, tgt: SocketAddr) {
        self.rt.spawn(TerminalConnection::connect_to(
            self.terminal_map.clone(),
            tgt,
            self.socket.clone(),
            self.public,
        ));
    }
    // pub fn comm_group_test(&self, addr: SocketAddr) {
    //     self.rt.block_on(
    //         self.socket
    //             .send(addr, TerminalMessage::Data(1, 2, [60; 483]).to_transfer()),
    //     );
    // }
    pub fn stop(self) {
        drop(self.network_terminate);
    }
}
impl TerminalAddressMap {
    fn new() -> TerminalAddressMap {
        TerminalAddressMap {
            active_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    async fn remove_connection(map: TerminalAddressMap, addr: SocketAddr) {
        let mut writer = map.active_connections.write().await;
        writer.remove(&addr);
    }
    //Will either get a connection or add and return a new connections if one does not yet exists for the addr provided
    async fn add(
        terminal_map: TerminalAddressMap,
        terminal_addr: SocketAddr,
        new_terminal: Arc<RwLock<TerminalConnection>>,
    ) {
        //We must first notify the new terminal of all known public terminals, then notify all known private terminals of this new one
        //if its public
        let mut terminal_map = terminal_map.active_connections.write().await;
        if let None = terminal_map.get(&terminal_addr) {
            {
                let new_terminal = new_terminal.read().await;
                for known_terminal in terminal_map.values() {
                    let known_terminal = known_terminal.read().await;
                    //Notifiing the new terminal of all known public terminals
                    if known_terminal.is_public {
                        new_terminal
                            .socket
                            .send(
                                new_terminal.addr,
                                TerminalMessage::ClusterNodeAddr(known_terminal.addr).to_transfer(),
                            )
                            .await;
                    } else if new_terminal.is_public {
                        //Notifiying all known private terminals of the new public one
                        known_terminal
                            .socket
                            .send(
                                known_terminal.addr,
                                TerminalMessage::ClusterNodeAddr(new_terminal.addr).to_transfer(),
                            )
                            .await;
                    }
                }
            }
            terminal_map.insert(terminal_addr, new_terminal.clone());
        } else {
            println!("Attempting to add already existing terminal connection");
        }
    }

    //Try's to get an exisiting connection
    async fn try_get(
        map: TerminalAddressMap,
        addr: SocketAddr,
    ) -> Option<Arc<RwLock<TerminalConnection>>> {
        let read = map.active_connections.read().await;
        if let Some(tdata) = read.get(&addr) {
            Some(tdata.clone())
        } else {
            None
        }
    }
    //Will wait for the terminal from addr to be added to the map up until timout_millis has passed
    async fn wait_get(
        map: TerminalAddressMap,
        addr: SocketAddr,
        timeout_millis: u32,
    ) -> Option<Arc<RwLock<TerminalConnection>>> {
        let mut time_spent = 0;
        loop {
            {
                let read = map.active_connections.read().await;
                if let Some(terminal) = read.get(&addr) {
                    return Some(terminal.clone());
                }
            }
            sleep(Duration::from_millis(1000)).await;
            time_spent += 1000;
            if time_spent > timeout_millis {
                return None;
            }
        }
    }
}
impl TerminalConnection {
    async fn connect_to(
        terminal_map: TerminalAddressMap,
        tgt_addr: SocketAddr,
        socket: SocketHandler,
        public: bool,
    ) -> Arc<RwLock<TerminalConnection>> {
        if let Some(tgt) = TerminalAddressMap::try_get(terminal_map.clone(), tgt_addr).await {
            tgt
        } else {
            println!("Connecting to public:{} terminal {}", true, tgt_addr);
            let lt = Arc::new(TerminateSignal::new());
            //If we are connecting to something then it must be public.
            //Private terminals can ONLY start connections not receive them
            let tgt = TerminalConnection {
                is_public: true,
                addr: tgt_addr,
                socket,
                terminal_map: terminal_map.clone(),
                keep_alive_time: Instant::now(),
                life: lt,
            };
            let tgt = Arc::new(RwLock::new(tgt));
            TerminalAddressMap::add(terminal_map, tgt_addr, tgt.clone()).await;
            tokio::spawn(Self::keep_alive(tgt.clone(), public));
            tgt
        }
    }
    async fn connect_from(
        terminal_map: TerminalAddressMap,
        tgt_addr: SocketAddr,
        socket: SocketHandler,
        public: bool,
    ) -> Arc<RwLock<TerminalConnection>> {
        if let Some(tgt) = TerminalAddressMap::try_get(terminal_map.clone(), tgt_addr).await {
            tgt
        } else {
            println!("Connecting from public:{} terminal {}", public, tgt_addr);
            let lt = Arc::new(TerminateSignal::new());
            let tgt = TerminalConnection {
                is_public: public,
                addr: tgt_addr,
                socket,
                terminal_map: terminal_map.clone(),
                keep_alive_time: Instant::now(),
                life: lt,
            };
            let tgt = Arc::new(RwLock::new(tgt));
            TerminalAddressMap::add(terminal_map, tgt_addr, tgt.clone()).await;
            //Since we have RECIEVED a connection, we must be a public terminal
            tokio::spawn(Self::keep_alive(tgt.clone(), true));
            tgt
        }
    }
    async fn send_message_fragments(nak: bool, message: Vec<u8>) {
        let message_id = thread_rng().gen::<u64>();
        //First thing we need to do is split the message into fragments
        let mut fragments = vec![];
        let chunks = message.chunks(MESSAGE_FRAGMENT_DATA_SIZE);
        let fragment_total = chunks.len() as u32;
        println!(
            "Splitting message of size {} into {} fragments",
            message.len(),
            fragment_total
        );
        let fragment_id = 0;
        for chunk in chunks {
            let mut data: [u8; MESSAGE_FRAGMENT_DATA_SIZE] = [0; MESSAGE_FRAGMENT_DATA_SIZE];
            //Chunk len will be 1024-8-8-8-4-4-4 or smaller
            for index in 0..chunk.len() {
                data[index] = chunk[index];
            }

            let fragment = MessageFragment {
                data_length: chunk.len() as u64,
                total_message_length: message.len() as u64,
                message_id,
                fragment_total,
                fragment_id,
                nak,
                data,
            };

            println!(
                "Broke message into fragement {} with data size {}",
                fragment.fragment_id, fragment.data_length
            );
            fragments.push(fragment);
        }

        //Now we must begin the transfer operation
    }
    async fn receive_message_fragments() {}
    async fn read_message() {}
    async fn receive(
        terminal_map: TerminalAddressMap,
        message: SocketMessage,
        socket: SocketHandler,
        public: bool,
    ) {
        let data;
        {
            let ptr = &message.1 as *const u8;
            data = unsafe { from_raw_parts(ptr as *const TerminalMessage, 1)[0].clone() };
        }
        match data {
            TerminalMessage::KeepAlive(public) => {
                println!(
                    "Recieved keep alive from public:{} terminal {}",
                    public, message.2
                );
                let terminal = TerminalConnection::connect_from(
                    terminal_map.clone(),
                    message.2,
                    socket.clone(),
                    public,
                )
                .await;
                let mut terminal = terminal.write().await;
                terminal.keep_alive_time = Instant::now();
            }
            TerminalMessage::ClusterNodeAddr(tgt_addr) => {
                println!(
                    "Recieved terminal address {} from terminal {}",
                    tgt_addr, message.2
                );
                Self::connect_to(terminal_map.clone(), tgt_addr, socket.clone(), public).await;
            }
        }
    }
    //This task will provide keep alive functionality as well as send the terminate connection signal
    //Turn all access to a Terminal Connection into an Arc access. If the keep_alive system timesout
    //we pull the terminal connection from the hashmap. Doing so will also end the root lifetime for
    //all of its child tasks.
    async fn keep_alive(tgt: Arc<RwLock<TerminalConnection>>, public: bool) {
        let reader = tgt.read().await;
        let life = reader.life.subscribe();
        drop(reader);
        //We just need to send a keep alive enum every so often
        loop {
            tokio::select! {
                _ = life.terminated()=>{}
                _ = sleep(Duration::from_millis(1000))=>{
                    let reader = tgt.read().await;
                    reader.socket.send(reader.addr,TerminalMessage::KeepAlive(public).to_transfer()).await;
                    if Instant::now()-reader.keep_alive_time > Duration::from_millis(10000){
                        TerminalAddressMap::remove_connection(reader.terminal_map.clone(),reader.addr).await;
                        break;
                    }
                }
            }
        }
    }
}
impl Drop for TerminalConnection {
    fn drop(&mut self) {
        println!("Terminating connection to {}", self.addr);
    }
}
impl SocketHandler {
    fn new(socket_addr: SocketAddr, rt: Arc<Runtime>) -> SocketHandler {
        let socket = rt.block_on(UdpSocket::bind(socket_addr)).unwrap();
        SocketHandler {
            socket: Arc::new(socket),
        }
    }
    async fn receive(&self) -> SocketMessage {
        let mut data = [0; 1024];
        loop {
            if let Ok((len, addr)) = self.socket.recv_from(&mut data).await {
                return (len, data, addr);
            }
        }
    }
    async fn send(&self, tgt: SocketAddr, data: &[u8]) {
        self.socket.send_to(data, tgt).await.unwrap();
    }
    fn local_address(&self) -> SocketAddr {
        self.socket.local_addr().unwrap()
    }
}
impl Transferable for TerminalMessage {
    fn to_transfer(&self) -> &[u8] {
        let ptr = self as *const Self;
        let data = unsafe { std::slice::from_raw_parts(ptr as *const u8, size_of::<Self>()) };
        data
    }
}
impl TerminateSignal {
    fn new() -> TerminateSignal {
        TerminateSignal {
            channel: flume::bounded(1),
        }
    }
    fn subscribe(&self) -> TerminateSignal {
        let rx = self.channel.1.clone();
        let tx = flume::bounded(1).0;
        TerminateSignal { channel: (tx, rx) }
    }
    async fn terminated(&self) {
        let _ = self.channel.1.recv_async().await;
    }
}
