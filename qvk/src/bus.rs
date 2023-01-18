use std::sync::Arc;
use tokio::sync::RwLock;
use rand::{self, thread_rng, Rng};

use ash::vk;
use qcom::bus::{BusElement, Bus};

use crate::init_bus::InstanceSource;
#[derive(Clone)]
pub enum QvkBusMessage{
    InstanceHandle(vk::Instance),
    GetInstance,
    Instance(Arc<dyn InstanceSource>)
}

pub type QvkElement = Arc<dyn BusElement<QvkBusMessage>>;
pub struct QvkBus{
    globals: RwLock<Vec<QvkElement>>,
    instance: RwLock<Option<QvkElement>>,
}

impl QvkBus{
    pub fn new() -> Arc<QvkBus> {
        Arc::new(
            Self{
                globals: RwLock::new(vec![]),
                instance: RwLock::new(None),
            }
        )
    }
    pub fn bind_global(&self, global: QvkElement){
        let mut globals = self.globals.blocking_write();
        globals.push(global);
    }
    pub fn bind_instance(&self, instance: QvkElement){
        let mut i = self.instance.blocking_write();
        if let Some(_) = *i{
            panic!("Cannot add bind instance to QvkBus twice");
        }

        *i = Some(instance);
    }
}

impl Bus<QvkBusMessage> for Arc<QvkBus>{
    fn as_trait_object(&self) -> &dyn Bus<QvkBusMessage> {
        self
    }

    fn get_elements(&self, msg: &QvkBusMessage) -> Vec<Arc<dyn BusElement<QvkBusMessage>>> {
        match msg{
            QvkBusMessage::InstanceHandle(_) => {
                return self.globals.blocking_read().clone();
            },
            QvkBusMessage::GetInstance => {
                let instance = self.instance.blocking_read();
                match &(*instance){
                    Some(i) => vec![i.clone()],
                    None => vec![],
                }
            },
            _ => vec![]
        }
    }

    fn get_transaction_uuid(&self) -> fn() -> u64 {
        || {thread_rng().gen::<u64>()}
    }

    fn get_uuid(&self) -> fn() -> u64 {
        || {thread_rng().gen::<u64>()}
    }
}

impl ToString for QvkBusMessage{
    fn to_string(&self) -> String {
        match self{
            QvkBusMessage::InstanceHandle(i) => format!("Created instance {:?}", i),
            QvkBusMessage::GetInstance => format!("Crequesting instance"),
            QvkBusMessage::Instance(i) => format!("Replied with instance {:?}", i.get_instance().handle()),
        }
    }
}
