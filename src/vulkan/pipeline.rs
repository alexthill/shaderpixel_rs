use super::{
    helpers::vs,
    shader::HotShader,
    vertex::VertexPos,
};

use std::sync::Arc;

use glam::Mat4;
use vulkano::{
    buffer::Subbuffer,
    device::Device,
    descriptor_set::DescriptorSet,
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            depth_stencil::{DepthState, DepthStencilState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::{CullMode, RasterizationState},
            vertex_input::{Vertex, VertexDefinition},
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::{RenderPass, Subpass},
    shader::ShaderModule,
};

pub struct DescriptorData {
    pub descriptor_sets: Vec<Arc<DescriptorSet>>,
    pub uniform_buffers: Vec<Subbuffer<vs::UniformBufferObject>>,
}

pub struct MyPipeline {
    name: String,
    model_matrix: Mat4,
    pipeline: Option<Arc<GraphicsPipeline>>,
    descriptor_data: Option<DescriptorData>,
    vertex_buffer: Subbuffer<[VertexPos]>,
    index_buffer: Subbuffer<[u32]>,
    vs: Arc<HotShader>,
    fs: Arc<HotShader>,
}

impl MyPipeline {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        model_matrix: Mat4,
        device: Arc<Device>,
        vertex_buffer: Subbuffer<[VertexPos]>,
        index_buffer: Subbuffer<[u32]>,
        vs: Arc<HotShader>,
        fs: Arc<HotShader>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> anyhow::Result<Self> {
        log::debug!("creating pipeline {name}");

        vs.set_device(device.clone());
        fs.set_device(device.clone());
        let vs_module = vs.get_module()?;
        let fs_module = fs.get_module()?;

        let pipeline = if let (Some(vs), Some(fs)) = (vs_module, fs_module) {
            Some(Self::create_pipeline(
                device,
                vs.clone(),
                fs.clone(),
                render_pass,
                viewport,
            )?)
        } else {
            vs.reload(false);
            fs.reload(false);
            None
        };

        Ok(Self {
            name,
            model_matrix,
            pipeline,
            descriptor_data: None,
            vertex_buffer,
            index_buffer,
            vs,
            fs,
        })
    }

    pub fn get_pipeline(&self) -> Option<&Arc<GraphicsPipeline>> {
        self.pipeline.as_ref()
    }

    pub fn get_descriptor_sets(&self) -> Option<&[Arc<DescriptorSet>]> {
        self.descriptor_data.as_ref().map(|data| &*data.descriptor_sets)
    }

    pub fn set_descriptor_data(&mut self, data: DescriptorData) {
        self.descriptor_data = Some(data);
    }

    pub fn get_vertex_buffer(&self) -> &Subbuffer<[VertexPos]> {
        &self.vertex_buffer
    }

    pub fn get_index_buffer(&self) -> &Subbuffer<[u32]> {
        &self.index_buffer
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
    ) -> anyhow::Result<()> {
        let Some(data) = self.descriptor_data.as_ref() else {
            return Err(anyhow::anyhow!("called update_uniforms on pipeline without data"));
        };
        *data.uniform_buffers[idx].write()? = vs::UniformBufferObject {
            model: self.model_matrix.to_cols_array_2d(),
            view: view.to_cols_array_2d(),
            proj: proj.to_cols_array_2d(),
        };
        Ok(())
    }

    pub fn update_pipeline(
        &mut self,
        device: Arc<Device>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> anyhow::Result<()> {
        let vs_module = self.vs.get_module()?;
        let fs_module = self.fs.get_module()?;

        self.pipeline = if let (Some(vs), Some(fs)) = (vs_module, fs_module) {
            log::debug!("updating pipeline {}", self.name);
            Some(Self::create_pipeline(
                device,
                vs.clone(),
                fs.clone(),
                render_pass,
                viewport
            )?)
        } else {
            self.vs.reload(false);
            self.fs.reload(false);
            None
        };

        Ok(())
    }

    fn create_pipeline(
        device: Arc<Device>,
        vs: Arc<ShaderModule>,
        fs: Arc<ShaderModule>,
        render_pass: Arc<RenderPass>,
        viewport: Viewport,
    ) -> anyhow::Result<Arc<GraphicsPipeline>> {
        let vs = vs.entry_point("main").unwrap();
        let fs = fs.entry_point("main").unwrap();

        let vertex_input_state = VertexPos::per_vertex().definition(&vs).unwrap();

        let stages = [
            PipelineShaderStageCreateInfo::new(vs),
            PipelineShaderStageCreateInfo::new(fs),
        ];

        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        )
        .unwrap();

        let subpass = Subpass::from(render_pass.clone(), 0).unwrap();

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
                multisample_state: Some(MultisampleState::default()),
                depth_stencil_state: Some(DepthStencilState {
                    depth: Some(DepthState::simple()),
                    ..Default::default()
                }),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )?;
        Ok(pipeline)
    }
}
