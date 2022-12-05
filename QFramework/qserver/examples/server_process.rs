use std::net::{SocketAddr, ToSocketAddrs};
use qserver::LocalServer;

use clap::Parser;

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
    let server = LocalServer::new(Some(addr), arg.discoverable, None);
    if let Ok(addr) = arg.tgt.to_socket_addrs(){
        if let Some(addr) = addr.last(){
            LocalServer::connect_to_server(server.clone(), addr);
        }
    }
    server.idle_async();
    
}