use std::{sync::Arc, collections::VecDeque};

use log::debug;

use super::{PartitionSystem, Partition};

/// (start_addr, size, tracker)
pub trait PartitionProvider{
    /// The alignment fn takes and offset and returns if the offset is aligned
    fn partition<F:Fn(u64) -> bool>(&mut self, size: u64, alignment_fn: F) -> Result<Partition, PartitionError>;    
}

#[derive(Debug)]
pub enum PartitionError{
    NoSpace,
}

impl PartitionSystem{
    pub fn new(size: u64) -> PartitionSystem {
        let mut partitions = VecDeque::new();
        partitions.push_back(Partition{tracker:Arc::new(true),size, offset: 0 });
        PartitionSystem{ partitions }
    }
}

impl PartitionProvider for PartitionSystem{
    fn partition<F:Fn(u64) -> bool>(&mut self, size: u64, alignment_fn: F) -> Result<Partition, PartitionError> {
        // As we try accumulating enough memory will will also combine unused memory
        // We will approach this by queueing through the partitions
        let mut res = Err(PartitionError::NoSpace);
        let mut active_queue:VecDeque<Partition> = VecDeque::with_capacity(self.partitions.len());        
        
        debug!("Looking for partiton of size {}", size);
        for partition in self.partitions.iter(){
            if let Some(p) = partition.try_claim(&mut active_queue, size, &alignment_fn){
                if let Err(_) = res{
                    // We just take the first available partition
                    // Since we use this double if, we allow the rest of the partitions to be
                    // organized because even if we find another partition, the traker will be 
                    // dropped with p
                    res = Ok(p)
                }
            }
        }
        self.partitions = active_queue;
        res
    }
}
impl Partition{
    
    pub fn used(&self) -> bool{
        Arc::strong_count(&self.tracker) > 1
    }
    
    /// This function will try to produce a new partition matching alignment and size reqs
    pub fn try_claim<F:Fn(u64) -> bool>(&self, active_queue: &mut VecDeque<Partition>, size: u64, alignment_fn: &F) -> Option<Partition>{
        if self.used(){
            // If we are in use we just add ourselves to the active queue and leave
            active_queue.push_back((*self).clone());
            return None;
        }
        
        let mut best_partition = (*self).clone();
        // We need to see if we can combine ourselves with the parition before us
        if let Some(mut p) = active_queue.pop_back(){
            if !p.used(){
                // If its free we can add our own size and the set it as the best
                p.size += self.size();
                best_partition = p;
            }
            else{
                active_queue.push_back(p);
            }
        }
        
        // Now we see if the best partition has enough size to hold our aligned request
        let local_offset = 0;
        while local_offset < best_partition.size(){
            if !alignment_fn(local_offset + best_partition.offset()) && local_offset + size <= best_partition.size(){
                continue;
            }
            if best_partition.size() - local_offset < size{
                // After our offset we are not big enough
                break;
            }
            
            // If we are aligned and have enough room we can split ourselves at the local offset
            // Remember, if we had combined ourselves with another partition, we will have taken it off the queue
            if local_offset > 0{
                // If there is some unused space we make a partition for it and add it to the queue
                let un_used = Partition{
                    tracker: Arc::new(true),
                    offset: best_partition.offset(),
                    size: local_offset,
                };
                active_queue.push_back(un_used);
            }
                       
            // Here is the matching queue
            let used = Partition{
                tracker: Arc::new(true),
                offset: best_partition.offset() + local_offset,
                size,
            };
            active_queue.push_back(used.clone());
            
            if used.offset() + used.size() < best_partition.size(){
                // If we have left overs we need to make a new partition
                let un_used = Partition{
                    tracker: Arc::new(true),
                    offset: used.offset()+used.size(),
                    size: best_partition.size() - used.offset()+used.size(),
                };
                active_queue.push_back(un_used);
            }
            
            return Some(used);
        }
        
        // If no suitable queue was made we just add the best partition (self or a grown other)
        // back to the queue
        active_queue.push_back(best_partition);
        None
    }
    
    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}
