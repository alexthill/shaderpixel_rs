use crate::{
    art::ArtObject,
    model::obj::NormalizedObj,
};
use super::{
    debug::*,
    helpers::*,
    pipeline::MyPipeline,
    shader::{watch_shaders, HotShader},
    vertex::MyVertexTrait,
};

use std::sync::Arc;

use anyhow::Context;
use glam::Mat4;
use shaderc::ShaderKind;
use vulkano::{
    buffer::allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo},
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::allocator::StandardCommandBufferAllocator,
    command_buffer::{CommandBufferExecFuture, PrimaryAutoCommandBuffer},
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    device::{Device, DeviceCreateInfo, DeviceExtensions, DeviceFeatures, Queue, QueueCreateInfo},
    format::Format,
    image::ImageUsage,
    instance::debug::DebugUtilsMessenger,
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::graphics::viewport::Viewport,
    render_pass::{Framebuffer, RenderPass},
    swapchain::{
        self,
        PresentFuture, PresentMode, Surface, Swapchain, SwapchainAcquireFuture,
        SwapchainCreateInfo, SwapchainPresentInfo,
    },
    sync::{
        self,
        future::{FenceSignalFuture, JoinFuture},
        GpuFuture,
    },
    Validated, VulkanError,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

type ComplexFenceFutureType = PresentFuture<CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>>;

pub struct App {
    pub view_matrix: Mat4,

    #[allow(dead_code)]
    instance: Arc<Instance>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
    depth_format: Format,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    viewport: Viewport,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
    fences: Vec<Option<Arc<FenceSignalFuture<ComplexFenceFutureType>>>>,
    previous_fence_i: usize,
    pipelines: Vec<MyPipeline>,
    uniform_buffers_frag: Vec<Subbuffer<fs::UniformBufferObject>>,

    // If this falls out of scope then there will be no more debug events.
    // Put it at the end so that it gets dropped last.
    #[allow(dead_code)]
    debug: Option<DebugUtilsMessenger>,

}

impl App {
    pub fn new(
        window: Arc<Window>,
        model: NormalizedObj,
        art_objs: Vec<ArtObject>,
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
            let min_image_count = 3
                .min(caps.max_image_count.unwrap_or(u32::MAX))
                .max(caps.min_image_count);

            Swapchain::new(
                device.clone(),
                surface,
                SwapchainCreateInfo {
                    min_image_count,
                    image_format,
                    image_extent: dimensions.into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha,
                    present_mode: PresentMode::Fifo,
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let depth_format = find_depth_format(&physical_device)
            .expect("failed to find a supported depth format");
        log::debug!("selected depth format: {depth_format:?}");
        let render_pass = get_render_pass(device.clone(), swapchain.clone(), depth_format);
        let depth_buffer = create_depth_buffer(
            memory_allocator.clone(),
            images[0].extent(),
            depth_format,
        );
        let framebuffers = get_framebuffers(&images, &depth_buffer, render_pass.clone());

        let (vertices, indices, _) = load_model(&model);
        let (vertex_buffer, index_buffer) = model_to_buffers(
            &vertices,
            indices,
            memory_allocator.clone(),
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

        let uniform_buffers_frag = (0..images.len()).map(|_| {
            uniform_buffer_allocator.allocate_sized::<fs::UniformBufferObject>().unwrap()
        }).collect::<Vec<_>>();
        let uniform_buffers = (0..images.len()).map(|_| {
            uniform_buffer_allocator.allocate_sized::<vs::UniformBufferObject>().unwrap()
        }).collect::<Vec<_>>();
        let pipeline_main = MyPipeline::new(
            "main".to_owned(),
            Mat4::IDENTITY,
            device.clone(),
            vertex_buffer,
            index_buffer,
            uniform_buffers,
            Arc::new(HotShader::new_nonhot(vs, ShaderKind::Vertex)),
            Arc::new(HotShader::new_nonhot(fs, ShaderKind::Fragment)),
            render_pass.clone(),
            viewport.clone(),
            &uniform_buffers_frag,
            descriptor_set_allocator.clone(),
        ).unwrap();

        let shader_iter = art_objs.iter().flat_map(|art_obj| {
            [art_obj.shader_vert.clone(), art_obj.shader_frag.clone()]
        });
        watch_shaders(shader_iter);

        let mut pipelines = vec![pipeline_main];
        for art_obj in art_objs {
            let (vertices, indices, _) = load_model(&art_obj.model);
            let (vertex_buffer, index_buffer) = model_to_buffers(
                &vertices,
                indices,
                memory_allocator.clone(),
            );
            let uniform_buffers = (0..images.len()).map(|_| {
                uniform_buffer_allocator.allocate_sized::<vs::UniformBufferObject>().unwrap()
            }).collect::<Vec<_>>();
            let pipeline = MyPipeline::new(
                art_obj.name,
                art_obj.matrix,
                device.clone(),
                vertex_buffer,
                index_buffer,
                uniform_buffers,
                art_obj.shader_vert,
                art_obj.shader_frag,
                render_pass.clone(),
                viewport.clone(),
                &uniform_buffers_frag,
                descriptor_set_allocator.clone(),
            ).unwrap();
            pipelines.push(pipeline);
        }

        let command_buffer_allocator =
            Arc::new(StandardCommandBufferAllocator::new(device.clone(), Default::default()));

        let command_buffers = get_command_buffers(
            &command_buffer_allocator,
            &queue,
            &pipelines,
            &framebuffers,
        );

        let frames_in_flight = images.len();
        let fences: Vec<Option<Arc<FenceSignalFuture<_>>>> = vec![None; frames_in_flight];

        Self {
            view_matrix: Mat4::IDENTITY,
            instance,
            device,
            queue,
            swapchain,
            memory_allocator,
            descriptor_set_allocator,
            depth_format,
            render_pass,
            framebuffers,
            viewport,
            command_buffer_allocator,
            command_buffers,
            fences,
            previous_fence_i: 0,
            pipelines,
            uniform_buffers_frag,
            debug,
        }
    }

    pub fn recreate_swapchain(&mut self, dimensions: PhysicalSize<u32>) {
        let (new_swapchain, new_images) = self.swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: dimensions.into(),
                ..self.swapchain.create_info()
            })
            .expect("failed to recreate swapchain");
        let depth_buffer = create_depth_buffer(
            self.memory_allocator.clone(),
            new_images[0].extent(),
            self.depth_format,
        );

        self.swapchain = new_swapchain;
        self.framebuffers = get_framebuffers(
            &new_images,
            &depth_buffer,
            self.render_pass.clone(),
        );

        self.viewport.extent = dimensions.into();
        for pipeline in self.pipelines.iter_mut() {
            pipeline.update_pipeline(
                self.device.clone(),
                self.render_pass.clone(),
                self.viewport.clone(),
                &self.uniform_buffers_frag,
                self.descriptor_set_allocator.clone(),
            ).unwrap();
        }
        self.command_buffers = get_command_buffers(
            &self.command_buffer_allocator,
            &self.queue,
            &self.pipelines,
            &self.framebuffers,
        );
    }

    pub fn draw(&mut self, time: f32) -> anyhow::Result<bool> {
        let mut pipeline_changed = false;
        for pipeline in self.pipelines[1..].iter_mut() {
            if pipeline.has_changed() {
                pipeline_changed = true;
            } else if pipeline.get_pipeline().is_none() {
                pipeline.update_pipeline(
                    self.device.clone(),
                    self.render_pass.clone(),
                    self.viewport.clone(),
                    &self.uniform_buffers_frag,
                    self.descriptor_set_allocator.clone(),
                ).context("failed to update pipeline")?;
                pipeline_changed |= pipeline.get_pipeline().is_some();
            }
        }
        if pipeline_changed {
            unsafe { self.device.wait_idle().unwrap(); }
            for pipeline in self.pipelines[1..].iter_mut() {
                pipeline.reload_shaders(false);
            }
            self.command_buffers = get_command_buffers(
                &self.command_buffer_allocator,
                &self.queue,
                &self.pipelines,
                &self.framebuffers,
            );
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

        let mut swapchain_dirty = suboptimal;

        // wait for the fence related to this image to finish (normally this would be the oldest fence)
        if let Some(image_fence) = &self.fences[image_i as usize] {
            image_fence.wait(None).unwrap();
        }

        let previous_future = match self.fences[self.previous_fence_i].clone() {
            // Create a NowFuture
            None => {
                let mut now = sync::now(self.device.clone());
                now.cleanup_finished();
                now.boxed()
            }
            // Use the existing FenceSignalFuture
            Some(fence) => fence.boxed(),
        };

        self.update_uniform_buffer(image_i as _, time);

        let future = previous_future
            .join(acquire_future)
            .then_execute(self.queue.clone(), self.command_buffers[image_i as usize].clone())
            .unwrap()
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_i),
            )
            .then_signal_fence_and_flush();

        self.fences[image_i as usize] = match future.map_err(Validated::unwrap) {
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

        self.previous_fence_i = image_i as _;
        Ok(swapchain_dirty)
    }

    fn update_uniform_buffer(&self, image_index: usize, time: f32) {
        let aspect_ratio = self.swapchain.image_extent()[0] as f32
            / self.swapchain.image_extent()[1] as f32;
        let proj = Mat4::perspective_rh(
            75_f32.to_radians(),
            aspect_ratio,
            0.01,
            200.0,
        );
        for pipeline in self.pipelines.iter() {
            if let Err(err) = pipeline.update_uniform_buffer(image_index, self.view_matrix, proj) {
                log::error!("failed to update uniforms: {err:?}");
            }
        }
        let write = self.uniform_buffers_frag[image_index].write();
        match write {
            Ok(mut write) => *write = fs::UniformBufferObject {
                time,
            },
            Err(err) => log::error!("failed to update uniforms: {err:?}"),
        }
    }
}

pub fn model_to_buffers<V>(
    vertices: &[V],
    indices: &[u32],
    memory_allocator: Arc<StandardMemoryAllocator>,
) -> (Subbuffer<[V]>, Subbuffer<[u32]>)
where
    V: MyVertexTrait + Copy,
{
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
    ).unwrap();

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
    ).unwrap();

    (vertex_buffer, index_buffer)
}
