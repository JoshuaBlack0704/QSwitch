use plotters::{prelude::*, style::colors};
use std::net::SocketAddr;

use glam;
use qforce::{
    self,
    data::Vector,
    engine::{self, Engine},
};
use qserver::{self, UdpServiceListener};
use qvk::{self, init::Initializer};
use rand::{Rng, SeedableRng};
use rand_xoshiro::Xoshiro256Plus;
use rayon::{self, prelude::*, vec, ThreadPool, ThreadPoolBuilder};
use tokio::{self, runtime::Runtime};

#[derive(Clone)]
pub struct ProceduralGenerationSettings {
    galaxy_gen: ProceduralGalaxyGenSettings,
}
impl Default for ProceduralGenerationSettings {
    fn default() -> Self {
        let galaxy_gen = ProceduralGalaxyGenSettings::default();
        ProceduralGenerationSettings { galaxy_gen }
    }
}

pub struct Universe {
    rt: Runtime,
    engine: Engine<Initializer>,
    threadpool: ThreadPool,
    udp_server: UdpServiceListener,
    rng: Xoshiro256Plus,
    galaxy: Galaxy,
}
impl Universe {
    pub fn load(host_addr: SocketAddr, save_file: Option<String>) -> Universe {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Could not start tokio runtime");
        let engine = engine::new();
        let threadpool = ThreadPoolBuilder::new()
            .build()
            .expect("Could not start rayon threadpool");
        let udp_server = UdpServiceListener::start(host_addr, &rt);
        let mut rng = Xoshiro256Plus::seed_from_u64(1);
        let galaxy = match save_file {
            Some(n) => todo!(),
            None => Self::generate_universe(1000000, &mut rng),
        };
        Universe {
            rt,
            engine,
            threadpool,
            udp_server,
            rng,
            galaxy,
        }
    }
    fn generate_universe<R: Rng>(system_count: usize, rng: &mut R) -> Galaxy {
        let settings = ProceduralGenerationSettings::default();
        Galaxy::generate(&settings.galaxy_gen, rng)
    }
    pub fn plot_galaxy(&self) {
        self.galaxy.plot_galaxy();
    }
}

//GalaxyStructure
#[derive(Clone)]
pub struct ProceduralGalaxyGenSettings {
    galaxy_max_size: f32,
    system_count: usize,
    max_system_size: f32,
    planetary_system_max_size: f32,
    planetary_system_max_cb_count: usize,
    co_max_size: f32,
}
//These not only systems, but also celstial bodies
#[derive(Clone)]
struct PlanetarySystem {
    galaxy_pos: Vector,
    spatial_bound: f32,
    star_size: f32,
    star_temp: f32,
    c_bodies: Vec<CelestialBody>,
}
//These might be planets, moon, dwarf planets, comets. The only requirement is that we assume they are static. That is, they do not dynamically spawn and despawn
//like an asteroid would. They are also big enough to render at far distances meaning upon
#[derive(Clone)]
struct CelestialBody {
    system_pos: Vector,
    size: f32,
    //0-1 where 0 is none and 1 is Venus like. 0.5 is Earth like
    atmosphere_quality: f32,
    //0-1 where 0 is none and 1 is completely covered
    water_content: f32,
}
//The main struct representing and automating the static data of the galaxy
//Will handle galaxy loading/saving, procedual generation, spatial data structure generation, gpu memory placement, and descriptor generation
//of static galaxy structure data
#[derive(Clone)]
struct Galaxy {
    bound: f32,
    plantary_systems: Vec<PlanetarySystem>,
}

//Galaxy Structure impl block
impl Galaxy {
    pub fn generate<R: Rng>(settings: &ProceduralGalaxyGenSettings, rng: &mut R) -> Galaxy {
        let mut psystems = vec![PlanetarySystem::generate(settings, rng); settings.system_count];
        for system in psystems.iter_mut() {
            *system = PlanetarySystem::generate(settings, rng);
        }
        Galaxy {
            bound: settings.galaxy_max_size,
            plantary_systems: psystems,
        }
    }
    pub fn plot_galaxy(&self) {
        let area = BitMapBackend::gif("plot.gif", (1000, 1000), 10)
            .unwrap()
            .into_drawing_area();
        area.fill(&BLACK);
        let x_axis = (0.0..self.bound).step(0.1);
        let y_axis = (0.0..self.bound).step(0.1);
        let z_axis = (0.0..self.bound).step(0.1);

        let mut chart = ChartBuilder::on(&area)
            .caption(format!("Galaxy"), ("sans", 29))
            .build_cartesian_3d(x_axis.clone(), y_axis.clone(), z_axis.clone())
            .unwrap();
        chart
            .configure_axes()
            .light_grid_style(WHITE)
            .max_light_lines(3)
            .draw()
            .unwrap();

        let data = self.plantary_systems.iter().map(|ps| {
            let coord = (ps.galaxy_pos.x, ps.galaxy_pos.y, ps.galaxy_pos.z);
            let fraction = ps.galaxy_pos.x / self.bound;
            let color = HSLColor(fraction as f64, 1.0, 0.5);
            TriangleMarker::new(coord, 2, &color)
        });
        for pitch in 0..1570 {
            area.fill(&BLACK).unwrap();
            let mut chart = ChartBuilder::on(&area)
                .caption(format!("Galaxy"), ("sans", 29))
                .build_cartesian_3d(x_axis.clone(), y_axis.clone(), z_axis.clone())
                .unwrap();
            chart.with_projection(|mut p| {
                p.pitch = 1.57 - (1.57 - pitch as f64 / 50.0).abs();
                p.scale = 0.7;
                p.into_matrix()
            });
            // chart
            //     .configure_axes()
            //     .light_grid_style(WHITE)
            //     .max_light_lines(3)
            //     .draw()
            //     .unwrap();
            chart.draw_series(data.clone()).unwrap();
            area.present().unwrap();
            println!("Frame {} of {} completed", pitch + 1, 1570);
        }
    }
}
impl PlanetarySystem {
    fn generate<R: Rng>(settings: &ProceduralGalaxyGenSettings, rng: &mut R) -> PlanetarySystem {
        let spatial_bound = rng.gen_range(100.0..=1000.0);
        let mut c_bodies = vec![
            CelestialBody::generate(settings, spatial_bound, rng);
            rng.gen_range(1..=settings.planetary_system_max_cb_count)
        ];
        for co in c_bodies.iter_mut() {
            *co = CelestialBody::generate(settings, spatial_bound, rng);
        }
        PlanetarySystem {
            galaxy_pos: Vector::random_cube(settings.galaxy_max_size, rng),
            spatial_bound,
            star_size: rng.gen_range(1.0..=10.0),
            star_temp: rng.gen_range(1000.0..=10000.0),
            c_bodies,
        }
    }
}
impl CelestialBody {
    fn generate<R: Rng>(
        settings: &ProceduralGalaxyGenSettings,
        spatial_bound: f32,
        rng: &mut R,
    ) -> CelestialBody {
        CelestialBody {
            system_pos: Vector::random_cube(spatial_bound, rng),
            size: rng.gen_range(10.0..=100.0),
            atmosphere_quality: rng.gen_range(0.0..1.0),
            water_content: rng.gen_range(0.0..1.0),
        }
    }
}
impl Default for ProceduralGalaxyGenSettings {
    fn default() -> Self {
        ProceduralGalaxyGenSettings {
            galaxy_max_size: 1000.0,
            system_count: 10000,
            max_system_size: 1000.0,
            planetary_system_max_size: 1000.0,
            planetary_system_max_cb_count: 10,
            co_max_size: 100.0,
        }
    }
}
