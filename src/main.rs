mod app;
mod art;
mod fs;
mod model;
mod vulkan;

use app::App;

use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
