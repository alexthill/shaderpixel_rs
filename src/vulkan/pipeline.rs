use crate::art::{ArtData, ArtObject};
use super::{
    geometry::Geometry,
    helpers::{fs, vs},
    shader::HotShader,
    texture::Texture,
};

use std::sync::Arc;

use anyhow::Context;
use glam::Mat4;
use vulkano::{
    buffer::{
        allocator::SubbufferAllocator,
        Subbuffer,
    },
    device::Device,
    descriptor_set::{
        allocator::StandardDescriptorSetAllocator,
        DescriptorSet, WriteDescriptorSet,
    },
    pipeline::{
        graphics::{
            color_blend::{
                AttachmentBlend, BlendFactor, BlendOp, ColorBlendAttachmentState, ColorBlendState
            },
            depth_stencil::{DepthState, DepthStencilState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::{CullMode, RasterizationState},
            vertex_input::VertexInputState,
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::{RenderPass, Subpass},
    shader::EntryPoint,
};

pub struct MyPipelineCreateInfo {
    pub name: String,
    pub vs: Arc<HotShader>,
    pub fs: Arc<HotShader>,
    pub enable_pipeline: bool,
    pub enable_depth_test: bool,
}

impl Default for MyPipelineCreateInfo {
    fn default() -> Self {
        Self {
            name: Default::default(),
            vs: Default::default(),
            fs: Default::default(),
            enable_pipeline: true,
            enable_depth_test: true,
        }
    }
}

impl From<&ArtObject> for MyPipelineCreateInfo {
    fn from(art_obj: &ArtObject) -> Self {
        Self {
            name: art_obj.name.clone(),
            vs: Arc::clone(&art_obj.shader_vert),
            fs: Arc::clone(&art_obj.shader_frag),
            enable_pipeline: art_obj.enable_pipeline,
            enable_depth_test: art_obj.enable_depth_test,
        }
    }
}

pub struct MyPipeline {
    name: String,
    art_idx: Option<usize>,
    texture: Option<Texture>,
    pipeline: Option<Arc<GraphicsPipeline>>,
    descriptor_sets: Option<Vec<Arc<DescriptorSet>>>,
    geometry: Geometry,
    uniform_buffers_vert: Vec<Subbuffer<vs::UniformBufferObject>>,
    uniform_buffers_frag: Vec<Subbuffer<fs::UniformBufferObject>>,
    vs: Arc<HotShader>,
    fs: Arc<HotShader>,
    pub enable_pipeline: bool,
    enable_depth_test: bool,
}

impl MyPipeline {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        create_info: MyPipelineCreateInfo,
        art_idx: Option<usize>,
        texture: Option<Texture>,
        device: Arc<Device>,
        geometry: Geometry,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
        frames_in_flight: usize,
        uniform_buffer_allocator: &SubbufferAllocator,
        descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    ) -> anyhow::Result<Self> {
        log::debug!("creating pipeline {}", create_info.name);

        create_info.vs.set_device(device.clone());
        create_info.fs.set_device(device.clone());

        let uniform_buffers_vert = (0..frames_in_flight).map(|_| {
            uniform_buffer_allocator.allocate_sized::<vs::UniformBufferObject>().unwrap()
        }).collect::<Vec<_>>();
        let uniform_buffers_frag = (0..frames_in_flight).map(|_| {
            uniform_buffer_allocator.allocate_sized::<fs::UniformBufferObject>().unwrap()
        }).collect::<Vec<_>>();


        let mut pipeline = Self {
            name: create_info.name,
            art_idx,
            texture,
            pipeline: None,
            descriptor_sets: None,
            geometry,
            uniform_buffers_vert,
            uniform_buffers_frag,
            vs: create_info.vs,
            fs: create_info.fs,
            enable_pipeline: create_info.enable_pipeline,
            enable_depth_test: create_info.enable_depth_test,
        };
        pipeline.update_pipeline(
            device,
            render_pass,
            viewport,
            descriptor_set_allocator,
        )?;
        Ok(pipeline)
    }

    #[allow(unused)]
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_pipeline(&self) -> Option<&Arc<GraphicsPipeline>> {
        self.pipeline.as_ref()
    }

    pub fn get_descriptor_sets(&self) -> Option<&[Arc<DescriptorSet>]> {
        self.descriptor_sets.as_deref()
    }

    pub fn get_vertex_buffer(&self) -> &Subbuffer<[u8]> {
        self.geometry.vertex_buffer()
    }

    pub fn get_index_buffer(&self) -> &Subbuffer<[u32]> {
        self.geometry.index_buffer()
    }

    pub fn get_art_idx(&self) -> Option<usize> { self.art_idx }

    pub fn set_shaders(&mut self, vs: Arc<HotShader>, fs: Arc<HotShader>) {
        if !Arc::ptr_eq(&self.vs, &vs) {
            self.vs = vs;
            self.pipeline = None;
        }
        if !Arc::ptr_eq(&self.fs, &fs) {
            self.fs = fs;
            self.pipeline = None;
        }
    }

    pub fn has_changed(&self) -> bool {
        self.vs.has_changed() || self.fs.has_changed()
    }

    pub fn reload_shaders(&mut self, forced: bool) -> bool {
        if self.vs.reload(forced) | self.fs.reload(forced) {
            self.pipeline = None;
            true
        } else {
            false
        }
    }

    pub fn update_uniform_buffer(
        &self,
        idx: usize,
        view: Mat4,
        proj: Mat4,
        time: f32,
        data: Option<ArtData>,
    ) -> anyhow::Result<()> {
        let model = data.map(|data| data.matrix).unwrap_or(Mat4::IDENTITY);
        *self.uniform_buffers_vert[idx].write()? = vs::UniformBufferObject {
            model: model.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
        };

        if let Some(data) = data {
            *self.uniform_buffers_frag[idx].write()? = fs::UniformBufferObject {
                light_pos: data.light_pos.to_array(),
                options: data.option_values.to_array(),
                time,
            };
        }

        Ok(())
    }

    pub fn update_pipeline(
        &mut self,
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
        descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    ) -> anyhow::Result<()> {
        if !self.enable_pipeline {
            return Ok(());
        }

        let vs_module = self.vs.get_module()?;
        let fs_module = self.fs.get_module()?;

        if let (Some(vs), Some(fs)) = (vs_module, fs_module) {
            log::debug!("updating pipeline {}", self.name);
            let vs_entry = vs.entry_point("main").ok_or_else(|| anyhow::anyhow!("no entrypoint"))?;
            let fs_entry = fs.entry_point("main").ok_or_else(|| anyhow::anyhow!("no entrypoint"))?;
            let pipeline = Self::create_pipeline(
                device,
                self.geometry.definition(&vs_entry)?,
                vs_entry,
                fs_entry,
                render_pass,
                viewport,
                self.enable_depth_test,
            )?;
            self.pipeline = Some(pipeline);
            self.update_descriptor_sets(descriptor_set_allocator)
                .context("failed to update descriptor_sets")?;
        } else {
            self.vs.reload(false);
            self.fs.reload(false);
        }

        Ok(())
    }

    fn update_descriptor_sets(
        &mut self,
        descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    ) -> anyhow::Result<()> {
        // sanity check
        debug_assert_eq!(self.uniform_buffers_vert.len(), self.uniform_buffers_frag.len());

        let Some(pipeline) = self.pipeline.as_ref() else {
            return Ok(());
        };
        let layout = &pipeline.layout().set_layouts()[0];
        let mut descriptor_sets = Vec::with_capacity(self.uniform_buffers_vert.len());

        // A for loop is nicer than zipping iterators together.
        #[allow(clippy::needless_range_loop)]
        for i in 0..self.uniform_buffers_vert.len() {
            let mut write_sets = vec![
                WriteDescriptorSet::buffer(0, self.uniform_buffers_vert[i].clone()),
                WriteDescriptorSet::buffer(1, self.uniform_buffers_frag[i].clone()),
            ];
            if let Some(Texture { view, sampler }) = self.texture.as_ref() {
                write_sets.push(
                    WriteDescriptorSet::image_view_sampler(2, view.clone(), sampler.clone()),
                );
            }
            let write_sets = write_sets
                .into_iter()
                .filter(|set| {
                    pipeline
                        .descriptor_binding_requirements()
                        .contains_key(&(0, set.binding()))
                })
                .collect::<Vec<_>>();
            descriptor_sets.push(DescriptorSet::new(
                descriptor_set_allocator.clone(),
                layout.clone(),
                write_sets,
                [],
            )?);
        }
        self.descriptor_sets = Some(descriptor_sets);
        Ok(())
    }

    fn create_pipeline(
        device: Arc<Device>,
        vertex_input_state: VertexInputState,
        vs_entry: EntryPoint,
        fs_entry: EntryPoint,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
        enable_depth_test: bool,
    ) -> anyhow::Result<Arc<GraphicsPipeline>> {
        let stages = [
            PipelineShaderStageCreateInfo::new(vs_entry),
            PipelineShaderStageCreateInfo::new(fs_entry),
        ];

        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        )
        .unwrap();

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

        let depth = if enable_depth_test {
            Some(DepthState::simple())
        } else {
            None
        };
        let pipeline = GraphicsPipeline::new(
            device.clone(),
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState {
                    viewports: [viewport].into_iter().collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(RasterizationState {
                    cull_mode: CullMode::Back,
                    ..Default::default()
                }),
                multisample_state: Some(MultisampleState {
                    rasterization_samples: render_pass.attachments()[0].samples,
                    ..Default::default()
                }),
                depth_stencil_state: Some(DepthStencilState {
                    depth,
                    ..Default::default()
                }),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState {
                        blend: Some(AttachmentBlend {
                            src_color_blend_factor: BlendFactor::SrcAlpha,
                            dst_color_blend_factor: BlendFactor::OneMinusSrcAlpha,
                            color_blend_op: BlendOp::Add,
                            src_alpha_blend_factor: BlendFactor::One,
                            dst_alpha_blend_factor: BlendFactor::Zero,
                            alpha_blend_op: BlendOp::Add,
                        }),
                        ..Default::default()
                    },
                )),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )?;
        Ok(pipeline)
    }
}
