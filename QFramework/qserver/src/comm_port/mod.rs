// The comm port will be the defact way in which users will
// communicate in a cluster
// The main peices of a comm port are its channel and id
// The channel is to identify a set of ports that will talk to each other and 
// all channels will be provided by users
// The id is the unique identifier for that particular port that will be
// used for routing

use std::sync::Arc;

use crate::{CommPort, LiveState};

#[derive(Clone)]
#[repr(C)]
struct CommPortHeader{
    id: u64,
    channel: u32
}

impl CommPort{
    pub(crate) fn new(id: u64, channel: u32, live_state: Arc<LiveState>) -> Arc<CommPort> {
        Arc::new(CommPort{ id, channel, live_state })
    }
    // Since we can't store type information inside of the CommPort we will
    // need to have an api which handles type transformations before we reach
    // internally
}
