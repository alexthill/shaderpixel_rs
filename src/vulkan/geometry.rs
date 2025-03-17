use crate::model::obj::NormalizedObj;
use super::vertex::*;

use std::sync::Arc;

use glam::Vec3;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::graphics::vertex_input::{Vertex, VertexDefinition, VertexInputState},
    shader::EntryPoint,
    ValidationError,
};

pub struct Geometry {
    vertex_type: VertexType,
    vertex_buffer: Subbuffer<[u8]>,
    index_buffer: Subbuffer<[u32]>,
    _extent_min: Vec3,
    _extent_max: Vec3,
}

impl Geometry {
    pub fn from_model(
        model: &NormalizedObj,
        vertex_type: VertexType,
        memory_allocator: Arc<StandardMemoryAllocator>,
        scale: Vec3,
    ) -> anyhow::Result<Self> {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for vertex in &model.vertices {
            for (i, &coord) in vertex.pos_coords.iter().enumerate() {
                min[i] = min[i].min(coord);
                max[i] = max[i].max(coord);
            }
        }

        let (vertex_buffer, index_buffer) = match vertex_type {
            VertexType::VertexPos => {
                let vertices = model.vertices.iter().copied().map(|mut vertex| {
                    vertex.pos_coords = (scale * Vec3::from(vertex.pos_coords)).into();
                    VertexPos::new(vertex.pos_coords, vertex.tex_coords, vertex.normal)
                }).collect::<Vec<_>>();
                let (vb, ib) = Self::model_to_buffers(&vertices, &model.indices, memory_allocator)?;
                (vb.into_bytes(), ib)
            }
            VertexType::VertexNorm => {
                let vertices = model.vertices.iter().map(|vertex| {
                    VertexNorm::new(vertex.pos_coords, vertex.tex_coords, vertex.normal)
                }).collect::<Vec<_>>();
                let (vb, ib) = Self::model_to_buffers(&vertices, &model.indices, memory_allocator)?;
                (vb.into_bytes(), ib)
            }
        };

        Ok(Self {
            vertex_type,
            vertex_buffer,
            index_buffer,
            _extent_min: min,
            _extent_max: max,
        })
    }

    pub fn vertex_buffer(&self) -> &Subbuffer<[u8]> {
        &self.vertex_buffer
    }

    pub fn index_buffer(&self) -> &Subbuffer<[u32]> {
        &self.index_buffer
    }

    pub fn definition(&self, entry: &EntryPoint) -> Result<VertexInputState, Box<ValidationError>> {
        match self.vertex_type {
            VertexType::VertexPos => VertexPos::per_vertex().definition(entry),
            VertexType::VertexNorm => VertexNorm::per_vertex().definition(entry),
        }
    }

    #[allow(clippy::type_complexity)]
    fn model_to_buffers<V: MyVertexTrait + Copy>(
        vertices: &[V],
        indices: &[u32],
        memory_allocator: Arc<StandardMemoryAllocator>,
    ) -> anyhow::Result<(Subbuffer<[V]>, Subbuffer<[u32]>)> {
        let vertex_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vertices.iter().copied(),
        )?;

        let index_buffer = Buffer::from_iter(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::INDEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            indices.iter().copied(),
        )?;

        Ok((vertex_buffer, index_buffer))
    }
}
