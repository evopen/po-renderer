use super::camera::Direction;

impl super::Engine {
    pub fn process_key(&mut self, keyboard_input: &winit::event::KeyboardInput) {
        if !self.in_control {
            return;
        }
        if let Some(keycode) = keyboard_input.virtual_keycode {
            match keycode {
                winit::event::VirtualKeyCode::W => {
                    self.camera.process_keyboard(
                        Direction::Forward,
                        self.move_speed * self.frame_time as f32,
                    );
                }
                winit::event::VirtualKeyCode::S => {
                    self.camera.process_keyboard(
                        Direction::Backward,
                        self.move_speed * self.frame_time as f32,
                    );
                }
                winit::event::VirtualKeyCode::A => {
                    self.camera.process_keyboard(
                        Direction::Left,
                        self.move_speed * self.frame_time as f32,
                    );
                }
                winit::event::VirtualKeyCode::D => {
                    self.camera.process_keyboard(
                        Direction::Right,
                        self.move_speed * self.frame_time as f32,
                    );
                }
                winit::event::VirtualKeyCode::Q => {
                    self.camera.process_keyboard(
                        Direction::Down,
                        self.move_speed * self.frame_time as f32,
                    );
                }
                winit::event::VirtualKeyCode::E => {
                    self.camera
                        .process_keyboard(Direction::Up, self.move_speed * self.frame_time as f32);
                }
                _ => {}
            }
        }
    }
}