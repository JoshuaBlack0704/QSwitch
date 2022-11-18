
use qserver::ClusterTerminal;

fn main(){
    
    let terminal = ClusterTerminal::new(None, true, None);
    let tgt = ClusterTerminal::new(None, true, Some(terminal.get_runtime()));
    terminal.connect_to(tgt.get_addr());
    terminal.idle_async();

}