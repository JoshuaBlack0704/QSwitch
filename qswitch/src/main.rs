use qforce::engine;
use quniverse::Universe;
use std::net::{SocketAddr, ToSocketAddrs};
fn main() {
    let engine = engine::new_windowed().1;
    engine.hello_window();
}
