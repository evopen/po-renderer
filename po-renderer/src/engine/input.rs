use super::camera::Direction;

#[derive(Debug, Default)]
pub struct Input {
    pub(super) command: String,
    pub(super) move_speed: f32,
    pub(super) in_control: bool,
}

impl super::Engine {
    pub fn process_key(&mut self, keyboard_input: &winit::event::KeyboardInput) {
        if let Some(keycode) = keyboard_input.virtual_keycode {
            match keycode {
                winit::event::VirtualKeyCode::W => {
                    self.camera.process_keyboard(
                        Direction::Forward,
                        self.input.move_speed * self.frame_time as f32,
                    );
                }
                winit::event::VirtualKeyCode::S => {
                    self.camera.process_keyboard(
                        Direction::Backward,
                        self.input.move_speed * self.frame_time as f32,
                    );
                }
                winit::event::VirtualKeyCode::A => {
                    self.camera.process_keyboard(
                        Direction::Left,
                        self.input.move_speed * self.frame_time as f32,
                    );
                }
                winit::event::VirtualKeyCode::D => {
                    self.camera.process_keyboard(
                        Direction::Right,
                        self.input.move_speed * self.frame_time as f32,
                    );
                }
                winit::event::VirtualKeyCode::Q => {
                    self.camera.process_keyboard(
                        Direction::Down,
                        self.input.move_speed * self.frame_time as f32,
                    );
                }
                winit::event::VirtualKeyCode::E => {
                    self.camera.process_keyboard(
                        Direction::Up,
                        self.input.move_speed * self.frame_time as f32,
                    );
                }
                _ => {}
            }
        }
    }
}
