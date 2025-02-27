use crate::vulkan::VkApp;

use std::{
    sync::Arc,
    time::Instant,
};

use anyhow::Context;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "shaderpixel";

struct FpsInfo {
    last_frame: Instant,
    last_count_start: Instant,
    frame_count: u32,
}

#[derive(Default)]
pub struct App {
    app: Option<(Arc<Window>, VkApp)>,
    swapchain_dirty: bool,

    /// Time passed since app start in fractional seconds.
    time: f32,
    /// Information about frame timing.
    fps_info: Option<FpsInfo>,
}

impl App {
    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), anyhow::Error> {
        let window_attrs = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT));
        let window = event_loop.create_window(window_attrs).context("Failed to create window")?;
        let window = Arc::new(window);
        let vk_app = VkApp::new(Arc::clone(&window));
        self.app = Some((window, vk_app));
        self.swapchain_dirty = true;
        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(err) = self.init(event_loop) {
            log::error!("Error while starting: {err:?}");
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => {
                event_loop.exit();
            }
            WindowEvent::Resized { .. } => {
                self.swapchain_dirty = true;
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if event_loop.exiting() {
            return;
        }

        let (window, vk_app) = self.app.as_mut().unwrap();

        let now = Instant::now();
        let fps_info = self.fps_info.get_or_insert(FpsInfo {
            last_frame: now,
            last_count_start: now,
            frame_count: 0,
        });
        self.time += fps_info.last_frame.elapsed().as_secs_f32();
        fps_info.last_frame = now;
        fps_info.frame_count += 1;
        {
            let time = fps_info.last_count_start.elapsed();
            if time.as_millis() > 1000 {
                use std::io::Write;

                eprint!("fps: {:.2}        \r", fps_info.frame_count as f32 / time.as_secs_f32());
                std::io::stdout().flush().unwrap();
                fps_info.last_count_start = now;
                fps_info.frame_count = 0;
            }
        }

        if self.swapchain_dirty {
            let size = window.inner_size();
            if size.width == 0 || size.height == 0 {
                return;
            }
            vk_app.recreate_swapchain(window.inner_size());
        }

        self.swapchain_dirty = vk_app.draw(self.time);
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        if let Some((_, _vk_app)) = self.app.as_ref() {
            //vk_app.wait_gpu_idle();
        }
    }
}
