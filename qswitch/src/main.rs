use quniverse::Universe;
use std::net::{SocketAddr, ToSocketAddrs};
fn main() {
    println!("Hello world");
    let addr = "127.0.0.1:0".to_socket_addrs().unwrap().last().unwrap();
    let universe = Universe::load(addr, None);
    universe.plot_galaxy();
}
