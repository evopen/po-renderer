pub struct Engine {
    device: maligog::Device,
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

        Self { device }
    }

    pub fn update(&mut self, event: &winit::event::Event<()>) {}

    pub fn render(&mut self) {}
}
