use std::sync::Arc;
use std::string::String;

use crate::bus::{Bus, BusListener, BusProducer, BusTransaction};
pub trait Loggable{
    fn log(&self) -> String;
}
impl Loggable for u32{
    fn log(&self) -> String {
        self.to_string()
    }
}

/// Is plugged into other buses
pub struct LogListener<B:Bus<String>>{
    pub log_bus: B,
}

impl<B:Bus<String> + Clone> LogListener<B>{
    pub fn new(log_bus: &B) -> Arc<LogListener<B>> {
        Arc::new(
            Self{
                log_bus: log_bus.clone(),
            }
        )
    }
}

impl<B:Bus<String>> BusListener<u32> for LogListener<B>{
    fn handle_transaction(&self, _src: &dyn Bus<u32>, transaction: &BusTransaction<u32>) {
        match transaction{
            BusTransaction::Broadcast(t) => {self.log_bus.broadcast(t.msg.log(), crate::bus::Channel::All);},
            BusTransaction::Intercept(_) => todo!(),
        }
    }
}

/// Prints all log strings to console
pub struct ConsoleLogger{
    
}

impl ConsoleLogger {
    pub fn new<B:Bus<String>>(log_bus: &B, channel: u32){
        let cl = Arc::new(Self{});
        log_bus.bind_producer(cl, channel);
    }
}

impl BusProducer<String> for ConsoleLogger{
    fn accepts_message(&self, _msg: &String) -> bool {
        true
    }

    fn handle_message(&self, _src: &dyn Bus<String>, msg: &String) -> Option<String> {
        println!("{msg}");
        None
    }
}