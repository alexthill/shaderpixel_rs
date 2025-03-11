use crate::model::obj::NormalizedObj;
use crate::vulkan::HotShader;

use std::sync::{Arc, RwLock};

use egui::Color32;
use glam::{Mat4, Vec4};

pub struct ArtObject {
    pub name: String,
    pub model: Arc<NormalizedObj>,
    pub matrix: Mat4,
    pub shader_vert: Arc<HotShader>,
    pub shader_frag: Arc<HotShader>,
    pub options: Vec<ArtOption>,
    pub option_values: Option<Arc<RwLock<Vec4>>>,
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
        &self.label
    }
}
