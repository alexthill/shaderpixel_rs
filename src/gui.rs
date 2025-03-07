use egui::{epaint::Shadow, Color32, CornerRadius, Frame, Margin, Window};
use egui_winit_vulkano::Gui;

#[derive(Debug, Clone)]
pub struct State {
    open: bool,
}

impl State {
    pub fn render(&mut self, gui: &mut Gui) {
        gui.immediate_ui(|gui| {
            let ctx = gui.context();
            Window::new("Transparent Window")
                .open(&mut self.open)
                .default_pos([25., 25.])
                .resizable(false)
                .default_width(300.0)
                .frame(
                    Frame::NONE
                        .fill(Color32::from_white_alpha(125))
                        .shadow(Shadow {
                            spread: 8,
                            blur: 10,
                            color: Color32::from_black_alpha(125),
                            ..Default::default()
                        })
                        .corner_radius(CornerRadius::same(5))
                        .inner_margin(Margin::same(10)),
                )
                .show(&ctx, |ui| {
                    ui.colored_label(Color32::BLACK, "some content");
                });
        });
    }

    pub fn toggle_open(&mut self) {
        self.open = !self.open;
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            open: true,
        }
    }
}
