use crate::{vec3, Vec3};

#[derive(Debug, Clone, Default)]
pub struct Camera {
    pub location: Vec3,
    pub up: Vec3,
    pub front: Vec3,
    pub right: Vec3,
    yaw: f32,
    pitch: f32,
    aspect_ratio: f32,
    fov: f32,
}

pub enum Direction {
    Forward,
    Backward,
    Left,
    Right,
    Up,
    Down,
}

impl Camera {
    pub fn new(location: Vec3, look_at: Vec3, aspect_ratio: f32, fov: f32) -> Self {
        let front = look_at - location;
        let front_length = front.length();
        let pitch = (front.y / front_length)
            .asin()
            .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);
        let mut yaw = (front.z / front_length).asin();
        if front.z >= 0.0 && front.x < 0.0 {
            yaw = std::f32::consts::PI - yaw;
        }
        let mut camera = Self {
            location,
            front,
            yaw,
            pitch,
            aspect_ratio,
            fov,
            ..Default::default()
        };

        camera.update_camera_vector();

        camera
    }

    pub fn update_camera_vector(&mut self) {
        self.front.x = self.yaw.cos() * self.pitch.cos();
        self.front.y = self.pitch.sin();
        self.front.z = self.yaw.sin() * self.pitch.cos();
        self.front = self.front.try_normalize().unwrap();
        self.right = self
            .front
            .cross(vec3(0.0, 1.0, 0.0))
            .try_normalize()
            .unwrap();
        self.up = self.right.cross(self.front).try_normalize().unwrap();
    }

    pub fn process_keyboard(&mut self, direction: Direction, distance: f32) {
        match direction {
            Direction::Forward => {
                self.location += self.front * distance;
            }
            Direction::Backward => {
                self.location -= self.front * distance;
            }
            Direction::Left => {
                self.location -= self.right * distance;
            }
            Direction::Right => {
                self.location += self.right * distance;
            }
            Direction::Up => {
                self.location += self.up * distance;
            }
            Direction::Down => {
                self.location -= self.up * distance;
            }
        }
    }

    pub fn process_mouse_movement(&mut self, yaw_offset: f32, pitch_offset: f32) {
        self.yaw += yaw_offset;
        self.pitch += pitch_offset;

        self.pitch = self
            .pitch
            .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

        self.update_camera_vector();
    }
}
