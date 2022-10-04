use std::{
    collections::VecDeque,
    mem::size_of,
    slice::{from_raw_parts, from_raw_parts_mut},
};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use log::debug;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, ToSocketAddrs},
    runtime::{self, Runtime},
    sync::{broadcast, mpsc},
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
pub enum NetworkChannelError {
    Closed,
    NoChunk,
}
#[async_trait::async_trait]
pub trait INetworkChannel {
    fn stage<O: Clone>(&mut self, object: &O);
    fn stage_slice<O: Clone>(&mut self, objects: &[O]);
    fn send(&mut self) -> Result<usize, NetworkChannelError>;
    fn try_chunk<O: Clone>(&mut self) -> Result<O, NetworkChannelError>;
    fn try_drain_chunks<O: Clone>(&mut self, dst: &mut [O]) -> Result<usize, NetworkChannelError>;
    fn drain(&mut self) -> Vec<Bytes>;
}
pub struct NetworkChannel {
    thread_link: (flume::Sender<Bytes>, flume::Receiver<Bytes>),
    message_stage: BytesMut,
    recieved_messages: VecDeque<Bytes>,
}
#[async_trait::async_trait]
impl INetworkChannel for NetworkChannel {
    fn stage<O: Clone>(&mut self, object: &O) {
        let data = unsafe { from_raw_parts((object as *const O) as *const u8, size_of::<O>()) };
        self.message_stage.put_slice(data);
    }

    fn stage_slice<O: Clone>(&mut self, objects: &[O]) {
        let data = unsafe {
            from_raw_parts(
                objects.as_ptr() as *const u8,
                size_of::<O>() * objects.len(),
            )
        };
        self.message_stage.put_slice(data);
    }

    fn send(&mut self) -> Result<usize, NetworkChannelError> {
        if let Err(_) = self.thread_link.0.send(self.message_stage.clone().into()) {
            Err(NetworkChannelError::Closed)
        } else {
            Ok(self.message_stage.len())
        }
    }

    fn try_chunk<O: Clone + Sized>(&mut self) -> Result<O, NetworkChannelError> {
        if self.thread_link.1.is_disconnected() {
            return Err(NetworkChannelError::Closed);
        }
        for m in self.thread_link.1.try_recv() {
            self.recieved_messages.push_front(m);
        }
        let mut message = match self.recieved_messages.pop_front() {
            Some(r) => r,
            None => return Err(NetworkChannelError::NoChunk),
        };
        let data = message.copy_to_bytes(size_of::<O>());
        assert_eq!(data.len(), size_of::<O>());
        let object = unsafe { from_raw_parts(data.as_ptr() as *const O, size_of::<O>()) }
            .first()
            .unwrap()
            .clone();
        if message.remaining() > 0 {
            self.recieved_messages.push_front(message);
        }

        Ok(object.clone())
    }

    fn try_drain_chunks<O: Clone>(&mut self, dst: &mut [O]) -> Result<usize, NetworkChannelError> {
        if self.thread_link.1.is_disconnected() {
            return Err(NetworkChannelError::Closed);
        }
        for m in self.thread_link.1.try_recv() {
            self.recieved_messages.push_front(m);
        }
        let mut message = match self.recieved_messages.pop_front() {
            Some(r) => r,
            None => return Err(NetworkChannelError::NoChunk),
        };
        if message.len() < dst.len() * size_of::<O>() {
            return Err(NetworkChannelError::NoChunk);
        }
        for (index, chunk) in message.chunks_exact(size_of::<O>()).enumerate() {
            let object = unsafe { from_raw_parts(chunk.as_ptr() as *const O, size_of::<O>()) }
                .first()
                .unwrap()
                .clone();
            *dst.get_mut(index).expect("Drain slice does not have index") = object;
        }
        Ok(1)
    }

    fn drain(&mut self) -> Vec<Bytes> {
        let messages = self
            .recieved_messages
            .drain(0..self.recieved_messages.len());
        messages.collect()
    }
}

enum NetworkChannelServiceMessage {
    InboundEstablishment(TcpStream),
    OutboundEstablishment(String, flume::Sender<Bytes>, flume::Receiver<Bytes>),
    NewChannel(NetworkChannel),
}
pub struct NetworkChannelService {
    lifetime: LifetimeTree,
    channel: (
        flume::Sender<NetworkChannelServiceMessage>,
        flume::Receiver<NetworkChannelServiceMessage>,
    ),
}
impl NetworkChannelService {
    pub fn new(lifetime: LifetimeTree) -> NetworkChannelService{
        NetworkChannelService { lifetime, channel: flume::unbounded() }
    }
    async fn start(self){
        let tree_control = LifetimeTree::new_tree();
        loop{
            tokio::select!{
                val = self.lifetime.shutdown() => {
                    break;
                }
                message = self.channel.1.recv_async() => {
                    let message = message.expect("The network channel service should not shutdown after its creator");
                    match message {
                        NetworkChannelServiceMessage::InboundEstablishment(s) => {
                            debug!("Received new network channel request");
                            let c1 = flume::unbounded();
                            let c2 = flume::unbounded();
                            // need to absract the concept of a double channel
                            let thread_link = (c1.0, c2.1);
                            let user_link = (c2.0, c1.1);
                            
                            let link = NetworkChannelLink::new(tree_control.tree.child_from_tree(), user_link);
                            
                            tokio::spawn(link.start());
                        },
                        NetworkChannelServiceMessage::OutboundEstablishment(a, s, r) => todo!(),
                        NetworkChannelServiceMessage::NewChannel(_) => panic!("The network channel service should not be getting NewChannel messages"),
                    }
                }
            }
        }
        tree_control.shutdown().await;
    }
}
pub struct NetworkChannelLink{
    lifetime: LifetimeTree,
    thread_link: (flume::Sender<Bytes>, flume::Receiver<Bytes>),
}
impl NetworkChannelLink{
    pub fn new(lifetime: LifetimeTree, user_link: (flume::Sender<Bytes>, flume::Receiver<Bytes>)) -> NetworkChannelLink{
        
    }
    async fn start(self){}
}

pub struct QServer {
    rt: Runtime,
    tree: LifetimeTreeController,
}

impl QServer {
    pub fn new<T: ToSocketAddrs>(bindpoint: T) -> QServer {
        let rt = runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        let tree = LifetimeTree::new_tree();

        QServer { rt, tree }
    }
    pub fn shutdown(self) {
        self.rt.block_on(self.tree.shutdown());
    }
}
