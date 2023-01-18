use qcom::{log::{ConsoleLogger, ConsoleListener}, bus::{BusSystem, Bus}};

fn main(){
    let log_bus = BusSystem::<String>::new();
    let data_bus = BusSystem::<u32>::new();

    let _cl = ConsoleLogger::new(&log_bus, 0);
    let _cl = ConsoleLogger::new(&log_bus, 0);
    let _cl = ConsoleLogger::new(&log_bus, 0);
    let _cl = ConsoleLogger::new(&log_bus, 0);
    let _cl = ConsoleLogger::new(&log_bus, 0);
    let _cl = ConsoleLogger::new(&log_bus, 0);
    let plug = ConsoleListener::new(&log_bus);

    data_bus.bind_listener(plug.clone(), 0);
    let _ = data_bus.broadcast(100, qcom::bus::Channel::Channel(0));
    
}
