use std::collections::VecDeque;
use std::time::Duration;

use egui::{
    epaint::Shadow,
    Color32, CornerRadius, Frame, Id, Margin, Window, Ui, Vec2,
};
use egui_winit_vulkano::Gui;

#[derive(Debug, Clone)]
pub struct State {
    id: Id,
    open: bool,
    frame_timings: VecDeque<Duration>, // seconds per frame
}

impl State {
    pub fn render(&mut self, gui: &mut Gui, time: Option<Duration>) {
        let total_time = if let Some(time) = time {
            self.frame_timings.push_front(time);
            let mut total_time = Duration::default();
            let new_len = self.frame_timings.iter().take_while(|&&t| {
                total_time += t;
                total_time < Duration::from_secs(5)
            }).count() + 1;
            self.frame_timings.truncate(new_len);
            total_time
        } else {
            Duration::from_secs(1)
        };
        let fps = self.frame_timings.len() as f32 / total_time.as_secs_f32();

        gui.immediate_ui(|gui| {
            let ctx = gui.context();
            Window::new(format!("FPS: {fps:.2}"))
                .id(self.id)
                .open(&mut self.open)
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
        });
    }

    pub fn toggle_open(&mut self) {
        self.open = !self.open;
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
        //let time_max = *frame_timings.iter().max().unwrap();
        //let span = time_max - time_min;
        let time_scale = 1. / time_min.as_secs_f32();

        let size = Vec2::new(w, h);
        let (response, painter) = ui.allocate_painter(size, Sense::hover());
        let rect = response.rect;
        let canvas_scale = h - padding;
        let pixels_per_frame = (w - padding) / frame_timings.len() as f32;

        painter.text(
            rect.min + vec2(padding * 2., 0.),
            Align2::LEFT_TOP,
            format!("{time_scale:2.}"),
            FontId::monospace(10.),
            Color32::WHITE,
        );

        let stroke = Stroke::new(1.0, Color32::WHITE);
        let a = Pos2::new(rect.left() + padding, rect.top());
        let b = Pos2::new(rect.left() + padding, rect.bottom());
        painter.line_segment([a, b], stroke);
        let a = Pos2::new(rect.left(), rect.bottom() - padding);
        let b = Pos2::new(rect.right(), rect.bottom() - padding);
        painter.line_segment([a, b], stroke);

        let stroke = Stroke::new(1.0, Color32::GRAY);
        let y = 1. / time_scale / frame_timings[0].as_secs_f32();
        let mut start = Pos2::new(rect.right(), rect.bottom() - padding - y * canvas_scale);
        for timing in frame_timings.iter().skip(1) {
            let y = 1. / time_scale / timing.as_secs_f32();
            let end = Pos2::new(
                start.x - pixels_per_frame,
                rect.bottom() - padding - y * canvas_scale
            );
            painter.line_segment([start, end], stroke);
            start = end;
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            id: Id::new("fps indicator"),
            open: true,
            frame_timings: VecDeque::new(),
        }
    }
}
