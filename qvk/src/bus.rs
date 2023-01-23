use std::sync::Arc;
use tokio::sync::RwLock;
use rand::{self, thread_rng, Rng};

use ash::vk;
use qcom::bus::{BusElement, Bus};

use crate::init_bus::InstanceSource;
#[derive(Clone)]
pub enum QvkBusMessage{
    GetInstance,
    Instance(Arc<dyn InstanceSource>)
}
#[derive(Clone)]
pub enum QvkBusId{
    Bus(String),
    Transaction(u64),
}

pub type QvkElement = Arc<dyn BusElement<QvkBusId,QvkBusMessage>>;
pub struct QvkBus{
    uuid: u64,
    qvk_elements: RwLock<Vec<QvkElement>>,
}

impl QvkBus{
    pub fn new() -> Arc<QvkBus> {
        Arc::new(
            Self{
                
                qvk_elements: RwLock::new(vec![]),
                uuid: thread_rng().gen::<u64>(),
            }
        )
    }

    pub fn bind_element(&self, qvk_element: QvkElement){
        let mut elements = self.qvk_elements.blocking_write();
        elements.push(qvk_element);
    }

    pub fn get_instance(self: Arc<Self>) -> Arc<dyn InstanceSource>{
        match self.exchange(QvkBusMessage::GetInstance)
            .expect("No instance source bound to the qvk bus")
            .reply.expect("Bound qvk instance did not return anything"){
            QvkBusMessage::Instance(i) => i,
            _ => panic!("Bound qvk instance returned wrong message type")
        }
    }
}


impl Bus<QvkBusId,QvkBusMessage> for Arc<QvkBus>{
    fn as_trait_object(&self) -> &dyn Bus<QvkBusId,QvkBusMessage> {
        self
    }

    fn get_elements(&self, _msg: &QvkBusMessage) -> Vec<Arc<dyn BusElement<QvkBusId,QvkBusMessage>>> {
        self.qvk_elements.blocking_read().clone()
    }

    fn get_transaction_uuid(&self) -> QvkBusId {
        QvkBusId::Transaction(thread_rng().gen::<u64>())
    }

    fn get_uuid(&self) -> QvkBusId {
        QvkBusId::Bus(format!("Main qvk bus: {}", self.uuid))
    }
}

impl ToString for QvkBusMessage{
    fn to_string(&self) -> String {
        match self{
            QvkBusMessage::GetInstance => format!("Requesting instance"),
            QvkBusMessage::Instance(i) => format!("Replied with instance {:?}", i.get_instance().handle()),
        }
    }
}

impl ToString for QvkBusId{
    fn to_string(&self) -> String {
        match self{
            QvkBusId::Bus(s) => s.clone(),
            QvkBusId::Transaction(uuid) => uuid.to_string(),
        }
    }
}
