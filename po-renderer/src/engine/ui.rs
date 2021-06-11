use egui_maligog::egui;

impl super::Engine {
    pub fn draw_ui(&mut self) {
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
                        if ui.button("Organize Windows").clicked() {
                            ui.ctx().memory().reset_areas();
                        }
                    });
                });
            },
        );
    }
}
