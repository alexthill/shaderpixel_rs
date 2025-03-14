mod app;
mod art;
mod art_objects;
mod fs;
mod gui;
mod model;
mod vulkan;

use app::App;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    env_logger::builder()
        .format_timestamp(Some(env_logger::fmt::TimestampPrecision::Millis))
        .init();

    let art_objects = match art_objects::get_art_objects() {
        Ok(art_objects) => art_objects,
        Err(err) => {
            log::error!("failed to load art objects: {err:?}");
            return;
        }
    };

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    app.art_objects = art_objects;
    event_loop.run_app(&mut app).unwrap();
}
