use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use super::TerminalMap;

impl TerminalMap{
    
    ///Creates a new terminal map
    pub fn new(discoverable: bool) -> Arc<TerminalMap> {
        Arc::new(TerminalMap{ active_connections: RwLock::new(HashMap::new()), discoverable })
    }
}