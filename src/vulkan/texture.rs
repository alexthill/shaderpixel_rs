use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use vulkano::{
    buffer::{Buffer, BufferCreateInfo, BufferUsage},
    command_buffer::{
        allocator::StandardCommandBufferAllocator,
        AutoCommandBufferBuilder, BlitImageInfo, CommandBufferUsage, CopyBufferToImageInfo,
        ImageBlit, PrimaryCommandBufferAbstract,
    },
    device::{physical::PhysicalDevice, Device, Queue},
    format::{Format, FormatFeatures},
    image::{
        view::ImageView,
        sampler::{Filter, Sampler, SamplerCreateInfo},
        Image, ImageAspects, ImageCreateInfo, ImageSubresourceLayers, ImageType, ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
    DeviceSize,
};

use image::ImageReader;

pub struct Texture {
    pub view: Arc<ImageView>,
    pub sampler: Arc<Sampler>,
}

impl Texture {
    pub fn new<P: AsRef<Path>>(
        path: P,
        device: Arc<Device>,
        queue: Arc<Queue>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        memory_allocator: Arc<StandardMemoryAllocator>,
    ) -> anyhow::Result<Self> {
        let mut command_buffer = AutoCommandBufferBuilder::primary(
            command_buffer_allocator.clone(),
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        let image = ImageReader::open(&path)
            .with_context(|| format!("failed to open image at {:?}", path.as_ref()))?
            .decode()
            .with_context(|| format!("failed to decode image at {:?}", path.as_ref()))?
            .flipv();
        let image_as_rgba = image.into_rgba8();
        let width = image_as_rgba.width();
        let height = image_as_rgba.height();
        let mip_levels = ((width.min(height) as f32).log2().floor() + 1.0) as u32;
        let format = Format::R8G8B8A8_UNORM;
        let extent = [width, height, 1];

        let upload_buffer = Buffer::new_slice(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            format.block_size() * width as DeviceSize * height as DeviceSize,
        )?;

        upload_buffer.write()?.copy_from_slice(image_as_rgba.as_raw());

        let image = Image::new(
            memory_allocator,
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format,
                extent,
                mip_levels,
                usage: ImageUsage::TRANSFER_SRC | ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )?;

        command_buffer.copy_buffer_to_image(
            CopyBufferToImageInfo::buffer_image(upload_buffer, image.clone()),
        )?;

        let view = ImageView::new_default(image.clone())?;
        let sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo::simple_repeat_linear(),
        )?;

        let _ = command_buffer.build()?.execute(queue.clone())?;
        Self::generate_mipmaps(
            device.physical_device(),
            queue,
            command_buffer_allocator,
            image,
            extent,
            format,
            mip_levels,
        )?;

        Ok(Self {
            view,
            sampler,
        })
    }

   fn generate_mipmaps(
        device: &PhysicalDevice,
        queue: Arc<Queue>,
        command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
        image: Arc<Image>,
        extent: [u32; 3],
        format: Format,
        mip_levels: u32,
    ) -> anyhow::Result<()> {
        let format_properties = device.format_properties(format)?;
        let required_format_features = FormatFeatures::SAMPLED_IMAGE_FILTER_LINEAR;
        if !format_properties.optimal_tiling_features.contains(required_format_features) {
            return Err(anyhow::anyhow!("device does not support linear blitting for {format:?}"));
        }

        let mut command_buffer = AutoCommandBufferBuilder::primary(
            command_buffer_allocator,
            queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )?;

        // TODO: Are these memory barriers needed?
        // It looks like not, but maybe they improve performance.
        // see <https://vulkan-tutorial.com/Generating_Mipmaps>

        /*
        let mut barrier = vk::ImageMemoryBarrier::default()
            .image(image)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                base_array_layer: 0,
                layer_count,
                level_count: 1,
                ..Default::default()
            });
            */

        let mut mip_width = extent[0];
        let mut mip_height = extent[1];
        for level in 1..mip_levels {
            let next_mip_width = (mip_width / 2).max(1);
            let next_mip_height = (mip_height / 2).max(1);

            /*
            barrier.subresource_range.base_mip_level = level - 1;
            barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
            barrier.dst_access_mask = vk::AccessFlags::TRANSFER_READ;
            let barriers = [barrier];

            unsafe {
                vk_context.device().cmd_pipeline_barrier(
                    buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &barriers,
                )
            };
            */

            let mut blit_info = BlitImageInfo::images(image.clone(), image.clone());
            blit_info.regions[0] = ImageBlit {
                src_subresource: ImageSubresourceLayers {
                    aspects: ImageAspects::COLOR,
                    mip_level: level - 1,
                    array_layers: 0..1,
                },
                src_offsets: [[0; 3], [mip_width, mip_height, 1]],
                dst_subresource: ImageSubresourceLayers {
                    aspects: ImageAspects::COLOR,
                    mip_level: level,
                    array_layers: 0..1,
                },
                dst_offsets: [[0; 3], [next_mip_width, next_mip_height, 1]],
                ..Default::default()
            };
            blit_info.filter = Filter::Linear;
            command_buffer.blit_image(blit_info)?;

            /*
            barrier.old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
            barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
            barrier.src_access_mask = vk::AccessFlags::TRANSFER_READ;
            barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
            let barriers = [barrier];

            unsafe {
                vk_context.device().cmd_pipeline_barrier(
                    buffer,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::PipelineStageFlags::FRAGMENT_SHADER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &barriers,
                )
            };
            */

            mip_width = next_mip_width;
            mip_height = next_mip_height;
        }

        /*
        barrier.subresource_range.base_mip_level = mip_levels - 1;
        barrier.old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        barrier.new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        barrier.src_access_mask = vk::AccessFlags::TRANSFER_WRITE;
        barrier.dst_access_mask = vk::AccessFlags::SHADER_READ;
        let barriers = [barrier];

        unsafe {
            vk_context.device().cmd_pipeline_barrier(
                buffer,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &barriers,
            )
        };
        */

        let _ = command_buffer.build()?.execute(queue)?;

        Ok(())
    }
}

impl Clone for Texture {
    fn clone(&self) -> Self {
        Self {
            view: Arc::clone(&self.view),
            sampler: Arc::clone(&self.sampler),
        }
    }
}
