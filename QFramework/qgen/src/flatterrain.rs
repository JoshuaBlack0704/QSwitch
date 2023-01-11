use glam::{Vec3, Mat3};

use noise::NoiseFn;
use qprimitives::{Primiative, ShapeVertex, Shape};

///Generates a flat terrain using the 2D noise function
///Resolution must be >= 1
pub fn make_terrain(width: u32, depth: u32, max_height: f32, scale: Mat3, noise_fn: &impl NoiseFn<f64,2>, shift: Option<Vec3>, vertex_data: &mut Vec<ShapeVertex>, index_data: &mut Vec<u32>){
    let color = Vec3::new(1.0,1.0,1.0);
    //The first thing we need to do is generate the vertices
    let mut vertices = Vec::with_capacity((width * depth) as usize);
    for x in 0..width{
        for z in 0..depth{
            let x = x as f32;
            let z = z as f32;
            let mut vertex = Vec3::new(x, noise_fn.get([x as f64,z as f64]) as f32 * max_height, z);
            if let Some(s) = shift{
                vertex += s;
            }
            vertices.push(scale * vertex);
        }
    }

    //Now we must generate primatives
    let mut primatives = vec![];
    let p_x = width - 1;
    let p_z = depth - 1;
    for x in 0..p_x{
        for z in 0..p_z{
            let x = x as usize;
            let z = z as usize;
           
            let v1 = vertices[x + x*z];
            let v2 = vertices[x + x*z + 1];
            let v3 = vertices[x + x*(z+1)];
            let v4 = vertices[x + x*(z+1) + 1];

            let top_primative = Primiative::new(v4,v3,v1,color);
            let bottom_primative = Primiative::new(v4,v1,v2,color);

            primatives.push(top_primative);
            primatives.push(bottom_primative);
        }
    }
 
    //Now we generate the shape
    Shape::from_primatives(vertex_data,index_data,&primatives,true);
}
