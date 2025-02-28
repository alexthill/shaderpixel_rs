use vulkano::{
    buffer::BufferContents,
    pipeline::graphics::vertex_input::Vertex,
};

pub trait MyVertexTrait {
    fn new(pos: [f32; 3], norm: [f32; 3], coords: [f32; 2]) -> Self;
}

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct VertexPos {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
}

impl MyVertexTrait for VertexPos {
    fn new(pos: [f32; 3], _: [f32; 3], _: [f32; 2]) -> Self {
        Self { position: pos }
    }
}
