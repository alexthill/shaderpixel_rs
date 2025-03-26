use std::f32::consts::PI;

use glam::{Mat4, Vec3, Vec4};

#[derive(Default)]
pub struct KeyStates {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub lmb: bool,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Camera {
    /// Camera yaw angle in radians.
    pub angle_yaw: f32,
    /// Camera pitch angle in radians.
    pub angle_pitch: f32,
    /// Camera position.
    pub position: Vec3,
    /// When in fly mode move into the direction the camera is looking, else move on the plane.
    pub fly_mode: bool,
}

impl Camera {
    pub fn update(&mut self, key_states: &KeyStates, delta: f32, x_ratio: f32, y_ratio: f32) {
        if key_states.lmb {
            self.angle_yaw += x_ratio * PI;
            self.angle_pitch += y_ratio * PI;
        }
        let translation = Vec4::from_array([
            (key_states.left    as i8 - key_states.right    as i8) as f32,
            (key_states.down    as i8 - key_states.up       as i8) as f32,
            (key_states.forward as i8 - key_states.backward as i8) as f32,
            0.
        ]) * delta * 2.;
        let rot = if self.fly_mode {
            Mat4::from_rotation_y(-self.angle_yaw)
                * Mat4::from_rotation_x(-self.angle_pitch)
        } else {
            Mat4::from_rotation_y(-self.angle_yaw)
        };
        self.position += (rot * -translation).truncate();
    }

    pub fn view_matrix(&self) -> Mat4 {
        return Mat4::from_rotation_x(self.angle_pitch)
            * Mat4::from_rotation_y(self.angle_yaw)
            * Mat4::from_translation(-self.position)
    }
}
