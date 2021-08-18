use ash::vk;

use crate::buffer::StagingBuffer;

use crate::vk_engine::VulkanEngine;
use ash::vk::{ImageLayout, ImageMemoryBarrier};
use ash::Device;

#[derive(Copy, Clone, Debug)]
pub struct TextureExtent {
    pub width: u32,
    pub height: u32,
}

pub struct Texture {
    pub format: vk::Format,
    pub image: vk::Image,
    pub extent: vk::Extent3D,
    pub num_array_layers: u32,
}

unsafe fn copy_buffer_to_image(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    image_staging_buffer: vk::Buffer,
    image: vk::Image,
    image_extent: vk::Extent3D,
    num_array_layers: u32,
) {
    // compare with https://github.com/SaschaWillems/Vulkan/blob/master/examples/texturearray/texturearray.cpp#L161
    device.cmd_copy_buffer_to_image(
        command_buffer,
        image_staging_buffer,
        image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[vk::BufferImageCopy {
            buffer_offset: 0,
            buffer_row_length: 0,
            buffer_image_height: 0,
            image_subresource: vk::ImageSubresourceLayers {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                mip_level: 0,
                base_array_layer: 0,
                layer_count: num_array_layers,
            },
            image_offset: vk::Offset3D::default(),
            image_extent,
        }],
    );
}

unsafe fn transition_layout(
    device: &Device,
    command_buffer: vk::CommandBuffer,
    image_memory_barriers: &[ImageMemoryBarrier],
    src_stage_mask: vk::PipelineStageFlags,
    dst_stage_mask: vk::PipelineStageFlags,
) {
    device.cmd_pipeline_barrier(
        command_buffer,
        src_stage_mask,
        dst_stage_mask,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        image_memory_barriers,
    );
}

impl Texture {
    /// Create a new texture.
    pub unsafe fn new(
        vk_engine: &VulkanEngine,
        format: vk::Format,
        image_type: vk::ImageType,
        image_extent: TextureExtent,
        num_array_layers: u32,
    ) -> Result<Self, vk::Result> {
        let extent = vk::Extent3D {
            width: image_extent.width,
            height: image_extent.height,
            depth: 1,
        };
        let image = vk_engine.device.create_image(
            &vk::ImageCreateInfo::builder()
                .image_type(image_type)
                .format(format)
                .extent(extent)
                .usage(vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST)
                .tiling(vk::ImageTiling::OPTIMAL)
                .samples(vk::SampleCountFlags::TYPE_1)
                .mip_levels(1)
                .array_layers(num_array_layers)
                .sharing_mode(vk::SharingMode::EXCLUSIVE)
                .build(),
            None,
        )?;

        Ok(Self {
            format,
            image,
            extent,
            num_array_layers,
        })
    }

    /// Get the memory requirements for this texture.
    pub unsafe fn get_memory_requirements(&self, device: &Device) -> vk::MemoryRequirements {
        device.get_image_memory_requirements(self.image)
    }

    /// Bind the given memory to this texture.
    pub unsafe fn bind_memory(
        &mut self,
        vk_engine: &VulkanEngine,
        memory: vk::DeviceMemory,
        offset: vk::DeviceSize,
    ) -> Result<(), vk::Result> {
        vk_engine
            .device
            .bind_image_memory(self.image, memory, offset)
    }

    /// Create an image memory barrier for the image resource held by `self`.
    pub unsafe fn get_transition_layout_image_memory_barrier(
        &self,
        src_access_mask: vk::AccessFlags,
        dst_access_mask: vk::AccessFlags,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) -> vk::ImageMemoryBarrier {
        vk::ImageMemoryBarrier::builder()
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask)
            .old_layout(old_layout)
            .new_layout(new_layout)
            .image(self.image)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .base_mip_level(0)
                    .level_count(1)
                    .base_array_layer(0)
                    .layer_count(self.num_array_layers)
                    .build(),
            )
            .build()
    }

    /// Utility method for copying data from a staging buffer to `self`.
    /// Performs a layout transition before and after copying with a single-use command buffer (using VulkanEngine::one_time_submit).
    /// Assumes that the image will be used only as a sample source in a fragment shader.
    /// This will use one pipeline barrier for every call.
    /// when dealing with many images, a manual implementation to reduce the number of pipeline barriers may be beneficial.
    pub unsafe fn copy_staging_to_image<T: Copy>(
        &mut self,
        vk_engine: &VulkanEngine,
        command_pool: vk::CommandPool,
        image_staging_buffer: &StagingBuffer<T>,
    ) {
        vk_engine.one_time_submit(command_pool, |command_buffer| {
            let barrier = self.get_transition_layout_image_memory_barrier(
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                ImageLayout::UNDEFINED,
                ImageLayout::TRANSFER_DST_OPTIMAL,
            );

            transition_layout(
                &vk_engine.device,
                command_buffer,
                &[barrier],
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
            );

            // Copy buffer to image
            copy_buffer_to_image(
                &vk_engine.device,
                command_buffer,
                image_staging_buffer.buffer.buffer,
                self.image,
                self.extent,
                self.num_array_layers,
            );

            let barrier = self.get_transition_layout_image_memory_barrier(
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
                ImageLayout::TRANSFER_DST_OPTIMAL,
                ImageLayout::SHADER_READ_ONLY_OPTIMAL,
            );

            transition_layout(
                &vk_engine.device,
                command_buffer,
                &[barrier],
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            );
        });
    }

    /// Free the image resource held by `self`.
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_image(self.image, None);
    }
}
