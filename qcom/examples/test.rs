use std::sync::Arc;

use qcom::bus::{BusProducer, BusSystem};

struct TestBed{}
impl Drop for TestBed{
    fn drop(&mut self) {
        println!("Test bed dropped");
    }
}
struct TestBed2{}
struct TestLog{
    log_bus: Arc<BusSystem<usize>>,
}
fn main(){
    let log_bus = BusSystem::<usize>::new();
    let t = Arc::new(TestBed{});
    let t2 = Arc::new(TestBed2{});
    let log = Arc::new(TestLog{log_bus: log_bus.clone()});
    log_bus.bind(log.clone());
    let bus = BusSystem::<String>::new();
    bus.bind(t);
    bus.bind(t2);
    bus.bind(log);
    bus.test();
    bus.broadcast(String::from("This works wonderfully"));
    bus.clear();
    bus.broadcast(String::from("Cleared"));

}
