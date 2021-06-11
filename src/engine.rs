pub use crate::camera::{Camera, Direction};
use crate::{vec3, Vec3};

pub struct Engine {
    device: maligog::Device,
    start_instant: std::time::Instant,
    last_frame_instant: std::time::Instant,
    frame_instant: std::time::Instant,
    frame_time: f64,
    camera: Camera,
}

impl Engine {
    pub fn new(window: &winit::window::Window) -> Self {
        let entry = maligog::Entry::new().unwrap();
        let required_extensions = maligog::Surface::required_extensions();
        let instance = entry.create_instance(&[], &&required_extensions);
        let device = instance
            .enumerate_physical_device()
            .into_iter()
            .find(|p| p.device_type() == maligog::PhysicalDeviceType::DISCRETE_GPU)
            .unwrap()
            .create_device();
        let start_instant = std::time::Instant::now();
        let frame_instant = start_instant;
        let last_frame_instant = start_instant;
        let frame_time = 0.0;
        let width = window.inner_size().width;
        let height = window.inner_size().height;

        let camera = Camera::new(
            vec3(0.0, 1.0, -1.0),
            vec3(0.0, 0.0, 0.0),
            width as f32 / height as f32,
            std::f32::consts::FRAC_PI_3,
        );

        Self {
            device,
            start_instant,
            last_frame_instant,
            frame_instant,
            frame_time,
            camera,
        }
    }

    pub fn update(&mut self, event: &winit::event::Event<()>) {
        self.last_frame_instant = self.frame_instant;
        self.frame_instant = std::time::Instant::now();
        self.frame_time = self.last_frame_instant.elapsed().as_secs_f64();

        match event {
            winit::event::Event::WindowEvent { window_id, event } => {}
            winit::event::Event::DeviceEvent { device_id, event } => {
                match event {
                    winit::event::DeviceEvent::MouseMotion { delta } => {
                        self.camera
                            .process_mouse_movement(delta.0 as f32, delta.1 as f32);
                    }
                    winit::event::DeviceEvent::Key(input) => {
                        if let Some(code) = input.virtual_keycode {
                            match code {
                                winit::event::VirtualKeyCode::W => {
                                    self.camera.process_keyboard(
                                        Direction::Forward,
                                        0.5 * self.frame_time as f32,
                                    );
                                }
                                winit::event::VirtualKeyCode::S => {
                                    self.camera.process_keyboard(
                                        Direction::Backward,
                                        0.5 * self.frame_time as f32,
                                    );
                                }
                                winit::event::VirtualKeyCode::A => {
                                    self.camera.process_keyboard(
                                        Direction::Left,
                                        0.5 * self.frame_time as f32,
                                    );
                                }
                                winit::event::VirtualKeyCode::D => {
                                    self.camera.process_keyboard(
                                        Direction::Right,
                                        0.5 * self.frame_time as f32,
                                    );
                                }
                                winit::event::VirtualKeyCode::Q => {
                                    self.camera.process_keyboard(
                                        Direction::Down,
                                        0.5 * self.frame_time as f32,
                                    );
                                }
                                winit::event::VirtualKeyCode::E => {
                                    self.camera.process_keyboard(
                                        Direction::Up,
                                        0.5 * self.frame_time as f32,
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn render(&mut self) {}
}
