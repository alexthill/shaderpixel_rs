use crate::{
    art::{ArtObject, ArtUpdateData},
    camera::{Camera, KeyStates},
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
    /// Current cursor position.
    cursor_position: Option<[i32; 2]>,
    /// Movement delta of cursor since last frame.
    cursor_delta: [i32; 2],
    /// Whether the application is in fullscreen or not.
    is_fullscreen: bool,
    skybox_rotation_angle: f32,
    box_idx: Option<usize>,
    mirror_idx: Option<usize>,
}

impl App {
    fn init(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let window_attrs = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT));
        let window = event_loop.create_window(window_attrs).context("Failed to create window")?;
        let window = Arc::new(window);

        let model = default_env().normalize()?;
        let vk_app = VkApp::new(Arc::clone(&window), model, &self.art_objects)?;
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
        self.box_idx = self.art_objects.iter().position(|art| art.name == "Portalbox");
        self.mirror_idx = self.art_objects.iter().position(|art| art.name == "Mirror");

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
        let Some((window, _, gui)) = self.app.as_mut() else { return };
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
                        for art_obj in self.art_objects.iter_mut() {
                            art_obj.data.inside_portal = false;
                        }
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
        for art in self.art_objects.iter_mut() {
            let dist = self.camera.position.distance_squared(art.position());
            art.data.dist_to_camera_sqr = dist;
        }
        let mut nearest_art = self.art_objects.iter_mut()
            .filter(|art| art.enable_pipeline && !art.options.is_empty()
                && art.data.dist_to_camera_sqr <= 2.25)
            .min_by(|a, b| {
                a.data.dist_to_camera_sqr.total_cmp(&b.data.dist_to_camera_sqr)
            });

        // render gui
        self.gui_state.render(gui, &mut nearest_art, elapsed_dur);

        // update camera
        let old_position = self.camera.position;
        let delta = elapsed * (self.scroll_lines * 0.4).exp();
        let x_ratio = self.cursor_delta[0] as f32 / extent.width as f32;
        let y_ratio = self.cursor_delta[1] as f32 / extent.height as f32;
        self.camera.update(&self.key_states, delta, x_ratio, y_ratio);
        self.cursor_delta = [0, 0];
        vk_app.view_matrix = self.camera.view_matrix();

        // update options data for nearest_art
        if let Some(art) = nearest_art.as_mut() {
            art.save_options();
        }

        // update data for all art
        if self.gui_state.options.sun_movement {
            self.skybox_rotation_angle += elapsed * self.gui_state.options.sun_speed;
        }
        let light_pos = Mat4::from_rotation_y(self.skybox_rotation_angle) * Vec4::splat(100.);
        for art in self.art_objects.iter_mut() {
            art.data.light_pos = light_pos;
            if let Some(fn_update_data) = art.fn_update_data.as_ref() {
                fn_update_data(&mut art.data, &ArtUpdateData {
                    skybox_rotation_angle: self.skybox_rotation_angle,
                    old_position,
                    new_position: self.camera.position,
                    camera: self.camera,
                });
            }
        }

        // handle portal
        if let (Some(box_idx), Some(portal_idx))
            = (self.box_idx, self.art_objects.iter().position(|art| art.data.inside_portal))
        {
            let portal_dist = self.art_objects[portal_idx].data.dist_to_camera_sqr;
            for art in self.art_objects.iter_mut() {
                art.enable_pipeline = art.data.dist_to_camera_sqr > portal_dist;
            }

            let portal = &self.art_objects[portal_idx];
            let (d, vs, fs) = (portal.data, portal.shader_vert.clone(), portal.shader_frag.clone());
            let box_obj = &mut self.art_objects[box_idx];
            box_obj.enable_pipeline = true;
            box_obj.data.matrix = d.matrix;
            box_obj.data.option_values = d.option_values;
            box_obj.data.option_values[1][3] = 1.;
            box_obj.shader_vert = vs;
            box_obj.shader_frag = fs;
        } else {
            for art in self.art_objects.iter_mut() {
                art.enable_pipeline = true;
            }
            self.art_objects[self.box_idx.unwrap()].enable_pipeline = false;
        }

        // handle mirror
        if let Some(mirror_idx) = self.mirror_idx {
            vk_app.mirror_matrix = self.art_objects[mirror_idx].data.matrix;
        }

        // draw and remember if swapchain is dirty
        vk_app.fov = self.gui_state.options.fov;
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
