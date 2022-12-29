use crate::memory::PartitionSource;
use std::{collections::VecDeque, sync::Arc};

use super::{Partition, PartitionSystem};

#[derive(Debug)]
pub enum PartitionError {
    NoSpace,
}

impl PartitionSystem {
    pub fn new(size: u64) -> PartitionSystem {
        let mut partitions = VecDeque::new();
        partitions.push_back(Partition {
            tracker: Arc::new(true),
            size,
            offset: 0,
        });
        PartitionSystem { partitions }
    }

    fn consolidate(&mut self) {
        // We need to loop through the queue and combine touching partitions
        let mut active_queue: VecDeque<Partition> = VecDeque::with_capacity(self.partitions.len());
        for partition in self.partitions.iter() {
            // If this partition is used we will just put it on the active queue
            if partition.used() {
                active_queue.push_back((*partition).clone());
                continue;
            }

            if let Some(mut p) = active_queue.pop_back() {
                // If something is in the active queue we need to see if its used
                if p.used() {
                    // If so we just add it back along with our partition
                    active_queue.push_back(p);
                    // Since if we cloned, the partiton's arc counter would have refs to self.partition, we need to make a clean partition
                    let decoupled_partition = Partition {
                        tracker: Arc::new(true),
                        offset: partition.offset(),
                        size: partition.size(),
                    };
                    active_queue.push_back(decoupled_partition);
                    continue;
                }

                // If not we should add our partitions size to it
                p.size += partition.size();
                // Then we add it back to the queue
                active_queue.push_back(p);
                continue;
            }

            //If there is nothing in the queue we just add the parition to it
            active_queue.push_back((*partition).clone());
        }

        self.partitions = active_queue;
    }
}

impl PartitionSource for PartitionSystem {
    fn partition<F: Fn(u64) -> bool>(
        &mut self,
        size: u64,
        alignment_fn: F,
    ) -> Result<Partition, PartitionError> {
        // As we try accumulating enough memory will will also combine unused memory
        // We will approach this by queueing through the partitions
        let mut res = Err(PartitionError::NoSpace);
        self.consolidate();
        let mut new_queue = VecDeque::with_capacity(self.partitions.len());

        for partition in self.partitions.iter() {
            if let Ok(_) = res {
                new_queue.push_back(partition.clone());
                continue;
            }

            if let Some(p) = partition.try_claim(size, &alignment_fn) {
                if let Some(p) = p.0 {
                    // Under flow
                    new_queue.push_back(p);
                }

                // The new partition
                new_queue.push_back(p.1.clone());

                if let Some(p) = p.2 {
                    // Overflow
                    new_queue.push_back(p);
                }

                res = Ok(p.1);
            } else {
                new_queue.push_back((*partition).clone());
            }
        }
        self.partitions = new_queue;
        res
    }
}
impl Partition {
    pub fn used(&self) -> bool {
        Arc::strong_count(&self.tracker) > 1
    }

    /// This function will try to produce a new partition matching alignment and size reqs
    pub fn try_claim<F: Fn(u64) -> bool>(
        &self,
        size: u64,
        alignment_fn: &F,
    ) -> Option<(Option<Partition>, Partition, Option<Partition>)> {
        if self.used() {
            // If we are in use we just add ourselves to the active queue and leave
            return None;
        }

        // Now we see if the best partition has enough size to hold our aligned request
        let mut local_offset = 0;
        while local_offset < self.size() {
            if !alignment_fn(local_offset + self.offset()) {
                // We are not aligned
                local_offset += 1;
                continue;
            }
            if self.size() - local_offset < size {
                // After our offset we are not big enough
                return None;
            }

            let mut underflow = None;
            let mut overflow = None;

            // If we are aligned and have enough room we can split ourselves at the local offset
            // Remember, if we had combined ourselves with another partition, we will have taken it off the queue
            if local_offset > 0 {
                // If there is some unused space before our main partition we make a partition for it
                underflow = Some(Partition {
                    tracker: Arc::new(true),
                    offset: self.offset(),
                    size: local_offset,
                });
            }

            // Here is the main partition that will be given to the requester
            let main = Partition {
                tracker: Arc::new(true),
                offset: self.offset() + local_offset,
                size,
            };

            if local_offset + main.size() < self.size() {
                // If we have left overs we need to make a new partition
                overflow = Some(Partition {
                    tracker: Arc::new(true),
                    offset: self.offset() + local_offset + main.size(),
                    size: self.size() - (local_offset + main.size()),
                });
            }

            return Some((underflow, main, overflow));
        }

        None
    }

    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}
