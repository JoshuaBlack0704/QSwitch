use glam::Vec3;

use noise::NoiseFn;
use qprimitives::{Primiative, ShapeVertex, Shape};

///Generates a flat terrain using the 2D noise function
///Resolution must be >= 1
///Width and depth are in units of quads
pub fn make_terrain(width: u32, depth: u32, max_height: f32, scale: f64, noise_fn: &impl NoiseFn<f64,2>, shift: Option<Vec3>, vertex_data: &mut Vec<ShapeVertex>, index_data: &mut Vec<u32>){
    //The first thing we need to do is generate the vertices
    let mut vertices = Vec::with_capacity((width * depth) as usize);
    for z in 0..depth+1{
        for x in 0..width+1{
            let x = x as f32;
            let z = z as f32;
            let n_x = (x/width as f32) as f64 * scale;
            let n_z = (z/depth as f32) as f64 * scale;
            let mut height = 1.0 + noise_fn.get([n_x,n_z]) as f32;
            height = max_height * (height/2.0).powi(5);
            
            
            let mut vertex = Vec3::new(x, height, z);
            if let Some(s) = shift{
                vertex += s;
            }
            vertices.push(vertex);
        }
    }

    //Now we must generate primatives
    let mut primatives = vec![];
    for z in 0..width{
        for x in 0..depth{
            let x = x as usize;
            let z = z as usize;
            let width = width as usize;
           
            let v1 = vertices[x +(width+1)*z];
            let v2 = vertices[x+1 + (width+1)*z];
            let v3 = vertices[x + (width+1)*(z+1)];
            let v4 = vertices[x+1 + (width+1)*(z+1)];

            let average = (v1.y + v2.y + v3.y)/3.0;
            let ratio = average/max_height;
            let color;
            if ratio < 0.02{
                color = Vec3::new(0.0,0.0,1.0);
            }
            else if ratio > 0.02 && ratio < 0.5{
                color = Vec3::new(0.0,1.0,0.0);
            }
            else{
                color = Vec3::new(1.0,1.0,1.0);
            }

            let top_primative = Primiative::from_positions(v4,v3,v1,color);
            let bottom_primative = Primiative::from_positions(v4,v1,v2,color);

            primatives.push(top_primative);
            primatives.push(bottom_primative);
        }
    }
 
    //Now we generate the shape
    Shape::from_primatives(vertex_data,index_data,&primatives,true);
}
