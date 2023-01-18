use qcom::bus::ConsoleLogger;
use qvk::bus::QvkBus;

fn main(){
    let qvk_bus = QvkBus::new();
    qvk_bus.bind_global(ConsoleLogger::new());

    let _instance = qvk::init_bus::InstanceBuilder::default().build(&qvk_bus);
}