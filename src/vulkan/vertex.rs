use vulkano::{
    buffer::BufferContents,
    pipeline::graphics::vertex_input::Vertex,
};

pub trait MyVertexTrait: BufferContents + Vertex {
    fn new(position: [f32; 3], coords: [f32; 2], normal: [f32; 3]) -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexType {
    #[allow(unused)]
    VertexPos,
    VertexNorm,
}

#[derive(Debug, Default, Clone, Copy, BufferContents, Vertex)]
#[repr(C)]
pub struct VertexPos {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
}

impl MyVertexTrait for VertexPos {
    fn new(position: [f32; 3], _: [f32; 2], _: [f32; 3]) -> Self {
        Self { position }
    }
}

#[derive(Debug, Default, Clone, Copy, BufferContents, Vertex)]
#[repr(C)]
pub struct VertexNorm {
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],
    #[format(R32G32B32_SFLOAT)]
    pub normal: [f32; 3],
}

impl MyVertexTrait for VertexNorm {
    fn new(position: [f32; 3], _: [f32; 2], normal: [f32; 3]) -> Self {
        Self { position, normal }
    }
}
