use std::{thread, time::Duration};

use qserver::ClusterTerminal;

fn main(){
    
    let mut terminal = ClusterTerminal::new(None, true, None);
    let mut tgt = ClusterTerminal::new(None, true, Some(terminal.get_runtime()));
    terminal.connect_to(tgt.get_addr());
    terminal.idle_async();

}