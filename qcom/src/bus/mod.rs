use std::{sync::{Arc, Mutex}, fmt::Debug};

pub trait BusElement<M: Debug>{
    fn receive_message(&self, msg: M){
        println!("{:?}", msg);
    }
}

pub struct Bus<M>{
    elements: Mutex<Vec<Arc<dyn BusElement<M>>>>
}

impl<M:Debug + Clone> Bus<M>{
    pub fn new() -> Arc<Bus<M>>  {
        Arc::new(
            Self{
                elements: Mutex::new(vec![]),
            }
        )
    }
    pub fn bind(&self, element: Arc<dyn BusElement<M>>){
        let mut elements = self.elements.lock().unwrap();
        elements.push(element);
    }
    pub fn broadcast(&self, msg: M){
        let elements = self.elements.lock().unwrap();
        for e in elements.iter(){
            e.receive_message(msg.clone());
        }
    }
    pub fn clear(&self){
        let mut elements = self.elements.lock().unwrap();
        elements.clear();
    }
}

impl Bus<String>{
    pub fn test(&self){
        let elements = self.elements.lock().unwrap();
        for e in elements.iter(){
            e.receive_message(String::from("TEST"));
        }
    }
}
