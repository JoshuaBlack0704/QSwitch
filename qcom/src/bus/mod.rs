
use std::{
    collections::HashMap,
    sync::Arc, time::Instant,
};
use tokio::sync::RwLock;
use rand::{self, thread_rng, Rng};

#[derive(Clone, Copy)]
pub enum Channel {
    Channel(u32),
    All,
}
pub trait BusProducer<M>: Send + Sync {
    fn accepts_message(&self, msg: &M) -> bool;
    fn handle_message(&self, src: &dyn Bus<M>, msg: &M) -> Option<M>;
}
pub trait BusListener<M>: Send + Sync{
    fn handle_transaction(&self, src: &dyn Bus<M>, transaction: &BusTransaction<M>);
}
pub trait BustInterceptor<M>: Send + Sync{
    fn accepts_message(&self, msg: &M) -> bool;
    fn intercept_message(&self, src: &dyn Bus<M>, msg: M) -> M; 
}

#[derive(Clone)]
pub enum BusTransaction<M>{
    Broadcast(BroadcastTransaction<M>),
    Intercept(InterceptionTransaction<M>),
}

#[derive(Clone)]
pub struct BroadcastTransaction<M>{
    pub bus_uuid: u64,
    pub transaction_uuid: u64,
    pub instant: Instant,
    pub channel: Channel,
    pub queried_producers: Vec<Arc<dyn BusProducer<M>>>,
    pub accepted_producers: Vec<Arc<dyn BusProducer<M>>>,
    pub msg: M,
    pub replies: Vec<M>,
}

#[derive(Clone)]
pub struct InterceptionTransaction<M>{
    pub bus_uuid: u64,
    pub transaction_uuid: u64,
    pub instant: Instant,
    pub channel: Channel,
    pub interceptor: Arc<dyn BustInterceptor<M>>,
    pub original_msg: M,
    pub mutated_msg: M,
}

pub trait Bus<M: Clone>: Send + Sync {
    fn bind_producer(&self, producer: Arc<dyn BusProducer<M>>, channel: u32);
    fn bind_listener(&self, listener: Arc<dyn BusListener<M>>, channel: u32);
    fn bind_interceptor(&self, interceptor: Arc<dyn BustInterceptor<M>>, channel: u32);
    
    /// Given a channel, should return all matching producers
    fn get_producers(&self, channel: Channel) -> Vec<Arc<dyn BusProducer<M>>>;
    /// Given a channel, should return all matching listeners
    fn get_listeners(&self, channel: Channel) -> Vec<Arc<dyn BusListener<M>>>;
    /// Given a channel, should return all matching interceptors
    fn get_interceptors(&self, channel: Channel) -> Vec<Arc<dyn BustInterceptor<M>>>;
    /// Should transform self into a trait object
    fn as_trait_object(&self) -> &dyn Bus<M>;
    /// You decide what the uuid represents
    fn get_uuid(&self) -> u64;

    fn notify_listeners(&self, transaction: &BusTransaction<M>, channel: Channel){
        let listeners = self.get_listeners(channel);
        for l in listeners.iter(){
            l.handle_transaction(self.as_trait_object(), transaction);
        }
    }

    fn intercept_message(&self, mut msg: M, channel: Channel, external_trans_id: Option<u64>) -> M{
        let interceptors = self.get_interceptors(channel);
        let _msg = msg.clone();
        for i in interceptors.iter().filter(|i| i.accepts_message(&_msg)){
            let original_msg = msg.clone();
            msg = i.intercept_message(self.as_trait_object(), msg.clone());
            let transaction_uuid = match external_trans_id{
                Some(uuid) => uuid,
                None => thread_rng().gen::<u64>(),
            };
            let transaction = BusTransaction::Intercept(InterceptionTransaction::<M>{
                bus_uuid: self.get_uuid(),
                transaction_uuid,
                instant: Instant::now(),
                channel,
                interceptor: i.clone(),
                original_msg,
                mutated_msg: msg.clone(),
            });

            self.notify_listeners(&transaction, channel);
        }
        msg
    }

    /// Will send msg to all producers that match the channel query and return any responses
    fn broadcast(&self, mut msg: M, channel: Channel) -> BroadcastTransaction<M>{
        let transaction_uuid = thread_rng().gen::<u64>();
        msg = self.intercept_message(msg, channel, Some(transaction_uuid));
        let producers = self.get_producers(channel);
        let accepted:Vec<Arc<dyn BusProducer<M>>> = producers.iter().filter(|p| p.accepts_message(&msg)).map(|p| p.clone()).collect();
        let mut replies = Vec::with_capacity(producers.len());

        for p in accepted.iter(){
            if let Some(response) = p.handle_message(self.as_trait_object(), &msg){
                replies.push(response);
            }
        }

        
        let transaction = BroadcastTransaction::<M>{
            bus_uuid: self.get_uuid(),
            transaction_uuid,
            instant: Instant::now(),
            channel,
            queried_producers: producers.clone(),
            accepted_producers: accepted.clone(),
            msg,
            replies: replies.clone(),
        };

        let ltrans = BusTransaction::Broadcast(transaction.clone());

        self.notify_listeners(&ltrans, channel);

        transaction
    }

    
}
pub struct BusSystem<M> {
    uuid: u64,
    producers: RwLock<HashMap<u32, Vec<Arc<dyn BusProducer<M>>>>>,
    listeners: RwLock<HashMap<u32, Vec<Arc<dyn BusListener<M>>>>>,
    interceptors: RwLock<HashMap<u32, Vec<Arc<dyn BustInterceptor<M>>>>>,
}

impl<M> BusSystem<M>{
    pub fn new() -> Arc<BusSystem<M>> {
        Arc::new(
            Self{
                producers: RwLock::new(HashMap::new()),
                listeners: RwLock::new(HashMap::new()),
                interceptors: RwLock::new(HashMap::new()),
                uuid: thread_rng().gen::<u64>(),
            }
        )
    }
    pub fn bind_producer(&self, producer: Arc<dyn BusProducer<M>>, channel: u32){
        let mut producers = self.producers.blocking_write();
        if let Some(ps) = producers.get_mut(&channel){
            ps.push(producer);
        }
        else{
            producers.insert(channel, vec![producer]);
        }
        
    }
    pub fn bind_listener(&self, listener: Arc<dyn BusListener<M>>, channel: u32){
        let mut listeners = self.listeners.blocking_write();
        if let Some(ps) = listeners.get_mut(&channel){
            ps.push(listener);
        }
        else{
            listeners.insert(channel, vec![listener]);
        }
        
    }
    pub fn bind_interceptor(&self, interceptor: Arc<dyn BustInterceptor<M>>, channel: u32){
        let mut interceptors = self.interceptors.blocking_write();
        if let Some(ps) = interceptors.get_mut(&channel){
            ps.push(interceptor);
        }
        else{
            interceptors.insert(channel, vec![interceptor]);
        }
        
    }
}

impl<M: Clone + Send + Sync> Bus<M> for Arc<BusSystem<M>> {
    fn bind_producer(&self, producer: Arc<dyn BusProducer<M>>, channel: u32){
        let mut producers = self.producers.blocking_write();
        if let Some(ps) = producers.get_mut(&channel){
            ps.push(producer);
        }
        else{
            producers.insert(channel, vec![producer]);
        }
        
    }
    fn bind_listener(&self, listener: Arc<dyn BusListener<M>>, channel: u32){
        let mut listeners = self.listeners.blocking_write();
        if let Some(ps) = listeners.get_mut(&channel){
            ps.push(listener);
        }
        else{
            listeners.insert(channel, vec![listener]);
        }
        
    }
    fn bind_interceptor(&self, interceptor: Arc<dyn BustInterceptor<M>>, channel: u32){
        let mut interceptors = self.interceptors.blocking_write();
        if let Some(ps) = interceptors.get_mut(&channel){
            ps.push(interceptor);
        }
        else{
            interceptors.insert(channel, vec![interceptor]);
        }
        
    }
    fn get_producers(&self, channel: Channel) -> Vec<Arc<dyn BusProducer<M>>> {
        let producers = self.producers.blocking_read();
        match channel{
            Channel::Channel(c) => {
                if let Some(ps) = producers.get(&c){
                    return ps.clone();
                }

                vec![]
            },
            Channel::All => {
                let mut ps = Vec::with_capacity(producers.len());
                for p in producers.values(){
                    ps.extend_from_slice(p);
                }
                ps
            },
        }

        
    }

    fn get_listeners(&self, channel: Channel) -> Vec<Arc<dyn BusListener<M>>> {
        let listeners = self.listeners.blocking_read();
        match channel{
            Channel::Channel(c) => {
                if let Some(ps) = listeners.get(&c){
                    return ps.clone();
                }

                vec![]
            },
            Channel::All => {
                let mut ps = Vec::with_capacity(listeners.len());
                for p in listeners.values(){
                    ps.extend_from_slice(p);
                }
                ps
            },
        }

    }

    fn as_trait_object(&self) -> &dyn Bus<M> {
        self
    }

    fn get_interceptors(&self, channel: Channel) -> Vec<Arc<dyn BustInterceptor<M>>> {
        let interceptors = self.interceptors.blocking_read();
        match channel{
            Channel::Channel(c) => {
                if let Some(ps) = interceptors.get(&c){
                    return ps.clone();
                }

                vec![]
            },
            Channel::All => {
                let mut ps = Vec::with_capacity(interceptors.len());
                for p in interceptors.values(){
                    ps.extend_from_slice(p);
                }
                ps
            },
        }
    }

    fn get_uuid(&self) -> u64 {
        self.uuid
    }
}
