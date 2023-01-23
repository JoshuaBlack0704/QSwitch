use std::sync::Arc;

#[derive(Clone, Copy)]
pub enum Channel {
    Channel(u32),
    All,
}
pub trait BusElement<Id,M>: Send + Sync{
    fn accepts_transaction(&self, src: &dyn Bus<Id,M>, transaction: &mut BusTransaction<Id,M>) -> bool;
    fn handle_transaction(&self, src: &dyn Bus<Id,M>, transaction: &mut BusTransaction<Id,M>) -> Option<M>;
}

#[derive(Clone, Debug)]
pub enum BusError{
    NoRecipients,
    NoAcceptors,
}
pub type BusResult<R> = Result<R, BusError>;
#[derive(Clone)]
pub enum BusTransaction<Id,M> {
    InProgress(M),
    /// Represents a completed broadcast transaction
    Broadcast(BroadcastTransaction<Id,M>),
    /// Represents a completed exchange transaction
    ExchangeTransaction(ExchangeTransaction<Id,M>),
}

type BE<Id,M> = Arc<dyn BusElement<Id,M>>;
#[derive(Clone)]
pub struct BroadcastTransaction<Id,M> {
    pub bus_id: Id,
    pub transaction_id: Id,
    pub queried_elements: Vec<BE<Id,M>>,
    pub accepted_elements: Vec<BE<Id,M>>,
    pub msg: M,
    pub replies: Vec<M>,
}

#[derive(Clone)]
pub struct ExchangeTransaction<Id,M>{
    pub bus_id: Id,
    pub transaction_id: Id,
    pub responder: BE<Id,M>,
    pub msg: M,
    pub reply: Option<M>,
}

pub trait Bus<Id:Clone, M: Clone>: Send + Sync {
    /// Should return all elements who might want to take this message.
    /// NOTE: During bus ops, each element will be asked if they accept this message
    /// NOTE: The order of elements might matter if any of them mutate the message during
    //  their transaction phase
    fn get_elements(&self, msg: &M) -> Vec<BE<Id,M>>;
    /// Should transform self into a trait object
    fn as_trait_object(&self) -> &dyn Bus<Id,M>;
    /// What function will be used to generate uuids for the bus
    fn get_uuid(&self) -> Id;
    /// What function will be used to generate uuids for transactions
    fn get_transaction_uuid(&self) -> Id;

    /// Will send msg to all producers that match the element query and return any responses
    fn broadcast(&self, msg: M) -> BusResult<BroadcastTransaction<Id, M>> {
        let elements = self.get_elements(&msg);
        if elements.len() == 0{
            return Err(BusError::NoRecipients);
        }
        let mut transaction = BusTransaction::<Id,M>::InProgress(msg.clone());

        let accepted: Vec<BE<Id,M>> = elements
            .iter()
            .filter(|p| p.accepts_transaction(self.as_trait_object(), &mut transaction))
            .map(|p| p.clone())
            .collect();
        let mut replies = Vec::with_capacity(elements.len());

        for e in accepted.iter() {
            if let Some(response) = e.handle_transaction(self.as_trait_object(), &mut transaction) {
                replies.push(response);
            }
        }

        let transaction = BroadcastTransaction::<Id,M> {
            bus_id: self.get_uuid(),
            transaction_id: self.get_transaction_uuid(),
            queried_elements: elements.clone(),
            accepted_elements: accepted.clone(),
            msg,
            replies: replies.clone(),
        };

        let mut complete_transaction = BusTransaction::Broadcast(transaction.clone());
        let accepts:Vec<&BE<Id,M>> = elements.iter().filter(|e|e.accepts_transaction(self.as_trait_object(), &mut complete_transaction)).collect();
        for a in accepts.iter(){
            a.handle_transaction(self.as_trait_object(), &mut complete_transaction);
        }
        
        Ok(transaction)
    }

    /// Will send the msg to the first accepting element and directly exit returning its reply
    fn exchange(&self, msg: M) -> BusResult<ExchangeTransaction<Id,M>>{
        let elements = self.get_elements(&msg);
        if elements.len() == 0{
            return Err(BusError::NoRecipients);
        }
        let mut transaction = BusTransaction::<Id,M>::InProgress(msg.clone());
        let accepts:Vec<&BE<Id,M>> = elements.iter().filter(|e| e.accepts_transaction(self.as_trait_object(), &mut transaction)).collect();

        if let Some(&e) = accepts.first(){
            let reply = e.handle_transaction(self.as_trait_object(), &mut transaction);
            let transaction = ExchangeTransaction::<Id,M>{
                bus_id: self.get_uuid(),
                transaction_id: self.get_transaction_uuid(),
                responder: e.clone(),
                msg,
                reply,
            };
            let mut complete_transaction = BusTransaction::ExchangeTransaction(transaction.clone());
            let accepts:Vec<&BE<Id,M>> = elements.iter().filter(|e|e.accepts_transaction(self.as_trait_object(), &mut complete_transaction)).collect();
            for a in accepts.iter(){
                a.handle_transaction(self.as_trait_object(), &mut complete_transaction);
            }
            return Ok(transaction);
        }

        Err(BusError::NoAcceptors)
        
    }
}

pub struct ConsoleLogger{
    
}
impl ConsoleLogger{
    pub fn new() -> Arc<ConsoleLogger> {
        Arc::new(
            Self{}
        )
    }
}

impl<Id:ToString, T:ToString> BusElement<Id,T> for ConsoleLogger{
    fn accepts_transaction(&self, _src: &dyn Bus<Id,T>, transaction: &mut BusTransaction<Id,T>) -> bool {
        match transaction{
            BusTransaction::InProgress(_) => false,
            _ => true,
        }
    }

    fn handle_transaction(&self, _src: &dyn Bus<Id,T>, transaction: &mut BusTransaction<Id,T>) -> Option<T> {
        match transaction{
            BusTransaction::InProgress(_) => panic!("Console logger must not respond to InProgress transactions"),
            BusTransaction::Broadcast(b) => {
                println!("Broadcast: {}", b.msg.to_string());
            },
            BusTransaction::ExchangeTransaction(e) => {
                println!("Exchange: {}", e.msg.to_string());
            },
        }
        None
    }
}
