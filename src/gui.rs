use std::collections::VecDeque;
use std::time::Duration;

use egui::{
    epaint::Shadow,
    Align2, Color32, CornerRadius, Frame, Id, Margin, Window, Ui, Vec2,
};
use egui_winit_vulkano::Gui;
use vulkano::swapchain::PresentMode;

const FPS_CHART_MAX_TIME: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct Options {
    pub recreate_swapchain: bool,
    pub present_modes: Vec<PresentMode>,
    pub present_mode: PresentMode,
}

#[derive(Debug, Clone)]
pub struct State {
    id_fps: Id,
    open: bool,
    open_fps: bool,
    open_options: bool,
    frame_timings: VecDeque<Duration>,
    pub options: Options,
}

impl State {
    pub fn render(&mut self, gui: &mut Gui, time: Option<Duration>) {
        let total_time = if let Some(time) = time {
            self.frame_timings.push_front(time);
            let mut total_time = Duration::default();
            let new_len = self.frame_timings.iter().take_while(|&&t| {
                total_time += t;
                total_time < FPS_CHART_MAX_TIME
            }).count() + 1;
            self.frame_timings.truncate(new_len);
            total_time
        } else {
            Duration::from_secs(1)
        };
        let fps = self.frame_timings.len() as f32 / total_time.as_secs_f32();

        gui.immediate_ui(|gui| {
            let ctx = gui.context();
            ctx.set_visuals_of(egui::Theme::Dark, egui::Visuals {
                override_text_color: Some(Color32::LIGHT_GRAY),
                ..Default::default()
            });

            Window::new(format!("FPS: {fps:.2}"))
                .id(self.id_fps)
                .open(&mut self.open_fps)
                .default_pos([0., 0.])
                .resizable(false)
                .default_width(300.)
                .frame(Frame::NONE
                    .fill(Color32::from_black_alpha(96))
                    .shadow(Shadow::NONE)
                    .corner_radius(CornerRadius::ZERO)
                    .inner_margin(Margin::same(5)),
                )
                .show(&ctx, |ui| {
                    Frame::canvas(ui.style())
                        .multiply_with_opacity(0.5)
                        .show(ui, |ui| Self::draw_fps_chart(ui, &self.frame_timings));
                });

            Window::new("Options")
                .open(&mut self.open_options)
                .anchor(Align2::RIGHT_TOP, [0., 0.])
                .resizable(false)
                .default_width(300.)
                .frame(Frame::NONE
                    .fill(Color32::from_black_alpha(96))
                    .shadow(Shadow::NONE)
                    .corner_radius(CornerRadius::ZERO)
                    .inner_margin(Margin::same(5)),
                )
                .show(&ctx, |ui| {
                    egui::Grid::new("options_grid")
                        .num_columns(2)
                        .spacing([40.0, 4.0])
                        .striped(true)
                        .show(ui, |ui| {
                            Self::options_grid_contents(ui, &mut self.options);
                        });
                });
        });
    }

    pub fn toggle_open(&mut self) {
        self.open = !self.open;
        self.open_fps = self.open;
        self.open_options = self.open;
    }

    fn options_grid_contents(ui: &mut Ui, state: &mut Options) {
        fn present_mode_label(mode: PresentMode) -> &'static str {
            match mode {
                PresentMode::Immediate => "Immediate",
                PresentMode::Mailbox => "Mailbox",
                PresentMode::Fifo => "Fifo",
                PresentMode::FifoRelaxed => "FifoRelaxed",
                _ => "Other",
            }
        }

        ui.label("Present Mode")
            .on_hover_ui(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Sets the vulkan present mode.");
                });
            });
        let present_mode_old = state.present_mode;
        egui::ComboBox::from_id_salt("Present mode")
            .selected_text(present_mode_label(present_mode_old))
            .show_ui(ui, |ui| {
                for &mode in state.present_modes.iter() {
                    ui.selectable_value(&mut state.present_mode, mode, present_mode_label(mode));
                }
                if state.present_mode != present_mode_old {
                    state.recreate_swapchain = true;
                }
            });
        ui.end_row();
    }

    fn draw_fps_chart(ui: &mut Ui, frame_timings: &VecDeque<Duration>) {
        use egui::{
            vec2, Align2, FontId, Pos2, Sense, Stroke,
        };

        if frame_timings.is_empty() {
            return;
        }

        let w = 250.;
        let h = 100.;
        let padding = 5.;

        let time_min = *frame_timings.iter().min().unwrap();
        let time_scale = 1. / time_min.as_secs_f32();

        let size = Vec2::new(w, h);
        let (response, painter) = ui.allocate_painter(size, Sense::hover());
        let rect = response.rect;
        let canvas_scale = h - padding;
        let pixels_per_sec = (w - padding) / FPS_CHART_MAX_TIME.as_secs_f32();

        // draw lines
        let stroke = Stroke::new(1.0, Color32::GRAY);
        let y = 1. / time_scale / frame_timings[0].as_secs_f32();
        let mut start = Pos2::new(rect.right(), rect.bottom() - padding - y * canvas_scale);
        for timing in frame_timings.iter().skip(1) {
            let y = 1. / time_scale / timing.as_secs_f32();
            let end = Pos2::new(
                start.x - pixels_per_sec * timing.as_secs_f32(),
                rect.bottom() - padding - y * canvas_scale
            );
            painter.line_segment([start, end], stroke);
            start = end;
        }

        // draw axis
        let stroke = Stroke::new(1.0, Color32::WHITE);
        let a = Pos2::new(rect.left() + padding, rect.top());
        let b = Pos2::new(rect.left() + padding, rect.bottom());
        painter.line_segment([a, b], stroke);
        let a = Pos2::new(rect.left(), rect.bottom() - padding);
        let b = Pos2::new(rect.right(), rect.bottom() - padding);
        painter.line_segment([a, b], stroke);

        // draw max fps
        painter.text(
            rect.min + vec2(padding * 2., 0.),
            Align2::LEFT_TOP,
            format!("{time_scale:2.}"),
            FontId::monospace(10.),
            Color32::WHITE,
        );
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            id_fps: Id::new("fps indicator"),
            open: true,
            open_fps: true,
            open_options: true,
            frame_timings: VecDeque::new(),
            options: Options {
                recreate_swapchain: false,
                present_modes: Vec::new(),
                present_mode: PresentMode::Fifo,
            },
        }
    }
}
