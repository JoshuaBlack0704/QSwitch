use clap::Parser;
use qserver::ClusterTerminal;
use std::net::{SocketAddr, ToSocketAddrs, IpAddr, Ipv4Addr};
use local_ip_address::local_ip;

#[derive(Parser, Debug)]
struct Arg {
    //Target cluster address
    #[arg(short)]
    target: String,
}
fn main() {
    let args = Arg::try_parse();
    let ip = local_ip().unwrap();
    let port = 0;
    let addr = SocketAddr::new(ip, port);
    let t1 = ClusterTerminal::new(addr);
    if let Ok(arg) = args{
        t1.join_cluster(arg.target.to_socket_addrs().unwrap().last().unwrap());
    }
    loop {}
}
