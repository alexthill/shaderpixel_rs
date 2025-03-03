use super::{
    shader::HotShader,
    vertex::VertexPos,
};

use std::sync::Arc;

use vulkano::{
    buffer::Subbuffer,
    device::Device,
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

pub struct MyPipeline {
    name: String,
    pipeline: Option<Arc<GraphicsPipeline>>,
    vertex_buffer: Subbuffer<[VertexPos]>,
    index_buffer: Subbuffer<[u32]>,
    vs: Arc<HotShader>,
    fs: Arc<HotShader>,
}

impl MyPipeline {
    pub fn new(
        name: String,
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
            pipeline,
            vertex_buffer,
            index_buffer,
            vs,
            fs,
        })
    }

    pub fn get_pipeline(&self) -> Option<&Arc<GraphicsPipeline>> {
        self.pipeline.as_ref()
    }

    pub fn get_vertex_buffer(&self) -> &Subbuffer<[VertexPos]> {
        &self.vertex_buffer
    }

    pub fn get_index_buffer(&self) -> &Subbuffer<[u32]> {
        &self.index_buffer
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
