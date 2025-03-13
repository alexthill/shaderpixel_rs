use crate::{
    art::{ArtObject, ArtUpdateData},
    gui::GuiState,
    model::{
        env_generator::default_env,
    },
    vulkan::VkApp,
};

use std::{
    sync::Arc,
    time::Instant,
};

use anyhow::Context;
use egui_winit_vulkano::{Gui, GuiConfig};
use glam::{Mat4, Vec3, Vec4};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    window::{Fullscreen, Window, WindowId},
};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;
const TITLE: &str = "shaderpixel";
const START_POSITION: Vec3 = Vec3::from_array([0., 1.5, 3.]);

#[derive(Debug)]
struct FpsInfo {
    last_frame: Instant,
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
    /// When in fly mode move into the direction the camera is looking, else move on the plane.
    fly_mode: bool,
}

#[derive(Default)]
struct KeyStates {
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
    pub art_objects: Vec<ArtObject>,
    app: Option<(Arc<Window>, VkApp, Gui)>,
    swapchain_dirty: bool,
    gui_state: GuiState,
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
    /// Whether the application is in fullscreen or not.
    is_fullscreen: bool,
    skybox_rotation_angle: f32,
}

impl App {
    fn init(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let window_attrs = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT));
        let window = event_loop.create_window(window_attrs).context("Failed to create window")?;
        let window = Arc::new(window);

        let model = default_env().normalize()?;
        let vk_app = VkApp::new(Arc::clone(&window), model, &self.art_objects);
        let gui = Gui::new_with_subpass(
            event_loop,
            vk_app.get_swapchain().surface().clone(),
            vk_app.get_queue().clone(),
            vk_app.gui_pass(),
            vk_app.get_swapchain().image_format(),
            GuiConfig::default(),
        );

        self.gui_state.options.present_modes = vk_app.get_surface_present_modes()?;
        self.app = Some((window, vk_app, gui));
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
        let (window, _, gui) = self.app.as_mut().unwrap();
        if gui.update(&event) {
            return;
        }

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
                    KeyCode::F1 if pressed => {
                        if self.is_fullscreen {
                            window.set_fullscreen(None);
                        } else {
                            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                        }
                        self.is_fullscreen = !self.is_fullscreen;
                    }
                    KeyCode::F2 if pressed => self.gui_state.toggle_open(),
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

        let (window, vk_app, gui) = self.app.as_mut().unwrap();

        // update fps info
        let now = Instant::now();
        let elapsed_dur = self.fps_info.as_ref().map(|info| now.duration_since(info.last_frame));
        let fps_info = self.fps_info.get_or_insert(FpsInfo {
            last_frame: now,
            frame_count: 0,
        });
        let elapsed = elapsed_dur.unwrap_or_default().as_secs_f32();
        self.time += elapsed;
        fps_info.last_frame = now;
        fps_info.frame_count += 1;

        // recreate swapchain if needed
        let extent = window.inner_size();
        if self.swapchain_dirty || self.gui_state.options.recreate_swapchain {
            if extent.width == 0 || extent.height == 0 {
                return;
            }
            self.gui_state.options.recreate_swapchain = false;
            if let Err(err) = vk_app.recreate_swapchain(extent, &self.gui_state.options) {
                log::error!("error while recreating swapchain, exiting: {err:?}");
                event_loop.exit();
                return;
            }
        }

        // setup nearest_art options
        let mut nearest_art = self.art_objects.iter_mut()
            .filter(|a| !a.options.is_empty())
            .min_by(|a, b| {
                let pos_a = a.data.get_matrix().transform_point3(Vec3::splat(0.));
                let dist_a = self.camera.position.distance_squared(pos_a);
                let pos_b = b.data.get_matrix().transform_point3(Vec3::splat(0.));
                let dist_b = self.camera.position.distance_squared(pos_b);
                dist_a.total_cmp(&dist_b)
            });
        if let Some(art) = nearest_art.as_mut() {
            let pos = art.data.get_matrix().transform_point3(Vec3::splat(0.));
            let dist = self.camera.position.distance_squared(pos);
            if dist > 2. || art.options.is_empty() {
                nearest_art = None;
            }
        }

        // render gui
        self.gui_state.render(gui, &mut nearest_art, elapsed_dur);

        // update options data for nearest_art
        if let Some(art) = nearest_art.as_mut() {
            if let Some(option_values) = art.data.get_options_mut() {
                let mut values = [0.; 4];
                let mut i = 0;
                for option in art.options.iter() {
                    option.ty.save_value(&mut values, &mut i);
                }
                *option_values = values.into();
            }
        }

        // update data for all art
        if self.gui_state.options.sun_movement {
            self.skybox_rotation_angle += elapsed * self.gui_state.options.sun_speed;
        }
        for art in self.art_objects.iter_mut() {
            if let Some(fn_update_data) = art.fn_update_data.as_ref() {
                fn_update_data(&mut art.data, &ArtUpdateData {
                    skybox_rotation_angle: self.skybox_rotation_angle,
                });
            }
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
        self.swapchain_dirty = match vk_app.draw(self.time, Some(gui), &self.art_objects) {
            Ok(swapchain_dirty) => swapchain_dirty,
            Err(err) => {
                log::error!("error while drawing, exiting: {err:?}");
                event_loop.exit();
                false
            }
        };
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        // nothing
    }
}
