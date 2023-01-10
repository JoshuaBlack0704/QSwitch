use glam::{Mat4, Vec3};
use winit::{
    self,
    event::{KeyboardInput, VirtualKeyCode},
};

pub struct Camera {
    pos: Vec3,
    dir: Vec3,
    speed: f32,
    angle_axis: Vec3,
    angles: Vec3,
    angle_rate: f32,
}

impl Camera {
    pub fn new(start: Vec3, speed: f32, angle_rate: f32) -> Camera {
        Camera {
            pos: start,
            dir: Vec3::new(0.0, 0.0, 0.0),
            speed,
            angle_axis: Vec3::new(0.0, 0.0, 0.0),
            angles: Vec3::default(),
            angle_rate,
        }
    }
    pub fn process_input(&mut self, input: KeyboardInput) {
        if let None = input.virtual_keycode {
            return;
        }
        match input.state {
            winit::event::ElementState::Pressed => {
                if let VirtualKeyCode::W = input.virtual_keycode.unwrap() {
                    self.dir.z = 1.0;
                }
                if let VirtualKeyCode::S = input.virtual_keycode.unwrap() {
                    self.dir.z = -1.0;
                }
                if let VirtualKeyCode::A = input.virtual_keycode.unwrap() {
                    self.dir.x = -1.0;
                }
                if let VirtualKeyCode::D = input.virtual_keycode.unwrap() {
                    self.dir.x = 1.0;
                }
                if let VirtualKeyCode::Q = input.virtual_keycode.unwrap() {
                    self.angle_axis.y = 1.0;
                }
                if let VirtualKeyCode::E = input.virtual_keycode.unwrap() {
                    self.angle_axis.y = -1.0;
                }
                if let VirtualKeyCode::R = input.virtual_keycode.unwrap() {
                    self.dir.y = 1.0;
                }
                if let VirtualKeyCode::F = input.virtual_keycode.unwrap() {
                    self.dir.y = -1.0;
                }
                if let VirtualKeyCode::T = input.virtual_keycode.unwrap() {
                    self.angle_axis.x = 1.0;
                }
                if let VirtualKeyCode::G = input.virtual_keycode.unwrap() {
                    self.angle_axis.x = -1.0;
                }
            }
            winit::event::ElementState::Released => {
                if let VirtualKeyCode::W = input.virtual_keycode.unwrap() {
                    self.dir.z = 0.0;
                }
                if let VirtualKeyCode::S = input.virtual_keycode.unwrap() {
                    self.dir.z = 0.0;
                }
                if let VirtualKeyCode::A = input.virtual_keycode.unwrap() {
                    self.dir.x = 0.0;
                }
                if let VirtualKeyCode::D = input.virtual_keycode.unwrap() {
                    self.dir.x = 0.0;
                }
                if let VirtualKeyCode::Q = input.virtual_keycode.unwrap() {
                    self.angle_axis.y = 0.0;
                }
                if let VirtualKeyCode::E = input.virtual_keycode.unwrap() {
                    self.angle_axis.y = 0.0;
                }
                if let VirtualKeyCode::R = input.virtual_keycode.unwrap() {
                    self.dir.y = 0.0;
                }
                if let VirtualKeyCode::F = input.virtual_keycode.unwrap() {
                    self.dir.y = 0.0;
                }
                if let VirtualKeyCode::T = input.virtual_keycode.unwrap() {
                    self.angle_axis.x = 0.0;
                }
                if let VirtualKeyCode::G = input.virtual_keycode.unwrap() {
                    self.angle_axis.x = 0.0;
                }
            }
        }
    }
    pub fn perspective(&self, fov: f32, aspect: f32) -> Mat4 {
        let n = 0.1;
        let f = 1000.0;
        let x = glam::Vec4::new(1.0 / ((aspect) * (fov / 2.0).tan()), 0.0, 0.0, 0.0);
        let y = glam::Vec4::new(0.0, 1.0 / (fov / 2.0).tan(), 0.0, 0.0);
        let z = glam::Vec4::new(0.0, 0.0, f / (f - n), 1.0);
        let w = glam::Vec4::new(0.0, 0.0, -(f * n) / (f - n), 0.0);
        glam::Mat4::from_cols(x, y, z, w)
    }
    pub fn view(&mut self, delta_time: f32) -> Mat4 {
        let angle = self.angle_rate * delta_time;
        self.angles += self.angle_axis * angle;
        let y_rot = glam::Quat::from_rotation_y(self.angles.y);
        let x_rot = glam::Quat::from_rotation_x(self.angles.x) * y_rot;
        let rot = (x_rot * y_rot).normalize();
        let rot = Mat4::from_quat(rot);

        if self.dir.length() != 0.0 {
            println!("Translating at {:?}", self.pos);
            let velocity = rot.transpose().transform_vector3(self.dir.normalize()) * self.speed;
            let displacement = velocity * delta_time;
            self.pos += displacement;
        }

        let x = glam::Vec4::new(1.0, 0.0, 0.0, 0.0);
        let y = glam::Vec4::new(0.0, 1.0, 0.0, 0.0);
        let z = glam::Vec4::new(0.0, 0.0, 1.0, 0.0);
        let w = glam::Vec4::new(-self.pos.x, -self.pos.y, -self.pos.z, 1.0);
        let translation = Mat4::from_cols(x, y, z, w);

        rot * translation
        // translation
    }
}
