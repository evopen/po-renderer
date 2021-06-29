use egui_maligog::egui;
use image::GenericImageView;

pub enum UiMessage {
    Render,
}

impl super::Engine {
    pub fn draw_ui(&mut self) -> Option<UiMessage> {
        let mut msg = None;
        egui::TopPanel::top(egui::Id::new("menu bar")).show(
            &self.ui_instance.context().clone(),
            |ui| {
                egui::menu::bar(ui, |ui| {
                    egui::menu::menu(ui, "File", |ui| {
                        if ui.button("Open Scene").clicked() {
                            match nfd2::open_file_dialog(Some("gltf,glb"), None).unwrap() {
                                nfd2::Response::Okay(p) => {
                                    log::info!("open {:?}", p);
                                    self.scene = Some(maligog_gltf::Scene::from_file(
                                        p.file_stem().map(|s| s.to_str().unwrap()),
                                        &self.device,
                                        &p,
                                    ));
                                }
                                nfd2::Response::OkayMultiple(p) => todo!(),
                                nfd2::Response::Cancel => {}
                            }
                        }
                        if ui.button("Import Skymap").clicked() {
                            match nfd2::open_file_dialog(Some("jpg,jpeg"), None).unwrap() {
                                nfd2::Response::Okay(p) => {
                                    let img = image::open(&p).unwrap();
                                    let img = img.into_rgba8();
                                    self.skymap = self.device.create_image_init(
                                        Some("skymap"),
                                        maligog::Format::R8G8B8A8_UNORM,
                                        img.width(),
                                        img.height(),
                                        maligog::ImageUsageFlags::SAMPLED,
                                        maligog::MemoryLocation::GpuOnly,
                                        &img.as_raw(),
                                    );
                                    self.skymap_view = self.skymap.create_view();
                                }
                                nfd2::Response::OkayMultiple(p) => todo!(),
                                nfd2::Response::Cancel => {}
                            }
                        }
                        if ui.button("Organize Windows").clicked() {
                            ui.ctx().memory().reset_areas();
                        }
                    });
                });
            },
        );
        egui::SidePanel::left("side", 100.0).show(&self.ui_instance.context(), |ui| {
            if ui.button("Wireframe").clicked() {
                self.scene_pass = self.wireframe.clone();
            }
            if ui.button("Ray Tracing").clicked() {
                self.scene_pass = self.ray_tracing.clone();
            }
        });
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(egui::Color32::from_rgb(0, 0, 0)))
            .show(&self.ui_instance.context(), |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.input.command)
                            .desired_rows(1)
                            .code_editor()
                            .lock_focus(true),
                    );
                    if response.changed() {
                        if let Some(last) = self.input.command.chars().last() {
                            if last == '\n' {
                                response.request_focus();
                                self.input.command.clear();
                            }
                        }
                    }
                });
                egui::Window::new("Camera").min_width(1600.0).show(
                    &self.ui_instance.context(),
                    |ui| {
                        ui.label(format!("Location: {}", self.camera.location));
                        ui.label(format!("Front: {}", self.camera.front));
                        ui.label(format!("Right: {}", self.camera.right));
                    },
                );
                egui::Window::new("Stats").min_width(1600.0).show(
                    &self.ui_instance.context(),
                    |ui| {
                        ui.label(format!("Frame time: {:.2}", self.frame_time * 1000.0));
                    },
                );
                egui::Window::new("Render").show(&self.ui_instance.context(), |ui| {
                    if ui.button("Render").clicked() {
                        msg = Some(UiMessage::Render);
                    }
                })
            });
        // egui::SidePanel::left("left panel", 500.0).show(&self.ui_instance.context(), |ui| {});

        return msg;
    }
}
