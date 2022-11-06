use clap::Parser;
use qserver::ClusterTerminal;
use std::net::{SocketAddr, ToSocketAddrs};
use local_ip_address::local_ip;

#[derive(Parser, Debug)]
struct Arg {
    //Target cluster address
    #[arg(short, default_value_t = String::new())]
    target: String,
    #[arg(short, long)]
    private: bool,
}
fn main() {
    let arg = Arg::parse();
    let ip = local_ip().unwrap();
    let port = 0;
    let addr = SocketAddr::new(ip, port);
    let t1 = ClusterTerminal::new(addr, !arg.private);
    if let Ok(addr) = arg.target.to_socket_addrs(){
        if let Some(addr) = addr.last(){
            t1.join_cluster(addr);
        }
    }
    loop {}
}
