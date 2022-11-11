use std::net::ToSocketAddrs;

use quniverse::Universe;

fn main() {
    let addr = "127.0.0.1:0".to_socket_addrs().unwrap().last().unwrap();
    let universe = Universe::load(addr, None);
}
