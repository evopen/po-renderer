use egui_maligog::{egui, ScreenDescriptor};
use egui_winit_platform::PlatformDescriptor;

pub struct Profiler {
    device: maligog::Device,
    ui_pass: egui_maligog::UiPass,
    ui_instance: egui_winit_platform::Platform,
    start_time: std::time::Instant,
    scale_factor: f64,
    paint_jobs: Vec<egui::ClippedMesh>,
    width: u32,
    height: u32,
}

impl Profiler {
    pub fn new(window: &winit::window::Window) -> Self {
        let scale_factor = window.scale_factor();
        let width = window.inner_size().width;
        let height = window.inner_size().width;

        let entry = maligog::Entry::new().unwrap();
        let required_extensions = maligog::Surface::required_extensions();
        let instance = entry.create_instance(&[], &&required_extensions);
        let device = instance
            .enumerate_physical_device()
            .into_iter()
            .find(|p| p.device_type() == maligog::PhysicalDeviceType::DISCRETE_GPU)
            .unwrap()
            .create_device();
        let surface = instance.create_surface(window);
        let swapchain = device.create_swapchain(surface, maligog::PresentModeKHR::FIFO);
        let ui_pass = egui_maligog::UiPass::new(&device);
        let ui_instance = egui_winit_platform::Platform::new(PlatformDescriptor {
            physical_width: width,
            physical_height: height,
            scale_factor: scale_factor,
            font_definitions: egui::FontDefinitions::default(),
            style: egui::Style::default(),
        });
        let start_time = std::time::Instant::now();
        Self {
            device,
            ui_pass,
            start_time,
            scale_factor,
            ui_instance,
            paint_jobs: vec![],
            width,
            height,
        }
    }

    pub fn update(&mut self, event: &winit::event::Event<()>) {
        self.ui_instance.handle_event(event);
        self.ui_instance
            .update_time(self.start_time.elapsed().as_secs_f64());
        self.ui_instance.begin_frame();
        puffin_egui::profiler_window(&self.ui_instance.context());
        let (_, paint_commands) = self.ui_instance.end_frame();
        self.paint_jobs = self.ui_instance.context().tessellate(paint_commands);
        self.ui_pass.update_buffers(
            &self.paint_jobs,
            &ScreenDescriptor {
                physical_width: self.width,
                physical_height: self.height,
                scale_factor: self.scale_factor as f32,
            },
        );
        self.ui_pass
            .update_texture(&self.ui_instance.context().texture());
    }

    pub fn render(&mut self) {}
}
