mod camera;
mod input;
mod scene_pass;
mod ui;
pub mod util;

use std::cell::RefCell;
use std::env;
use std::rc::Rc;
use std::str::FromStr;

use egui_winit_platform::PlatformDescriptor;

use maligog::vk;

use crate::{vec3, Vec3};
pub use camera::{Camera, Direction};

use egui_maligog::egui;

pub struct Engine {
    device: maligog::Device,
    swapchain: maligog::Swapchain,
    start_instant: std::time::Instant,
    last_frame_instant: std::time::Instant,
    frame_instant: std::time::Instant,
    frame_time: f64,
    camera: Camera,
    ui_pass: egui_maligog::UiPass,
    ui_instance: egui_winit_platform::Platform,
    scale_factor: f64,
    width: u32,
    height: u32,
    paint_jobs: Vec<egui::ClippedMesh>,
    scene: Option<maligog_gltf::Scene>,
    input: input::Input,
    scene_pass: Rc<RefCell<dyn scene_pass::ScenePass>>,
    wireframe: Rc<RefCell<scene_pass::Wireframe>>,
    ray_tracing: Rc<RefCell<scene_pass::RayTracing>>,
    skymap: Option<maligog::Image>,
    skymap_view: Option<maligog::ImageView>,
}

impl Engine {
    pub fn new(window: &winit::window::Window) -> Self {
        let entry = maligog::Entry::new().unwrap();
        let required_extensions = maligog::Surface::required_extensions();
        let instance = entry.create_instance(
            &[maligog::name::instance::Layer::LunargMonitor],
            &required_extensions,
        );
        let device = instance
            .enumerate_physical_device()
            .into_iter()
            .find(|p| p.device_type() == maligog::PhysicalDeviceType::DISCRETE_GPU)
            .unwrap()
            .create_device();
        let surface = instance.create_surface(window);
        let swapchain = device.create_swapchain(surface, maligog::PresentModeKHR::FIFO);

        let start_instant = std::time::Instant::now();
        let frame_instant = start_instant;
        let last_frame_instant = start_instant;
        let frame_time = 0.0;
        let width = window.inner_size().width;
        let height = window.inner_size().height;

        let camera = Camera::new(
            vec3(0.0, 0.0, 10.0),
            vec3(0.0, 0.0, 0.0),
            width as f32 / height as f32,
            std::f32::consts::FRAC_PI_3,
        );

        let move_speed = 1000.0;
        let in_control = false;

        let scale_factor = window.scale_factor();

        let ui_pass = egui_maligog::UiPass::new(&device);
        let ui_instance = egui_winit_platform::Platform::new(PlatformDescriptor {
            physical_width: width,
            physical_height: height,
            scale_factor: scale_factor,
            font_definitions: egui::FontDefinitions::default(),
            style: egui::Style::default(),
        });

        let wireframe = Rc::new(RefCell::new(scene_pass::Wireframe::new(&device)));
        let ray_tracing = Rc::new(RefCell::new(scene_pass::RayTracing::new(
            &device, width, height,
        )));
        let scene_pass = ray_tracing.clone();

        let scene = match env::var("DEFAULT_SCENE") {
            Ok(p) => {
                let p = std::path::PathBuf::from_str(&p).unwrap();
                Some(maligog_gltf::Scene::from_file(
                    Some(p.file_stem().unwrap().to_str().unwrap()),
                    &device,
                    &p,
                ))
            }
            Err(_) => None,
        };
        let skymap = match env::var("DEFAULT_SKYMAP") {
            Ok(p) => {
                log::info!("loading skymap");
                let p = std::path::PathBuf::from_str(&p).unwrap();
                let img = image::open(&p).unwrap();
                let img = img.into_rgba8();
                Some(device.create_image_init(
                    Some("skymap"),
                    maligog::Format::R8G8B8A8_UNORM,
                    img.width(),
                    img.height(),
                    maligog::ImageUsageFlags::SAMPLED,
                    maligog::MemoryLocation::GpuOnly,
                    &img.as_raw(),
                ))
            }
            Err(_) => None,
        };
        let skymap_view = match &skymap {
            Some(s) => Some(s.create_view()),
            None => None,
        };

        Self {
            device,
            swapchain,
            start_instant,
            last_frame_instant,
            frame_instant,
            frame_time,
            camera,
            ui_pass,
            ui_instance,
            scale_factor,
            width,
            height,
            paint_jobs: vec![],
            scene,
            input: input::Input {
                move_speed,
                ..Default::default()
            },
            scene_pass: scene_pass,
            wireframe,
            ray_tracing,
            skymap,
            skymap_view,
        }
    }

    pub fn update(&mut self, event: &winit::event::Event<()>) {
        self.scene_pass.borrow_mut().update();
        self.ui_instance.handle_event(event);
        self.ui_instance
            .update_time(self.start_instant.elapsed().as_secs_f64());

        self.ui_instance.begin_frame();
        self.draw_ui();
        let (_, paint_commands) = self.ui_instance.end_frame();
        self.paint_jobs = self.ui_instance.context().tessellate(paint_commands);

        self.ui_pass.update_buffers(
            &self.paint_jobs,
            &egui_maligog::ScreenDescriptor {
                physical_width: self.width,
                physical_height: self.height,
                scale_factor: self.scale_factor as f32,
            },
        );
        self.ui_pass
            .update_texture(&self.ui_instance.context().texture());

        self.last_frame_instant = self.frame_instant;
        self.frame_instant = std::time::Instant::now();
        self.frame_time = self.last_frame_instant.elapsed().as_secs_f64();

        use winit::event::{ElementState, MouseButton};
        match event {
            winit::event::Event::WindowEvent { window_id, event } => {
                match event {
                    winit::event::WindowEvent::MouseInput { state, button, .. } => {
                        if button.eq(&MouseButton::Right) {
                            if state.eq(&ElementState::Pressed) {
                                self.input.in_control = true;
                            } else if state.eq(&ElementState::Released) {
                                self.input.in_control = false;
                            }
                        }
                    }
                    _ => {}
                }
            }
            winit::event::Event::DeviceEvent { device_id, event } => {
                match event {
                    winit::event::DeviceEvent::MouseMotion { delta } => {
                        if self.input.in_control {
                            self.camera.process_mouse_movement(
                                delta.0 as f32 / 1000.0,
                                delta.1 as f32 / 1000.0,
                            );
                        }
                    }
                    winit::event::DeviceEvent::Key(input) => {
                        if self.input.in_control {
                            self.process_key(input);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        // log::info!(
        //     "{} {}",
        //     self.camera.location.to_string(),
        //     self.camera.front.to_string()
        // );
    }

    pub fn render(&mut self) {
        if let Ok(index) = self.swapchain.acquire_next_image() {
            let frame = self.swapchain.get_image(index);

            let mut cmd_buf = self.device.create_command_buffer(
                Some("main cmd buf"),
                self.device.graphics_queue_family_index(),
            );
            cmd_buf.encode(|rec| {
                if let Some(scene) = &self.scene {
                    self.scene_pass.borrow_mut().execute(
                        rec,
                        scene,
                        &frame.create_view(),
                        &self.camera,
                        Some(maligog::ClearColorValue {
                            float32: [1.0, 1.0, 1.0, 1.0],
                        }),
                        self.skymap_view.as_ref().unwrap(),
                    );
                    self.ui_pass.execute(
                        rec,
                        &frame,
                        &self.paint_jobs,
                        &egui_maligog::ScreenDescriptor {
                            physical_width: self.width,
                            physical_height: self.height,
                            scale_factor: self.scale_factor as f32,
                        },
                        None,
                    );
                } else {
                    self.ui_pass.execute(
                        rec,
                        &frame,
                        &self.paint_jobs,
                        &egui_maligog::ScreenDescriptor {
                            physical_width: self.width,
                            physical_height: self.height,
                            scale_factor: self.scale_factor as f32,
                        },
                        Some(maligog::ClearColorValue {
                            float32: [1.0, 1.0, 1.0, 1.0],
                        }),
                    );
                }
            });
            self.device.graphics_queue().submit_blocking(&[cmd_buf]);
            self.swapchain
                .present(index, &[&self.swapchain.image_available_semaphore()]);
        }
    }
}
