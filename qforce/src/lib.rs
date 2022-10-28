use rand::Rng;

pub struct Engine {}
#[derive(Clone)]
pub struct Vector {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
impl Vector {
    pub fn random_cube<R: Rng>(bounds: f32, rng: &mut R) -> Vector {
        let x = rng.gen_range(0.0..=bounds);
        let y = rng.gen_range(0.0..=bounds);
        let z = rng.gen_range(0.0..=bounds);
        Vector { x, y, z }
    }
}
