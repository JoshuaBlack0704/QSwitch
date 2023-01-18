use ash::vk;
use qcom::{self, bus::{BusListener, Bus, BusTransaction}, log::LogListener};
pub const LOG_CHANNEL:u32 = u32::MAX;
#[derive(Clone)]
pub enum QvkBusMessage{
    InstanceHandle(vk::Instance),
}

impl<B:Bus<String>> BusListener<QvkBusMessage> for LogListener<B>{
    fn handle_transaction(&self, _src: &dyn Bus<QvkBusMessage>, transaction: &qcom::bus::BusTransaction<QvkBusMessage>) {
        if let BusTransaction::Broadcast(b) = transaction{
            match b.msg{
                QvkBusMessage::InstanceHandle(i) => {
                    let log = format!("Broadcasted instance {:?}", i);
                    self.log_bus.broadcast(log, qcom::bus::Channel::All);
                },
            }
        }
    }
}
