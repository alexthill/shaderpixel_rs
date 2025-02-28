use crate::model::env_generator::default_env;
use crate::vulkan::VkApp;

use std::{
    sync::Arc,
    time::Instant,
};

use anyhow::Context;
use glam::{Mat4, Vec3, Vec4};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    window::{Window, WindowId},
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "shaderpixel";
const START_POSITION: Vec3 = Vec3::from_array([0., 1.5, 3.]);

struct FpsInfo {
    last_frame: Instant,
    last_count_start: Instant,
    frame_count: u32,
}

#[derive(Default)]
struct Camera{
    /// Camera yaw angle in radians.
    angle_yaw: f32,
    /// Camera pitch angle in radians.
    angle_pitch: f32,
    /// Camera position.
    position: Vec3,
    /// When in fly mode move into the direction the camera is looking.
    /// Else move on the plane.
    fly_mode: bool,
}

#[derive(Default)]
pub struct KeyStates {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    lmb: bool,
}

#[derive(Default)]
pub struct App {
    app: Option<(Arc<Window>, VkApp)>,
    swapchain_dirty: bool,

    /// Time passed since app start in fractional seconds.
    time: f32,
    /// Information about frame timing.
    fps_info: Option<FpsInfo>,
    /// Information about the current camera position and orientation.
    camera: Camera,
    /// Rembers for some keys if they are pressed
    key_states: KeyStates,
    /// Number of lines scrolled. Used to determine movement speed.
    scroll_lines: f32,
    /// Current cursor postion.
    cursor_position: Option<[i32; 2]>,
    /// Movement delta of cursor since last frame.
    cursor_delta: [i32; 2],
}

impl App {
    fn init(&mut self, event_loop: &ActiveEventLoop) -> Result<(), anyhow::Error> {
        let window_attrs = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT));
        let window = event_loop.create_window(window_attrs).context("Failed to create window")?;
        let window = Arc::new(window);

        let model = default_env().normalize()?;
        let vk_app = VkApp::new(Arc::clone(&window), model);

        self.app = Some((window, vk_app));
        self.swapchain_dirty = true;
        self.camera.position = START_POSITION;
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
            WindowEvent::Resized { .. } => {
                self.swapchain_dirty = true;
            }
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
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        logical_key,
                        physical_key: PhysicalKey::Code(physical_key_code),
                        repeat: false,
                        ..
                    },
                ..
            } => {
                let pressed = state.is_pressed();
                match physical_key_code {
                    KeyCode::KeyW => self.key_states.forward = pressed,
                    KeyCode::KeyA => self.key_states.left = pressed,
                    KeyCode::KeyS => self.key_states.backward = pressed,
                    KeyCode::KeyD => self.key_states.right = pressed,
                    KeyCode::Space => self.key_states.up = pressed,
                    KeyCode::ShiftLeft => self.key_states.down = pressed,
                    KeyCode::ControlLeft if pressed => self.camera.fly_mode = !self.camera.fly_mode,
                    _ => {}
                }
                match (logical_key.as_ref(), pressed) {
                    (Key::Character("l"), true) => {
                        self.camera.angle_yaw = 0.;
                        self.camera.angle_pitch = 0.;
                        self.camera.position = START_POSITION;
                        self.scroll_lines = 0.0;
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseInput { button: MouseButton::Left, state, .. } => {
                self.key_states.lmb = state == ElementState::Pressed;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let new_pos: (i32, i32) = position.into();
                if self.key_states.lmb {
                    if let Some(old_pos) = self.cursor_position {
                        self.cursor_delta[0] += new_pos.0 - old_pos[0];
                        self.cursor_delta[1] += new_pos.1 - old_pos[1];
                    }
                }
                self.cursor_position = Some([new_pos.0, new_pos.1]);
            }
            WindowEvent::MouseWheel {
                delta: MouseScrollDelta::LineDelta(_, v_lines),
                ..
            } => {
                self.scroll_lines += v_lines;
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if event_loop.exiting() {
            return;
        }

        let (window, vk_app) = self.app.as_mut().unwrap();

        // update fps info
        let now = Instant::now();
        let fps_info = self.fps_info.get_or_insert(FpsInfo {
            last_frame: now,
            last_count_start: now,
            frame_count: 0,
        });
        let elapsed = fps_info.last_frame.elapsed().as_secs_f32();
        self.time += elapsed;
        fps_info.last_frame = now;
        fps_info.frame_count += 1;

        // print fps
        let time = fps_info.last_count_start.elapsed();
        if time.as_millis() > 1000 {
            use std::io::Write;

            eprint!("fps: {:.2}        \r", fps_info.frame_count as f32 / time.as_secs_f32());
            std::io::stdout().flush().unwrap();
            fps_info.last_count_start = now;
            fps_info.frame_count = 0;
        }

        // recreate swapchain if needed
        let extent = window.inner_size();
        if self.swapchain_dirty {
            if extent.width == 0 || extent.height == 0 {
                return;
            }
            vk_app.recreate_swapchain(extent);
        }

        // update position
        let delta = elapsed * (self.scroll_lines * 0.4).exp();
        let x_ratio = self.cursor_delta[0] as f32 / extent.width as f32;
        let y_ratio = self.cursor_delta[1] as f32 / extent.height as f32;

        if self.key_states.lmb {
            use std::f32::consts::PI;
            self.camera.angle_yaw += x_ratio * PI;
            self.camera.angle_pitch += y_ratio * PI;
        }
        self.cursor_delta = [0, 0];

        let translation = Vec4::from_array([
            (self.key_states.left    as i8 - self.key_states.right    as i8) as f32,
            (self.key_states.down    as i8 - self.key_states.up       as i8) as f32,
            (self.key_states.forward as i8 - self.key_states.backward as i8) as f32,
            0.
        ]) * delta * 2.;
        let rot = if self.camera.fly_mode {
            Mat4::from_rotation_y(-self.camera.angle_yaw)
                * Mat4::from_rotation_x(-self.camera.angle_pitch)
        } else {
            Mat4::from_rotation_y(-self.camera.angle_yaw)
        };
        self.camera.position += (rot * -translation).truncate();

        vk_app.view_matrix = Mat4::from_rotation_x(self.camera.angle_pitch)
            * Mat4::from_rotation_y(self.camera.angle_yaw)
            * Mat4::from_translation(-self.camera.position);


        // draw and remember if swapchain is dirty
        self.swapchain_dirty = vk_app.draw(self.time);
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        if let Some((_, _vk_app)) = self.app.as_ref() {
            //vk_app.wait_gpu_idle();
        }
    }
}
