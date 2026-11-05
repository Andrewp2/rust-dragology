use eframe::egui;
struct App(dragology_egui::scene::Canvas<hanoi::Hanoi>);
impl eframe::App for App {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        if ui.input(|i| i.key_pressed(egui::Key::R) && i.modifiers.is_none()) {
            self.0.set_model(Default::default());
        }
        ui.painter()
            .rect_filled(ui.max_rect(), 0., egui::Color32::from_rgb(246, 244, 235));
        let area = ui.available_size();
        dragology_egui::show(ui, &mut self.0, egui::vec2(area.x, (area.y - 32.).max(0.)));
        ui.horizontal(|ui| {
            ui.add_space(16.);
            ui.label(
                egui::RichText::new(self.0.model().status())
                    .color(egui::Color32::from_rgb(81, 70, 60))
                    .size(12.),
            );
        });
    }
}
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([hanoi::WIDTH as f32, hanoi::HEIGHT as f32 + 40.])
            .with_min_inner_size([540., 300.]),
        ..Default::default()
    };
    eframe::run_native(
        hanoi::TITLE,
        options,
        Box::new(|_| Ok(Box::new(App(hanoi::board())))),
    )
}
