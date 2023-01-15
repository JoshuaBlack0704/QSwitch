use std::sync::Arc;

use qcom::bus::{BusElement, Bus};

struct TestBed{}
impl Drop for TestBed{
    fn drop(&mut self) {
        println!("Test bed dropped");
    }
}
struct TestBed2{}
struct TestLog{
    log_bus: Arc<Bus<usize>>,
}
impl BusElement<String> for TestLog{
    fn receive_message(&self, msg: String){
        println!("Logging {:?}", msg);
        self.log_bus.broadcast(msg.len());
    }
}
impl BusElement<usize> for TestLog{
    fn receive_message(&self, msg: usize){
        println!("Writing {:?} bytes to log", msg);
    }
}
impl BusElement<String> for TestBed{}
impl BusElement<String> for TestBed2{
    fn receive_message(&self, msg: String){
        println!("Adjusted message: {:?}", msg);
    }
}
fn main(){
    let log_bus = Bus::<usize>::new();
    let t = Arc::new(TestBed{});
    let t2 = Arc::new(TestBed2{});
    let log = Arc::new(TestLog{log_bus: log_bus.clone()});
    log_bus.bind(log.clone());
    let bus = Bus::<String>::new();
    bus.bind(t);
    bus.bind(t2);
    bus.bind(log);
    bus.test();
    bus.broadcast(String::from("This works wonderfully"));
    bus.clear();
    bus.broadcast(String::from("Cleared"));

}
