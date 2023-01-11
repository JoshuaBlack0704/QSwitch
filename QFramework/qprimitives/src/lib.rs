use std::mem::size_of;

use qvk::pipelines::graphics::VertexStateFactory;
use ash::vk;
use glam::{Mat3, Vec3};
pub struct Shape {
    _starting_index: u32,
    _starting_vertex: u32,
}
impl Shape {
    pub fn new(
        vertex_data: &mut Vec<ShapeVertex>,
        index_data: &mut Vec<u32>,
        vertices: &[ShapeVertex],
        indices: &[u32],
    ) -> Shape {
        let starting_index = index_data.len() as u32;
        let starting_vertex = vertex_data.len() as u32;
        vertex_data.extend_from_slice(vertices);
        index_data.extend_from_slice(indices);
        Shape {
            _starting_index: starting_index,
            _starting_vertex: starting_vertex,
        }
    }

    pub fn from_primatives(vertex_data: &mut Vec<ShapeVertex>, index_data: &mut Vec<u32>, primatives: &[Primiative], smooth: bool){
        let mut vertices:Vec<ShapeVertex> = Vec::with_capacity(primatives.len() * 3);
        let mut indices:Vec<u32> = vec![];
        for p in primatives.iter(){
            for v in p.vertices().iter(){
                if smooth{
                    if let Some(i) = vertices.iter().position(|vp| v.pos == vp.pos){
                        indices.push(i as u32);
                        continue;
                    }
                }

                indices.push(vertices.len() as u32);
                vertices.push(v.clone());
            }
        }

        vertex_data.extend_from_slice(&vertices);
        index_data.extend_from_slice(&indices);
    }

    pub fn tetrahedron(vertex_data: &mut Vec<ShapeVertex>, index_data: &mut Vec<u32>) -> Shape {
        let rotation = Mat3::from_rotation_x(3.14 / 2.0);
        let _offset = Vec3::new(0.0, 0.0, 0.0);
        let f = rotation * Vec3::new(0.0, -0.5, -0.5) + _offset;
        let br = rotation * Vec3::new(0.5, -0.5, 0.5) + _offset;
        let bl = rotation * Vec3::new(-0.5, -0.5, 0.5) + _offset;
        let t = rotation * Vec3::new(0.0, 1.0, 0.0) + _offset;
        let color = Vec3::new(1.0, 1.0, 1.0);

        let p1 = Primiative::new(f, br, t, color).vertices();
        let p2 = Primiative::new(bl, t, br, color).vertices();
        let p3 = Primiative::new(bl, f, t, color).vertices();
        let p4 = Primiative::new(f, bl, br, color).vertices();

        let mut vertices = vec![];
        vertices.extend_from_slice(&p1);
        vertices.extend_from_slice(&p2);
        vertices.extend_from_slice(&p3);
        vertices.extend_from_slice(&p4);
        let indices: Vec<u32> = (0..vertices.len()).map(|i| i as u32).collect();

        Self::new(vertex_data, index_data, &vertices, &indices)
    }
}
pub struct Primiative {
    v1: Vec3,
    v2: Vec3,
    v3: Vec3,
    normal: Vec3,
    color: Vec3,
}
#[repr(C)]
#[derive(Clone)]
pub struct ShapeVertex {
    pos: Vec3,
    normal: Vec3,
    color: Vec3,
}

impl Primiative {
    pub fn new(v1: Vec3, v2: Vec3, v3: Vec3, color: Vec3) -> Primiative {
        let t1 = v2 - v1;
        let t2 = v3 - v1;
        let normal = t2.cross(t1).normalize();
        Primiative {
            v1,
            v2,
            v3,
            normal,
            color,
        }
    }
    pub fn vertices(&self) -> [ShapeVertex; 3] {
        let v1 = ShapeVertex {
            pos: self.v1,
            normal: self.normal,
            color: self.color,
        };
        let v2 = ShapeVertex {
            pos: self.v2,
            normal: self.normal,
            color: self.color,
        };
        let v3 = ShapeVertex {
            pos: self.v3,
            normal: self.normal,
            color: self.color,
        };
        [v1, v2, v3]
    }
}

impl VertexStateFactory for ShapeVertex {
    fn flags(&self) -> Option<ash::vk::PipelineVertexInputStateCreateFlags> {
        None
    }

    fn bindings(&self) -> Vec<ash::vk::VertexInputBindingDescription> {
        let b1 = vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build();
        vec![b1]
    }

    fn attributes(&self) -> Vec<ash::vk::VertexInputAttributeDescription> {
        let att1 = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec3>() as u32 * 0)
            .build();
        let att2 = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec3>() as u32 * 1)
            .build();
        let att3 = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec3>() as u32 * 2)
            .build();

        vec![att1, att2, att3]
    }
}

impl Default for ShapeVertex {
    fn default() -> Self {
        Self {
            pos: Vec3::new(0.0, 0.0, 0.0),
            normal: Vec3::new(0.0, 0.0, 0.0),
            color: Vec3::new(0.0, 0.0, 0.0),
        }
    }
}
