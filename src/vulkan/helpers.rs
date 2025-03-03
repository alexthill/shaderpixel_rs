use crate::model::obj::NormalizedObj;
use super::{
    pipeline::MyPipeline,
    vertex::MyVertexTrait,
};

use std::sync::Arc;

use glam::Vec3;
use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator,
        AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
        SubpassBeginInfo, SubpassContents,
    },
    descriptor_set::DescriptorSet,
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceExtensions, Queue, QueueFlags
    },
    format::{ClearValue, Format},
    image::{
        view::ImageView,
        sys::ImageCreateInfo,
        Image, ImageFormatInfo, ImageTiling, ImageType, ImageUsage,
    },
    instance::Instance,
    memory::allocator::{AllocationCreateInfo, MemoryAllocator},
    pipeline::{
        Pipeline, PipelineBindPoint,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass},
    swapchain::{Surface, Swapchain},
};

pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450

            layout(location = 0) in vec3 position;

            layout(set = 0, binding = 0) uniform UniformBufferObject {
                mat4 model;
                mat4 view;
                mat4 proj;
            } ubo;

            void main() {
                mat4 mvp = ubo.proj * ubo.view * ubo.model;
                gl_Position = mvp * vec4(position, 1.0);
                gl_Position.y = -gl_Position.y;
            }
        ",
    }
}

pub mod fs {
    vulkano_shaders::shader! {
        ty: "fragment",
        src: r"
            #version 450

            layout(location = 0) out vec4 out_color;

            // from <https://stackoverflow.com/a/10625698>
            float random(vec2 p) {
                vec2 k1 = vec2(
                    23.14069263277926, // e^pi
                    2.665144142690225  // 2^sqrt(2)
                );
                return fract(cos(dot(p, k1)) * 12345.6789);
            }

            void main() {
                vec3 color = vec3(
                    random(vec2(gl_PrimitiveID, 1.1)),
                    random(vec2(gl_PrimitiveID, 2.2)),
                    random(vec2(gl_PrimitiveID, 3.3))
                );
                out_color = vec4(color, 1.0);
            }
        ",
    }
}

pub fn select_physical_device(
    instance: &Arc<Instance>,
    surface: &Arc<Surface>,
    device_extensions: &DeviceExtensions,
) -> (Arc<PhysicalDevice>, u32) {
    instance
        .enumerate_physical_devices()
        .expect("failed to enumerate physical devices")
        .filter(|p| p.supported_extensions().contains(device_extensions))
        .filter_map(|p| {
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    q.queue_flags.contains(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, surface).unwrap_or(false)
                })
                .map(|q| (p, q as u32))
        })
        .min_by_key(|(p, _)| match p.properties().device_type {
            PhysicalDeviceType::DiscreteGpu => 0,
            PhysicalDeviceType::IntegratedGpu => 1,
            PhysicalDeviceType::VirtualGpu => 2,
            PhysicalDeviceType::Cpu => 3,
            _ => 4,
        })
        .expect("no device available")
}

pub fn get_render_pass(
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    depth_format: Format,
) -> Arc<RenderPass> {
    vulkano::single_pass_renderpass!(
        device,
        attachments: {
            color: {
                format: swapchain.image_format(), // set the format the same as the swapchain
                samples: 1,
                load_op: Clear,
                store_op: Store,
            },
            depth_stencil: {
                format: depth_format,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
        },
        pass: {
            color: [color],
            depth_stencil: {depth_stencil},
        },
    )
    .unwrap()
}

pub fn get_framebuffers(
    images: &[Arc<Image>],
    depth_buffer: &Arc<ImageView>,
    render_pass: Arc<RenderPass>
) -> Vec<Arc<Framebuffer>> {
    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view, depth_buffer.clone()],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>()
}

pub fn get_command_buffers(
    command_buffer_allocator: &Arc<StandardCommandBufferAllocator>,
    queue: &Arc<Queue>,
    pipelines: &[MyPipeline],
    framebuffers: &[Arc<Framebuffer>],
    descriptor_sets: &[Arc<DescriptorSet>],
) -> Vec<Arc<PrimaryAutoCommandBuffer>> {
    framebuffers
        .iter()
        .enumerate()
        .map(|(i, framebuffer)| {
            let mut builder = AutoCommandBufferBuilder::primary(
                command_buffer_allocator.clone(),
                queue.queue_family_index(),
                CommandBufferUsage::MultipleSubmit,
            )
            .unwrap();

            builder
                .begin_render_pass(
                    RenderPassBeginInfo {
                        clear_values: vec![
                            Some([0.0, 0.0, 0.8, 1.0].into()),  // color
                            Some(ClearValue::Depth(1.0)),       // depth
                        ],
                        ..RenderPassBeginInfo::framebuffer(framebuffer.clone())
                    },
                    SubpassBeginInfo {
                        contents: SubpassContents::Inline,
                        ..Default::default()
                    },
                )
                .unwrap();
            for my_pipeline in pipelines {
                let Some(pipeline) = my_pipeline.get_pipeline() else {
                    continue;
                };
                let vertex_buffer = my_pipeline.get_vertex_buffer();
                let index_buffer = my_pipeline.get_index_buffer();
                builder
                    .bind_pipeline_graphics(pipeline.clone())
                    .unwrap()
                    .bind_descriptor_sets(
                        PipelineBindPoint::Graphics,
                        pipeline.layout().clone(),
                        0,
                        descriptor_sets[i].clone(),
                    )
                    .unwrap()
                    .bind_vertex_buffers(0, vertex_buffer.clone())
                    .unwrap()
                    .bind_index_buffer(index_buffer.clone())
                    .unwrap();
                unsafe { builder.draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0) }
                    .unwrap();
            }
            builder
                .end_render_pass(Default::default())
                .unwrap();

            builder.build().unwrap()
        })
        .collect()
}

pub fn find_depth_format(device: &PhysicalDevice) -> Option<Format> {
    let candidates = [
        Format::D32_SFLOAT,
        Format::D32_SFLOAT_S8_UINT,
        Format::D24_UNORM_S8_UINT,
        Format::D16_UNORM,
    ];
    candidates.into_iter().find(|&format| {
        device.image_format_properties(ImageFormatInfo {
            format,
            image_type: ImageType::Dim2d,
            tiling: ImageTiling::Optimal,
            usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT,
            ..Default::default()
        }).ok().is_some()
    })
}

pub fn create_depth_buffer(
    memory_allocator: Arc<dyn MemoryAllocator>,
    extent: [u32; 3],
    format: Format,
) -> Arc<ImageView> {
    ImageView::new_default(
        Image::new(
            memory_allocator,
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format,
                extent,
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap()
}

pub fn load_model<V: MyVertexTrait>(model: &NormalizedObj) -> (Vec<V>, &[u32], (Vec3, Vec3)) {
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for vertex in &model.vertices {
        for (i, &coord) in vertex.pos_coords.iter().enumerate() {
            min[i] = min[i].min(coord);
            max[i] = max[i].max(coord);
        }
    }
    let vertices = model.vertices.iter().map(|vertex| {
        let tex_coords = if model.has_tex_coords {
            vertex.tex_coords
        } else {
            [vertex.pos_coords[2], vertex.pos_coords[1]]
        };
        let norm = [0.; 3]; // TODO implement getting norm
        V::new(vertex.pos_coords, norm, tex_coords)
    }).collect();

    (vertices, &model.indices, (min, max))
}
