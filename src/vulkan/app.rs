use super::debug::*;
use super::helpers::*;

use std::sync::Arc;

use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{CommandBufferExecFuture, PrimaryAutoCommandBuffer},
    command_buffer::allocator::StandardCommandBufferAllocator,
    device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo},
    image::ImageUsage,
    instance::{Instance, InstanceCreateFlags, InstanceCreateInfo},
    instance::debug::DebugUtilsMessenger,
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    pipeline::graphics::viewport::Viewport,
    render_pass::RenderPass,
    shader::ShaderModule,
    swapchain::{self, PresentFuture, Surface, Swapchain, SwapchainAcquireFuture,
        SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{
        self,
        future::{FenceSignalFuture, JoinFuture},
        GpuFuture
    },
    Validated, VulkanError,
};
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub struct App {
    #[allow(dead_code)]
    instance: Arc<Instance>,
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    render_pass: Arc<RenderPass>,
    viewport: Viewport,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    command_buffers: Vec<Arc<PrimaryAutoCommandBuffer>>,
    queue: Arc<Queue>,
    vertex_buffer: Subbuffer<[MyVertex]>,
    fences: Vec<Option<Arc<FenceSignalFuture<PresentFuture<CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>>>>>>,
    previous_fence_i: usize,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,

    // If this falls out of scope then there will be no more debug events.
    // Put it at the end so that it will get dropped last.
    #[allow(dead_code)]
    debug: DebugUtilsMessenger,

}

impl App {
    pub fn new(window: Arc<Window>) -> Self {
        log::debug!("creating vulkan app");

        let dimensions = window.inner_size();
        let library = vulkano::VulkanLibrary::new()
            .expect("no local Vulkan library/DLL");

        let (debug_extensions, debug_layers) = get_debug_extensions_and_layers();
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

        let (physical_device, queue_family_index) =
            select_physical_device(&instance, &surface, &device_extensions);

        let (device, mut queues) = Device::new(
            physical_device.clone(),
            DeviceCreateInfo {
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions, // new
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

            Swapchain::new(
                device.clone(),
                surface,
                SwapchainCreateInfo {
                    min_image_count: caps.min_image_count,
                    image_format,
                    image_extent: dimensions.into(),
                    image_usage: ImageUsage::COLOR_ATTACHMENT,
                    composite_alpha,
                    ..Default::default()
                },
            )
            .unwrap()
        };

        let render_pass = get_render_pass(device.clone(), swapchain.clone());
        let framebuffers = get_framebuffers(&images, render_pass.clone());

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let vertex1 = MyVertex {
            position: [-0.5, -0.5],
        };
        let vertex2 = MyVertex {
            position: [0.0, 0.5],
        };
        let vertex3 = MyVertex {
            position: [0.5, -0.25],
        };
        let vertex_buffer = Buffer::from_iter(
            memory_allocator,
            BufferCreateInfo {
                usage: BufferUsage::VERTEX_BUFFER,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            vec![vertex1, vertex2, vertex3],
        )
        .unwrap();

        let vs = vs::load(device.clone()).expect("failed to create shader module");
        let fs = fs::load(device.clone()).expect("failed to create shader module");

        let viewport = Viewport {
            offset: [0.0, 0.0],
            extent: dimensions.into(),
            depth_range: 0.0..=1.0,
        };

        let pipeline = get_pipeline(
            device.clone(),
            vs.clone(),
            fs.clone(),
            render_pass.clone(),
            viewport.clone(),
        );

        let command_buffer_allocator =
            Arc::new(StandardCommandBufferAllocator::new(device.clone(), Default::default()));

        let command_buffers = get_command_buffers(
            &command_buffer_allocator,
            &queue,
            &pipeline,
            &framebuffers,
            &vertex_buffer,
        );

        let frames_in_flight = images.len();
        let fences: Vec<Option<Arc<FenceSignalFuture<_>>>> = vec![None; frames_in_flight];

        Self {
            instance,
            debug,
            device,
            swapchain,
            render_pass,
            viewport,
            command_buffer_allocator,
            command_buffers,
            queue,
            vertex_buffer,
            fences,
            previous_fence_i: 0,
            vs,
            fs,
        }
    }

    pub fn recreate_swapchain(&mut self, dimensions: PhysicalSize<u32>) {
        let (new_swapchain, new_images) = self.swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: dimensions.into(),
                ..self.swapchain.create_info()
            })
            .expect("failed to recreate swapchain");

        self.swapchain = new_swapchain;
        let new_framebuffers = get_framebuffers(&new_images, self.render_pass.clone());

        self.viewport.extent = dimensions.into();
        let new_pipeline = get_pipeline(
            self.device.clone(),
            self.vs.clone(),
            self.fs.clone(),
            self.render_pass.clone(),
            self.viewport.clone(),
        );
        self.command_buffers = get_command_buffers(
            &self.command_buffer_allocator,
            &self.queue,
            &new_pipeline,
            &new_framebuffers,
            &self.vertex_buffer,
        );
    }

    pub fn draw(&mut self) -> bool {
        let (image_i, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None)
                .map_err(Validated::unwrap)
            {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    return true;
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
        swapchain_dirty
    }
}
