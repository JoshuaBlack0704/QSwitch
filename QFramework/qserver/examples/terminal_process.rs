use std::net::{SocketAddr, ToSocketAddrs};

use clap::Parser;
use qserver::ClusterTerminal;

#[derive(Parser)]
struct Args {
    
    #[arg(short, default_value_t = String::new())]
    tgt: String,
    
    #[arg(short, long)]
    discoverable: bool
    
}

fn main(){
    let arg = Args::parse();
    let ip = local_ip_address::local_ip().unwrap();
    let port = 0;
    let addr = SocketAddr::new(ip, port);
    let terminal = ClusterTerminal::new(Some(addr), arg.discoverable, None);
    if let Ok(addr) = arg.tgt.to_socket_addrs(){
        if let Some(addr) = addr.last(){
            terminal.connect_to(addr);
        }
    }
    terminal.idle_async();
    
}