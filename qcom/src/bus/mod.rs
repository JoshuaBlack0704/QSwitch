use async_trait::async_trait;
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::RwLock;

pub type CHANNEL = usize;
pub trait BusProducer<M>: Send + Sync {
    fn accpects_message(&self, msg: &M) -> bool;
    fn handle_message(&self, src: &dyn Bus<M>, msg: M) -> Option<M>;
}
pub trait BusListener<M>: Send + Sync{
    fn handle_message(&self, src: &dyn Bus<M>, msg: M);
}

#[async_trait]
pub trait Bus<M> {
    /// Adds a producer to the bus
    fn bind_producer(&self, producer: Arc<dyn BusProducer<M>>, channel: Option<CHANNEL>);
    /// Async adds a producer to the bus
    async fn bind_producer_async(&self, producer: Arc<dyn BusProducer<M>>, channel: Option<CHANNEL>);
    
    /// Adds a passive listener to the bus
    fn bind_listener(&self, listener: Arc<dyn BusListener<M>>, channel: Option<CHANNEL>);
    /// Async adds a producer to the bus
    async fn bind_listener_async(&self, listener: Arc<dyn BusListener<M>>, channel: Option<CHANNEL>);

    /// Removes all bounded producers
    fn clear(&self);
    /// Async Removes all bounded producers
    async fn clear_async(&self);

    /// Tests all bindings in the bus for response willingness, stopping and returning the response of the first to
    /// accept the message
    fn send(&self, msg: M, channel: Option<CHANNEL>) -> Option<M>;
    /// Async Tests all bindings in the bus for response willingness, stopping and returning the response of the first to
    /// accept the message
    async fn send_async(&self, msg: M, channel: Option<CHANNEL>) -> Option<M>;

    /// Sends msg to all bindings, returning any respones it gets
    fn broadcast(&self, msg: M, channel: Option<CHANNEL>) -> Vec<M>;
    /// Async Sends msg to all bindings, returning any respones it gets
    async fn broadcast_async(&self, msg: M, channel: Option<CHANNEL>) -> Vec<M>;

    /// Gets all producers who say they can handle the message
    fn get_acceptors(&self, msg: M, channel: Option<CHANNEL>) -> Vec<Arc<dyn BusProducer<M>>>;
    /// Async Gets all producers who say they can handle the message
    async fn get_acceptors_async(
        &self,
        msg: M,
        channel: Option<CHANNEL>,
    ) -> Vec<Arc<dyn BusProducer<M>>>;
}
pub struct BusSystem<M> {
    channeled_producers: RwLock<HashMap<CHANNEL, Vec<Arc<dyn BusProducer<M>>>>>,
    producers: RwLock<Vec<Arc<dyn BusProducer<M>>>>,
    channeled_listeners: RwLock<HashMap<CHANNEL, Vec<Arc<dyn BusListener<M>>>>>,
    listeners: RwLock<Vec<Arc<dyn BusListener<M>>>>,
}

#[async_trait]
impl<M: Clone + Send + Sync> Bus<M> for Arc<BusSystem<M>> {
    fn bind_producer(&self, producer: Arc<dyn BusProducer<M>>, channel: Option<CHANNEL>) {
        if let Some(c) = channel {
            let mut channels = self.channeled_producers.blocking_write();
            if let Some(elements) = channels.get_mut(&c) {
                elements.push(producer);
                return;
            }
            let _ = channels.insert(c, vec![producer]);
            return;
        }

        let mut elements = self.producers.blocking_write();
        elements.push(producer);
    }

    async fn bind_producer_async(&self, producer: Arc<dyn BusProducer<M>>, channel: Option<CHANNEL>) {
        if let Some(c) = channel {
            let mut channels = self.channeled_producers.write().await;
            if let Some(elements) = channels.get_mut(&c) {
                elements.push(producer);
                return;
            }
            let _ = channels.insert(c, vec![producer]);
            return;
        }

        let mut elements = self.producers.write().await;
        elements.push(producer);
    }

    fn bind_listener(&self, listener: Arc<dyn BusListener<M>>, channel: Option<CHANNEL>) {
        if let Some(c) = channel {
            let mut channels = self.channeled_listeners.blocking_write();
            if let Some(listeners) = channels.get_mut(&c) {
                listeners.push(listener);
                return;
            }
            let _ = channels.insert(c, vec![listener]);
            return;
        }

        let mut listeners = self.listeners.blocking_write();
        listeners.push(listener);
    }

    async fn bind_listener_async(&self, listener: Arc<dyn BusListener<M>>, channel: Option<CHANNEL>) {
        if let Some(c) = channel {
            let mut channels = self.channeled_listeners.write().await;
            if let Some(listeners) = channels.get_mut(&c) {
                listeners.push(listener);
                return;
            }
            let _ = channels.insert(c, vec![listener]);
            return;
        }

        let mut listeners = self.listeners.write().await;
        listeners.push(listener);
    }

    fn clear(&self) {
        self.channeled_producers.blocking_write().clear();
        self.producers.blocking_write().clear();
    }

    async fn clear_async(&self) {
        self.channeled_producers.write().await.clear();
        self.producers.write().await.clear();
    }

    fn send(&self, msg: M, channel: Option<CHANNEL>) -> Option<M> {
        if let Some(c) = channel {
            let channels = self.channeled_producers.blocking_read();
            if let Some(p) = channels.get(&c) {
                for p in p.iter() {
                    if p.accpects_message(&msg) {
                        return p.handle_message(self, msg);
                    }
                }
            }
        }

        let producers = self.producers.blocking_read();
        for p in producers.iter() {
            if p.accpects_message(&msg) {
                return p.handle_message(self, msg);
            }
        }

        None
    }

    async fn send_async(&self, msg: M, channel: Option<CHANNEL>) -> Option<M> {
        if let Some(c) = channel {
            let channels = self.channeled_producers.read().await;
            if let Some(p) = channels.get(&c) {
                for p in p.iter() {
                    if p.accpects_message(&msg) {
                        return p.handle_message(self, msg);
                    }
                }
            }
            return None;
        }

        let producers = self.producers.read().await;
        for p in producers.iter() {
            if p.accpects_message(&msg) {
                return p.handle_message(self, msg);
            }
        }

        None
    }

    fn broadcast(&self, msg: M, channel: Option<CHANNEL>) -> Vec<M> {
        let mut responses = vec![];
        if let Some(c) = channel {
            let channels = self.channeled_producers.blocking_read();
            if let Some(p) = channels.get(&c) {
                for p in p.iter() {
                    if p.accpects_message(&msg) {
                        if let Some(r) = p.handle_message(self, msg.clone()) {
                            responses.push(r);
                        }
                    }
                }
            }
            return responses;
        }

        let producers = self.producers.blocking_read();
        for p in producers.iter() {
            if p.accpects_message(&msg) {
                if let Some(r) = p.handle_message(self, msg.clone()) {
                    responses.push(r);
                }
            }
        }

        responses
    }

    async fn broadcast_async(&self, msg: M, channel: Option<CHANNEL>) -> Vec<M> {
        let mut responses = vec![];
        if let Some(c) = channel {
            let channels = self.channeled_producers.read().await;
            if let Some(p) = channels.get(&c) {
                for p in p.iter() {
                    if p.accpects_message(&msg) {
                        if let Some(r) = p.handle_message(self, msg.clone()) {
                            responses.push(r);
                        }
                    }
                }
            }
            return responses;
        }

        let producers = self.producers.read().await;
        for p in producers.iter() {
            if p.accpects_message(&msg) {
                if let Some(r) = p.handle_message(self, msg.clone()) {
                    responses.push(r);
                }
            }
        }

        responses
    }

    fn get_acceptors(&self, msg: M, channel: Option<CHANNEL>) -> Vec<Arc<dyn BusProducer<M>>> {
        let mut eligible_producers = vec![];
        if let Some(c) = channel {
            let channels = self.channeled_producers.blocking_read();
            if let Some(p) = channels.get(&c) {
                for p in p.iter() {
                    if p.accpects_message(&msg) {
                        eligible_producers.push(p.clone())
                    }
                }
            }
            return eligible_producers;
        }

        let producers = self.producers.blocking_read();
        for p in producers.iter() {
            if p.accpects_message(&msg) {
                eligible_producers.push(p.clone());
            }
        }

        eligible_producers
    }

    async fn get_acceptors_async(
        &self,
        msg: M,
        channel: Option<CHANNEL>,
    ) -> Vec<Arc<dyn BusProducer<M>>> {
        let mut eligible_producers = vec![];
        if let Some(c) = channel {
            let channels = self.channeled_producers.read().await;
            if let Some(p) = channels.get(&c) {
                for p in p.iter() {
                    if p.accpects_message(&msg) {
                        eligible_producers.push(p.clone())
                    }
                }
            }
            return eligible_producers;
        }

        let producers = self.producers.read().await;
        for p in producers.iter() {
            if p.accpects_message(&msg) {
                eligible_producers.push(p.clone());
            }
        }

        eligible_producers
    }
}
