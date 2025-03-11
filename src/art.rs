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
    Slider { value: f32, min: f32, max: f32 },
    Stroke { width: f32, color: Color32 },
}

impl ArtOptionType {
    pub fn save_value(&self, values: &mut [f32], i: &mut usize) {
        match self {
            Self::Slider { value, .. } => {
                values[*i] = *value;
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
    pub fn slider(label: &'static str, value: f32, min: f32, max: f32) -> Self {
        Self {
            label,
            ty: ArtOptionType::Slider { value, min, max },
        }
    }

    pub fn stroke(label: &'static str, width: f32, color: Color32) -> Self {
        Self {
            label,
            ty: ArtOptionType::Stroke { width, color },
        }
    }

    pub fn label(&self) -> &str {
        &self.label
    }
}
