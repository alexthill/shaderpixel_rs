use crate::model::obj::NormalizedObj;
use crate::vulkan::HotShader;

use std::path::PathBuf;
use std::sync::Arc;

use egui::Color32;
use glam::{Mat4, Vec3, Vec4};

pub type UpdateFunction = dyn Fn(&mut ArtData, &ArtUpdateData);

#[derive(Default)]
pub struct ArtObject {
    pub name: String,
    pub model: Arc<NormalizedObj>,
    pub shader_vert: Arc<HotShader>,
    pub shader_frag: Arc<HotShader>,
    pub texture: Option<PathBuf>,
    pub options: Vec<ArtOption>,
    pub data: ArtData,
    pub fn_update_data: Option<Box<UpdateFunction>>,
}

impl ArtObject {
    pub fn position(&self) -> Vec3 {
        self.data.position()
    }
}

#[derive(Debug, Default)]
pub struct ArtUpdateData {
    pub skybox_rotation_angle: f32,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ArtData {
    pub dist_to_camera: f32,
    pub matrix: Mat4,
    pub light_pos: Vec4,
    pub option_values: Option<Vec4>,
}

impl ArtData {
    pub fn new(matrix: Mat4) -> Self {
        Self {
            matrix,
            ..Default::default()
        }
    }

    pub fn position(&self) -> Vec3 {
        self.matrix.transform_point3(Vec3::splat(0.))
    }
}

pub enum ArtOptionType {
    Checkbox { checked: bool },
    SliderF32 { value: f32, min: f32, max: f32 },
    SliderI32 { value: i32, min: i32, max: i32 },
    Stroke { width: f32, color: Color32 },
}

impl ArtOptionType {
    pub fn save_value(&self, values: &mut [f32], i: &mut usize) {
        match self {
            Self::Checkbox { checked } => {
                values[*i] = if *checked { 1. } else { 0. };
                *i += 1;
            }
            Self::SliderF32 { value, .. } => {
                values[*i] = *value;
                *i += 1;
            }
            Self::SliderI32 { value, .. } => {
                values[*i] = *value as f32;
                *i += 1;
            }
            Self::Stroke { color, .. } => {
                for &component in &color.to_array()[..3] {
                    values[*i] = component as f32 / 255.;
                    *i += 1;
                }
            }
        }
    }
}

pub struct ArtOption {
    label: &'static str,
    pub ty: ArtOptionType,
}

impl ArtOption {
    pub fn checkbox(label: &'static str, checked: bool) -> Self {
        Self { label, ty: ArtOptionType::Checkbox { checked } }
    }

    pub fn slider_f32(label: &'static str, value: f32, min: f32, max: f32) -> Self {
        Self { label, ty: ArtOptionType::SliderF32 { value, min, max } }
    }

    pub fn slider_i32(label: &'static str, value: i32, min: i32, max: i32) -> Self {
        Self { label, ty: ArtOptionType::SliderI32 { value, min, max } }
    }

    pub fn stroke(label: &'static str, width: f32, color: Color32) -> Self {
        Self { label, ty: ArtOptionType::Stroke { width, color } }
    }

    pub fn label(&self) -> &str {
        self.label
    }
}
