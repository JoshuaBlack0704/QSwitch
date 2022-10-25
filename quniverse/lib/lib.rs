use std::net::SocketAddr;

use glam;
use qforce::{self, Engine};
use qserver::{self, UdpServiceListener};
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;
use rayon::{self, prelude::*, ThreadPool, ThreadPoolBuilder};
use tokio::{self, runtime::Runtime};

pub struct Universe {
    rt: Runtime,
    engine: Engine,
    threadpool: ThreadPool,
    udp_server: UdpServiceListener,
}
impl Universe {
    pub fn load(host_addr: SocketAddr, save_file: Option<String>) -> Universe {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Could not start tokio runtime");
        let engine = Engine {};
        let threadpool = ThreadPoolBuilder::new()
            .build()
            .expect("Could not start rayon threadpool");
        let udp_server = UdpServiceListener::start(host_addr, &rt);
        match save_file {
            Some(n) => todo!(),
            None => Self::generate_universe(1000000),
        }
        Universe {
            rt,
            engine,
            threadpool,
            udp_server,
        }
    }
    fn generate_universe(system_count: usize) {
        let mut rng1 = Xoshiro256Plus::seed_from_u64(2);
        let mut systems = vec![data::star_systems::System::new(&mut rng1); system_count];
        println!("Starting system gen");
        for system in systems.iter_mut(){
            *system = data::star_systems::System::new(&mut rng1);
            // println!("Generated new system: {:?}", *system);
        }
    }
}
mod data {
    pub mod star_systems{
        use glam::Vec3;
        use rand::Rng;

        #[derive(Clone, Debug)]
        pub struct System {
            pos: Vec3,
            planets: Vec<Planet>,
        }
        impl System {
            pub fn new<R: Rng>(rng: &mut R) -> System {
                let planet_count = rng.gen_range(0..=10);
                let x = rng.gen_range(0.0..=10000.0);
                let y = rng.gen_range(0.0..=10000.0);
                let z = rng.gen_range(0.0..=10000.0);
                let pos = Vec3::new(x,y,z);
                let mut planets = vec![Planet::new(rng);planet_count];
                for planet in planets.iter_mut(){
                    *planet = Planet::new(rng);
                }
                System{ pos, planets }
            }
        }
        #[derive(Clone, Debug)]
        pub struct Planet{
            pos: Vec3,
            radius: f32,
            atmosphere_pressure: f32,
            moon_count: u8,
        }
        impl Planet{
            fn new<R:Rng>(rng: &mut R) -> Planet {
                let x = rng.gen_range(0.0..=10000.0);
                let y = rng.gen_range(0.0..=10000.0);
                let z = rng.gen_range(0.0..=10000.0);
                let pos = Vec3::new(x,y,z);
                let radius = rng.gen_range(1000.0..=10000.0);
                let atmosphere_pressure = rng.gen_range(0.0..=100.0);
                let moon_count = rng.gen_range(0..5);
                Planet{ pos, radius, atmosphere_pressure, moon_count }
            }
        }
    }
}
