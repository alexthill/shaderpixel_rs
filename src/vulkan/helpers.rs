use super::pipeline::MyPipeline;

use std::sync::Arc;

use vulkano::{
    command_buffer::{
        allocator::StandardCommandBufferAllocator,
        AutoCommandBufferBuilder, CommandBufferInheritanceInfo, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderPassBeginInfo,
        SecondaryAutoCommandBuffer, SubpassBeginInfo, SubpassContents,
    },
    device::{
        physical::{PhysicalDevice, PhysicalDeviceType},
        Device, DeviceExtensions, Queue, QueueFlags
    },
    format::{ClearValue, Format},
    image::{
        view::ImageView,
        sys::ImageCreateInfo,
        Image, ImageFormatInfo, ImageTiling, ImageType, ImageUsage, SampleCount,
    },
    instance::Instance,
    memory::allocator::{AllocationCreateInfo, MemoryAllocator},
    pipeline::{
        Pipeline, PipelineBindPoint,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    swapchain::{Surface, Swapchain},
};

pub mod vs {
    vulkano_shaders::shader! {
        ty: "vertex",
        src: r"
            #version 450

            layout(location = 0) in vec3 position;
            layout(location = 1) in vec3 normal;

            layout(set = 0, binding = 0) uniform UniformBufferObject {
                mat4 model;
                mat4 view;
                mat4 proj;
            } ubo;

            layout(location = 0) out vec3 fragPos;
            layout(location = 1) out vec3 fragNorm;

            void main() {
                fragPos = (ubo.model * vec4(position, 1.0)).xyz;

                mat3 norm_matrix = transpose(inverse(mat3(ubo.model)));
                fragNorm = normalize(norm_matrix * normal);

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

            layout(location = 0) in vec3 fragPos;
            layout(location = 1) in vec3 fragNorm;

            layout(location = 0) out vec4 outColor;

            // each element in an array takes up the same space as a whole vec4
            // use a vec4 as better alternative
            layout(set = 0, binding = 1) uniform UniformBufferObject {
                vec4 light_pos;
                vec4 options;
                float time;
            } ubo;

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

                vec3 normal = normalize(fragNorm);
                vec3 to_light_dir = normalize(ubo.light_pos.xyz - fragPos);
                float ambient_coef = 0.4;
                float diffuse_coef = max(0.0, dot(normal, to_light_dir));
                color = color * min(2.0, ambient_coef + diffuse_coef);

                outColor = vec4(color, 1.0);
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

pub fn select_msaa_sample_count(device: &PhysicalDevice) -> SampleCount {
    let color_sample_counts = device.properties().framebuffer_color_sample_counts;
    let depth_sample_counts = device.properties().framebuffer_depth_sample_counts;
    let sample_counts = color_sample_counts.intersection(depth_sample_counts);
    [SampleCount::Sample8, SampleCount::Sample4, SampleCount::Sample2]
        .into_iter()
        .find(|sample_count| sample_counts.contains_enum(*sample_count))
        .unwrap_or(SampleCount::Sample1)
}

pub fn get_render_pass(
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    depth_format: Format,
    msaa_sample_count: SampleCount,
) -> Arc<RenderPass> {
    vulkano::ordered_passes_renderpass!(
        device,
        attachments: {
            mirror: {
                format: swapchain.image_format(),
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
            intermediary: {
                format: swapchain.image_format(),
                samples: msaa_sample_count as u32,
                load_op: Clear,
                store_op: Store,
            },
            depth_stencil_no_msaa: {
                format: depth_format,
                samples: 1,
                load_op: Clear,
                store_op: DontCare,
            },
            depth_stencil: {
                format: depth_format,
                samples: msaa_sample_count as u32,
                load_op: Clear,
                store_op: DontCare,
            },
            color: {
                format: swapchain.image_format(),
                samples: 1,
                load_op: DontCare,
                store_op: Store,
            },
        },
        passes: [
            // Mirror render pass
            {
                color: [mirror],
                depth_stencil: {depth_stencil_no_msaa},
                input: [],
            },
            // Scene render pass
            {
                color: [intermediary],
                color_resolve: [color],
                depth_stencil: {depth_stencil},
                input: [mirror],
            },
            // Gui render pass
            {
                color: [color],
                depth_stencil: {},
                input: [],
            },
        ],
    )
    .unwrap()
}

pub fn get_mirror_buffer(
    format: Format,
    extent: [u32; 3],
    memory_allocator: Arc<dyn MemoryAllocator>,
) -> Arc::<ImageView> {
    ImageView::new_default(
        Image::new(
            memory_allocator,
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format,
                extent,
                usage: ImageUsage::COLOR_ATTACHMENT
                    | ImageUsage::INPUT_ATTACHMENT
                    | ImageUsage::TRANSIENT_ATTACHMENT,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        ).unwrap(),
    ).unwrap()
}

pub fn get_framebuffers(
    images: &[Arc<Image>],
    depth_format: Format,
    render_pass: Arc<RenderPass>,
    memory_allocator: Arc<dyn MemoryAllocator>,
    msaa_sample_count: SampleCount,
    mirror_buffer: &Arc<ImageView>,
) -> Vec<Arc<Framebuffer>> {
    println!("image {:#?}", images[0]);
    let intermediary = ImageView::new_default(
        Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: images[0].format(),
                extent: images[0].extent(),
                usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                samples: msaa_sample_count,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();
    let depth_buffer_no_msaa = ImageView::new_default(
        Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: depth_format,
                extent: images[0].extent(),
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();
    let depth_buffer = ImageView::new_default(
        Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: depth_format,
                extent: images[0].extent(),
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                samples: msaa_sample_count,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();

    images
        .iter()
        .enumerate()
        .map(|(image_i, image)| {
            let view = ImageView::new_default(image.clone()).unwrap();
            let fb = Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![
                        mirror_buffer.clone(),
                        intermediary.clone(),
                        depth_buffer_no_msaa.clone(),
                        depth_buffer.clone(),
                        view,
                    ],
                    ..Default::default()
                },
            )
            .unwrap();
            for (i, a) in fb.attachments().iter().enumerate() {
                log::info!("{image_i}:{i}: {a:#?}");
            }
            fb
        })
        .collect::<Vec<_>>()
}

pub fn get_primary_command_buffer(
    command_buffer_allocator: &Arc<StandardCommandBufferAllocator>,
    queue: &Arc<Queue>,
    framebuffer: Arc<Framebuffer>,
    subpasses: impl IntoIterator<Item = Arc<SecondaryAutoCommandBuffer>>,
) -> anyhow::Result<Arc<PrimaryAutoCommandBuffer>> {
    let mut subpasses = subpasses.into_iter();
    let mut builder = AutoCommandBufferBuilder::primary(
        command_buffer_allocator.clone(),
        queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
    )?;
    builder
        .begin_render_pass(
            RenderPassBeginInfo {
                clear_values: vec![
                    Some([0.0, 0.8, 0.0, 1.0].into()),  // mirror color
                    Some([0.0, 0.0, 0.8, 1.0].into()),  // intermediary color
                    Some(ClearValue::Depth(1.0)),       // depth no msaa
                    Some(ClearValue::Depth(1.0)),       // depth
                    None,                               // final color
                ],
                ..RenderPassBeginInfo::framebuffer(framebuffer)
            },
            SubpassBeginInfo {
                contents: SubpassContents::SecondaryCommandBuffers,
                ..Default::default()
            },
        )?;
    builder.execute_commands(subpasses.next().expect("no subpasses"))?;
    for subpass in subpasses {
        builder
            .next_subpass(
                Default::default(),
                SubpassBeginInfo {
                    contents: SubpassContents::SecondaryCommandBuffers,
                    ..Default::default()
                }
            )?
            .execute_commands(subpass)?;
    }
    builder.end_render_pass(Default::default())?;
    Ok(builder.build()?)
}

use vulkano::image::{ImageLayout, ImageSubresourceRange};
use vulkano::sync::{AccessFlags, ImageMemoryBarrier};

fn _get_image_memory_barrier(
    image: Arc<Image>,
) -> ImageMemoryBarrier {
    let format = image.format();
    let mut barrier = ImageMemoryBarrier::image(image);
    barrier.src_access = AccessFlags::SHADER_WRITE;
    barrier.dst_access = AccessFlags::SHADER_READ;
    barrier.old_layout = ImageLayout::Undefined;
    barrier.new_layout = ImageLayout::ShaderReadOnlyOptimal;
    barrier.subresource_range = ImageSubresourceRange::from_parameters(format, 1, 1);
    barrier
        /*
        src_stages: PipelineStages,
        src_access: AccessFlags,
        dst_stages: PipelineStages,
        dst_access: AccessFlags,
        old_layout: ImageLayout,
        new_layout: ImageLayout,
        queue_family_ownership_transfer: Option<QueueFamilyOwnershipTransfer>,
        image,
        subresource_range: ImageSubresourceRange,
        ..Default::default()
        */
}

pub fn get_command_buffers(
    count: usize,
    command_buffer_allocator: &Arc<StandardCommandBufferAllocator>,
    queue: &Arc<Queue>,
    pipelines: &[MyPipeline],
    pipeline_order: &[usize],
    subpass: &Subpass,
) -> Vec<Arc<SecondaryAutoCommandBuffer>> {
    (0..count).map(|i| {
        let mut builder = AutoCommandBufferBuilder::secondary(
            command_buffer_allocator.clone(),
            queue.queue_family_index(),
            CommandBufferUsage::MultipleSubmit,
            CommandBufferInheritanceInfo {
                render_pass: Some(subpass.clone().into()),
                ..Default::default()
            },
        )
        .unwrap();
        for &pip_idx in pipeline_order {
            let my_pipeline = &pipelines[pip_idx];
            if !my_pipeline.enable_pipeline {
                continue;
            }
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
                    my_pipeline.get_descriptor_sets().unwrap()[i].clone(),
                )
                .unwrap()
                .bind_vertex_buffers(0, vertex_buffer.clone())
                .unwrap()
                .bind_index_buffer(index_buffer.clone())
                .unwrap();
            unsafe { builder.draw_indexed(index_buffer.len() as u32, 1, 0, 0, 0) }
                .unwrap();
        }
        builder.build().unwrap()
    }).collect()
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
