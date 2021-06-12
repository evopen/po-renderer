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
        egui::CentralPanel::default().show(&self.ui_instance.context(), |ui| {
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
        });
        // egui::SidePanel::left("left panel", 500.0).show(&self.ui_instance.context(), |ui| {});
    }
}
