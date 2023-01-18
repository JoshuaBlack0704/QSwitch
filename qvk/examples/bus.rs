use qcom::{bus::BusSystem, log::{ConsoleLogger, LogListener}};
use qvk::bus::{QvkBusMessage, LOG_CHANNEL};

fn main(){
    let log_bus = BusSystem::<String>::new();
    let qvk_bus = BusSystem::<QvkBusMessage>::new();
    ConsoleLogger::new(&log_bus, 0);
    qvk_bus.bind_listener(LogListener::new(&log_bus), LOG_CHANNEL);

    let instance = qvk::init_bus::InstanceBuilder::default().build(&qvk_bus);
    

    
    
}