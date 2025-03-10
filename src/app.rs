use crate::{
    art::ArtObject,
    fs,
    gui::State,
    model::{
        env_generator::default_env,
        obj::NormalizedObj,
    },
    vulkan::{HotShader, VkApp},
};

use std::{
    sync::Arc,
    time::Instant,
};

use anyhow::Context;
use egui_winit_vulkano::{Gui, GuiConfig};
use glam::{Mat4, Quat, Vec3, Vec4};
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
    app: Option<(Arc<Window>, VkApp, Gui)>,
    swapchain_dirty: bool,
    state: State,
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
}

impl App {
    fn init(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
        let window_attrs = Window::default_attributes()
            .with_title(TITLE)
            .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT));
        let window = event_loop.create_window(window_attrs).context("Failed to create window")?;
        let window = Arc::new(window);

        let model = default_env().normalize()?;
        let art_objects = {
            let model_square = Arc::new(NormalizedObj::from_reader(fs::load("assets/models/square.obj")?)?);
            let model_cube = Arc::new(NormalizedObj::from_reader(fs::load("assets/models/cube_inside.obj")?)?);
            let shader_2d = Arc::new(HotShader::new_vert("assets/shaders/art2d.vert"));
            let shader_3d = Arc::new(HotShader::new_vert("assets/shaders/art3d.vert"));
            vec![
                ArtObject {
                    name: "mandelbrot".to_owned(),
                    model: model_square.clone(),
                    matrix: Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.5),
                        Quat::from_rotation_y(90_f32.to_radians()),
                        [5.99, 1.5, -1.5].into(),
                    ),
                    shader_vert: shader_2d.clone(),
                    shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mandelbrot.frag")),
                },
                ArtObject {
                    name: "sdf_cat".to_owned(),
                    model: model_square.clone(),
                    matrix: Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.5),
                        Quat::from_rotation_y(90_f32.to_radians()),
                        [5.99, 1.5, -4.5].into(),
                    ),
                    shader_vert: shader_2d.clone(),
                    shader_frag: Arc::new(HotShader::new_frag("assets/shaders/sdf_cat.frag")),
                },
                ArtObject {
                    name: "mandelbox".to_owned(),
                    model: model_cube.clone(),
                    matrix: Mat4::from_scale_rotation_translation(
                        Vec3::splat(0.5),
                        Quat::from_rotation_y(0_f32.to_radians()),
                        [-2.5, 1.51, -0.5].into(),
                    ),
                    shader_vert: shader_3d.clone(),
                    shader_frag: Arc::new(HotShader::new_frag("assets/shaders/mandelbox.frag")),
                },
            ]
        };
        let vk_app = VkApp::new(Arc::clone(&window), model, art_objects);
        let gui = Gui::new_with_subpass(
            event_loop,
            vk_app.get_swapchain().surface().clone(),
            vk_app.get_queue().clone(),
            vk_app.gui_pass(),
            vk_app.get_swapchain().image_format(),
            GuiConfig::default(),
        );

        self.state.options.present_modes = vk_app.get_surface_present_modes()?;
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
                    KeyCode::F2 if pressed => self.state.toggle_open(),
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
        if self.swapchain_dirty || self.state.options.recreate_swapchain {
            if extent.width == 0 || extent.height == 0 {
                return;
            }
            self.state.options.recreate_swapchain = false;
            if let Err(err) = vk_app.recreate_swapchain(extent, &self.state.options) {
                log::error!("error while recreating swapchain, exiting: {err:?}");
                event_loop.exit();
                return;
            }
        }

        // render gui
        self.state.render(gui, elapsed_dur);

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
        self.swapchain_dirty = match vk_app.draw(self.time, Some(gui)) {
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
