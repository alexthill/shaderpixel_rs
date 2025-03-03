use crate::model::obj::NormalizedObj;
use crate::vulkan::HotShader;

use std::sync::Arc;

use glam::Mat4;

pub struct ArtObject {
    pub name: String,
    pub model: Arc<NormalizedObj>,
    pub matrix: Mat4,
    pub shader_vert: Arc<HotShader>,
    pub shader_frag: Arc<HotShader>,
}
