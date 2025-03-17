use crate::{
    art::{ArtData, ArtObject},
    model::obj::NormalizedObj,
};
use super::{
    debug::*,
    helpers::*,
    geometry::Geometry,
    pipeline::{MyPipeline, MyPipelineCreateInfo},
    shader::{watch_shaders, HotShader},
    texture::Texture,
    vertex::VertexType,
};

use std::cmp::Ordering;
use std::sync::Arc;

use anyhow::Context;
use egui_winit_vulkano::Gui;
use glam::{Mat4, Vec3};
use shaderc::ShaderKind;
use vulkano::{
    buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo},
    buffer::BufferUsage,
    command_buffer::allocator::{StandardCommandBufferAllocator, StandardCommandBufferAllocatorCreateInfo},
    command_buffer::SecondaryAutoCommandBuffer,
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures, Queue, QueueCreateInfo},
    format::Format,
    image::{ImageUsage, SampleCount},
    instance::debug::DebugUtilsMessenger,
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
    memory::allocator::{MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, RenderPass, Subpass},
    swapchain::{
        self,
        PresentMode, Surface, SurfaceInfo, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo,
    },
    sync::{
        self,
        future::FenceSignalFuture,
        GpuFuture,
    },
    Validated, VulkanError,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

const PREFFERED_IMAGE_COUNT: u32 = 3;

pub struct App {
    pub view_matrix: Mat4,

    #[allow(dead_code)]
    instance: Arc<Instance>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    msaa_sample_count: SampleCount,
    memory_allocator: Arc<StandardMemoryAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    depth_format: Format,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    viewport: Viewport,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    command_buffers: Vec<Arc<SecondaryAutoCommandBuffer>>,
    #[allow(clippy::type_complexity)]
    fences: Vec<Option<Arc<FenceSignalFuture<Box<dyn GpuFuture>>>>>,
    previous_fence_i: usize,
    pipelines: Vec<MyPipeline>,
    pipeline_order: Vec<usize>,

    // If this falls out of scope then there will be no more debug events.
    // Put it at the end so that it gets dropped last.
    _debug: Option<DebugUtilsMessenger>,

}

impl App {
    pub fn new(
        window: Arc<Window>,
        model: NormalizedObj,
        art_objs: &[ArtObject],
    ) -> Self {
        log::debug!("creating vulkan app");

        let dimensions = window.inner_size();
        let library = vulkano::VulkanLibrary::new()
            .expect("no local Vulkan library/DLL");

        let (debug_extensions, debug_layers) = get_debug_extensions_and_layers();
        for layer in debug_layers.iter() {
            if !check_layer_support(&library, layer).unwrap() {
                panic!("Layer {layer} is not supported");
            }
        }
        let required_extensions = Surface::required_extensions(window.as_ref())
            .expect("failed to get required extensions");
        let enabled_extensions = required_extensions.union(&debug_extensions);

        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
                enabled_layers: debug_layers,
                enabled_extensions,
                ..Default::default()
            },
        )
        .expect("failed to create instance");

        let debug = setup_debug_callback(Arc::clone(&instance))
            .expect("failed to setup debug callback");

        let surface = Surface::from_window(instance.clone(), window).unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };
        let device_features = DeviceFeatures {
            geometry_shader: true,
            ..DeviceFeatures::empty()
        };

        let (physical_device, queue_family_index) =
            select_physical_device(&instance, &surface, &device_extensions);
        if !physical_device.supported_features().contains(&device_features) {
            panic!("the physical device does not support all required features");
        }

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                enabled_features: device_features,
                ..Default::default()
            },
        )
        .expect("failed to create device");

        let queue = queues.next().unwrap();

        let (swapchain, images) = {
            let caps = physical_device
                .surface_capabilities(&surface, Default::default())
                .expect("failed to get surface capabilities");

            let composite_alpha = caps.supported_composite_alpha.into_iter().next().unwrap();
            let image_format = physical_device
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0;
            let min_image_count = PREFFERED_IMAGE_COUNT
                .min(caps.max_image_count.unwrap_or(u32::MAX))
                .max(caps.min_image_count);

            Swapchain::new(
                device.clone(),
                surface,
                SwapchainCreateInfo {
                    min_image_count,
                    image_format,
                    image_extent: dimensions.into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
                    composite_alpha,
                    present_mode: PresentMode::Fifo,
                    ..Default::default()
                },
            )
            .unwrap()
        };
        let frames_in_flight = images.len();

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let msaa_sample_count = select_msaa_sample_count(&physical_device);
        log::debug!("selected msaa sample count: {msaa_sample_count:?}");
        let depth_format = find_depth_format(&physical_device)
            .expect("failed to find a supported depth format");
        log::debug!("selected depth format: {depth_format:?}");

        let render_pass = get_render_pass(
            device.clone(),
            swapchain.clone(),
            depth_format,
            msaa_sample_count,
        );
        let framebuffers = get_framebuffers(
            &images,
            depth_format,
            render_pass.clone(),
            memory_allocator.clone(),
            msaa_sample_count,
        );

        let vs = vs::load(device.clone()).expect("failed to create shader module");
        let fs = fs::load(device.clone()).expect("failed to create shader module");

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: dimensions.into(),
            depth_range: 0.0..=1.0,
        };

        let descriptor_set_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default(),
        ));

        let uniform_buffer_allocator = SubbufferAllocator::new(
            memory_allocator.clone(),
            SubbufferAllocatorCreateInfo {
                buffer_usage: BufferUsage::UNIFORM_BUFFER,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
        );

        let command_buffer_allocator = Arc::new(StandardCommandBufferAllocator::new(
            device.clone(),
            StandardCommandBufferAllocatorCreateInfo {
                secondary_buffer_count: 32,
                ..Default::default()
            },
        ));

        let geometry = Geometry::from_model(
            &model,
            VertexType::VertexNorm,
            memory_allocator.clone(),
            Vec3::splat(1.),
        ).expect("failed to parse model");
        let pipeline_main = MyPipeline::new(
            MyPipelineCreateInfo {
                name: "main".to_owned(),
                vs: Arc::new(HotShader::new_nonhot(vs, ShaderKind::Vertex)),
                fs: Arc::new(HotShader::new_nonhot(fs, ShaderKind::Fragment)),
                ..Default::default()
            },
            None,
            None,
            device.clone(),
            geometry,
            render_pass.clone(),
            viewport.clone(),
            frames_in_flight,
            &uniform_buffer_allocator,
            descriptor_set_allocator.clone(),
        ).unwrap();

        let shader_iter = art_objs.iter().flat_map(|art_obj| {
            [art_obj.shader_vert.clone(), art_obj.shader_frag.clone()]
        });
        watch_shaders(shader_iter);

        let mut pipelines = vec![pipeline_main];
        for (art_idx, art_obj) in art_objs.iter().enumerate() {
            let geometry = Geometry::from_model(
                &art_obj.model,
                VertexType::VertexPos,
                memory_allocator.clone(),
                art_obj.container_scale,
            ).expect("failed to parse model");
            let texture = if let Some(path) = art_obj.texture.as_ref() {
                let texture = Texture::new(
                    path,
                    device.clone(),
                    queue.clone(),
                    command_buffer_allocator.clone(),
                    memory_allocator.clone(),
                );
                match texture {
                    Ok(texture) => Some(texture),
                    Err(err) => {
                        log::error!("failed to load texture: {err:?}");
                        None
                    }
                }
            } else {
                None
            };
            let pipeline = MyPipeline::new(
                art_obj.into(),
                Some(art_idx),
                texture,
                device.clone(),
                geometry,
                render_pass.clone(),
                viewport.clone(),
                frames_in_flight,
                &uniform_buffer_allocator,
                descriptor_set_allocator.clone(),
            ).unwrap();
            pipelines.push(pipeline);
        }
        let pipeline_order = Self::get_pipeline_order(&pipelines, art_objs);

        let command_buffers = get_command_buffers(
            frames_in_flight,
            &command_buffer_allocator,
            &queue,
            &pipelines,
            &pipeline_order,
            render_pass.clone(),
        );

        Self {
            view_matrix: Mat4::IDENTITY,
            instance,
            device,
            queue,
            swapchain,
            msaa_sample_count,
            memory_allocator,
            descriptor_set_allocator,
            depth_format,
            render_pass,
            framebuffers,
            viewport,
            command_buffer_allocator,
            command_buffers,
            fences: vec![None; frames_in_flight],
            previous_fence_i: 0,
            pipelines,
            pipeline_order,
            _debug: debug,
        }
    }

    pub fn get_queue(&self) -> &Arc<Queue> { &self.queue }

    pub fn get_swapchain(&self) -> &Arc<Swapchain> { &self.swapchain }

    pub fn get_surface_present_modes(&self) -> Result<Vec<PresentMode>, Validated<VulkanError>> {
        self.device.physical_device().surface_present_modes(
            self.swapchain.surface(),
            SurfaceInfo::default(),
        )
    }

    pub fn gui_pass(&self) -> Subpass {
        Subpass::from(self.render_pass.clone(), 1).unwrap()
    }

    pub fn recreate_swapchain(
        &mut self,
        dimensions: PhysicalSize<u32>,
        options: &crate::gui::Options,
    ) -> anyhow::Result<()> {
        let (new_swapchain, new_images) = self.swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: dimensions.into(),
                present_mode: options.present_mode,
                ..self.swapchain.create_info()
            })
            .context("failed to recreate swapchain")?;

        self.swapchain = new_swapchain;
        self.framebuffers = get_framebuffers(
            &new_images,
            self.depth_format,
            self.render_pass.clone(),
            self.memory_allocator.clone(),
            self.msaa_sample_count,
        );

        self.viewport.extent = dimensions.into();
        for pipeline in self.pipelines.iter_mut() {
            pipeline.update_pipeline(
                self.device.clone(),
                self.render_pass.clone(),
                self.viewport.clone(),
                self.descriptor_set_allocator.clone(),
            ).context("failed to update pipeline")?;
        }
        self.update_command_buffers();

        Ok(())
    }

    /// Draws the render_pass and returns whether the swapchain is dirty.
    pub fn draw(
        &mut self,
        time: f32,
        gui: Option<&mut Gui>,
        art_objs: &[ArtObject],
    ) -> anyhow::Result<bool> {
        let mut pipeline_changed = false;
        for pipeline in self.pipelines[1..].iter_mut() {
            if pipeline.has_changed() {
                pipeline_changed = true;
                pipeline.reload_shaders(false);
            } else if pipeline.get_pipeline().is_none() {
                pipeline.update_pipeline(
                    self.device.clone(),
                    self.render_pass.clone(),
                    self.viewport.clone(),
                    self.descriptor_set_allocator.clone(),
                ).context("failed to update pipeline")?;
                pipeline_changed |= pipeline.get_pipeline().is_some();
            }
        }

        let new_order = Self::get_pipeline_order(&self.pipelines, art_objs);
        if new_order != self.pipeline_order {
            self.pipeline_order = new_order;
            pipeline_changed = true;
        }

        for (pipeline, art_obj) in self.pipelines.iter_mut().filter_map(|pip| {
            pip.get_art_idx().map(|idx| (pip, &art_objs[idx]))
        }) {
            if art_obj.enable_pipeline != pipeline.enable_pipeline {
                pipeline.enable_pipeline = art_obj.enable_pipeline;
                pipeline_changed = true;
            }
        }

        if pipeline_changed {
            self.update_command_buffers();
        }

        let (image_i, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None)
                .map_err(Validated::unwrap)
            {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    return Ok(true);
                }
                Err(e) => panic!("failed to acquire next image: {e}"),
            };
        let image_i = image_i as usize;

        let mut swapchain_dirty = suboptimal;

        // wait for the fence related to this image to finish
        // (normally this would be the oldest fence)
        if let Some(image_fence) = &self.fences[image_i] {
            image_fence.wait(None).unwrap();
        }

        let previous_future = match self.fences[self.previous_fence_i].clone() {
            None => {
                let mut now = sync::now(self.device.clone());
                now.cleanup_finished();
                now.boxed()
            }
            Some(fence) => fence.boxed(),
        };

        self.update_uniform_buffer(image_i, time, art_objs);

        let mut subpasses = vec![self.command_buffers[image_i].clone()];
        if let Some(gui) = gui {
            subpasses.push(gui.draw_on_subpass_image(self.swapchain.image_extent()));
        }
        let command_buffer = get_primary_command_buffer(
            &self.command_buffer_allocator,
            &self.queue,
            self.framebuffers[image_i].clone(),
            subpasses,
        )?;

        let future = previous_future
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i as u32),
            )
            .boxed()
            .then_signal_fence_and_flush();

        self.fences[image_i] = match future.map_err(Validated::unwrap) {
            // We need to call .boxed() on the future at some point to get a dyn GpuFuture.
            // To do this it needs to be wrapped in an Arc, even if it is not send/sync.
            #[allow(clippy::arc_with_non_send_sync)]
            Ok(value) => Some(Arc::new(value)),
            Err(VulkanError::OutOfDate) => {
                swapchain_dirty = true;
                None
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                None
            }
        };

        self.previous_fence_i = image_i;
        Ok(swapchain_dirty)
    }

    fn get_pipeline_order(pipelines: &[MyPipeline], art_objs: &[ArtObject]) -> Vec<usize> {
        let mut pipeline_order = (0..pipelines.len()).collect::<Vec<_>>();
        pipeline_order.sort_unstable_by(|&a, &b| {
            match (pipelines[a].get_art_idx(), pipelines[b].get_art_idx()) {
                (Some(idx_a), Some(idx_b)) => {
                    let a = &art_objs[idx_a];
                    let b = &art_objs[idx_b];
                    a.data.dist_to_camera_sqr.total_cmp(&b.data.dist_to_camera_sqr).reverse()
                }
                (Some(_), None) => return Ordering::Greater,
                (None, Some(_)) => return Ordering::Less,
                (None, None) => return Ordering::Equal,
            }
        });
        pipeline_order
    }

    fn update_uniform_buffer(&self, image_idx: usize, time: f32, art_objs: &[ArtObject]) {
        let aspect_ratio = self.swapchain.image_extent()[0] as f32
            / self.swapchain.image_extent()[1] as f32;
        let proj = Mat4::perspective_rh(
            75_f32.to_radians(),
            aspect_ratio,
            0.01,
            200.0,
        );
        for pipeline in self.pipelines.iter() {
            let data = pipeline.get_art_idx().map(|idx| art_objs[idx].data).unwrap_or_else(|| {
                ArtData {
                    dist_to_camera_sqr: f32::MAX,
                    matrix: Mat4::IDENTITY,
                    light_pos: art_objs[0].data.light_pos,
                    option_values: Default::default(),
                }
            });
            let data = Some(data);
            let res = pipeline.update_uniform_buffer(image_idx, self.view_matrix, proj, time, data);
            if let Err(err) = res {
                log::error!("failed to update uniforms: {err:?}");
            }
        }
    }

    fn update_command_buffers(&mut self) {
        self.command_buffers = get_command_buffers(
            self.fences.len(),
            &self.command_buffer_allocator,
            &self.queue,
            &self.pipelines,
            &self.pipeline_order,
            Arc::clone(&self.render_pass),
        );
    }
}
