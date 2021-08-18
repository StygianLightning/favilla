use crate::vk_engine::VulkanEngine;
use ash::{vk, Device};

/// Helper struct holding a command pool and per-frame data: semaphores, fences and command buffers.
pub struct FrameDataManager {
    pub frame_data: Vec<PerFrameData>,
    pub command_pool: vk::CommandPool,
}

impl FrameDataManager {
    pub unsafe fn new(vk_engine: &VulkanEngine) -> Self {
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                    | vk::CommandPoolCreateFlags::TRANSIENT,
            )
            .queue_family_index(vk_engine.queue_family_index);
        let pool = vk_engine
            .device
            .create_command_pool(&pool_create_info, None)
            .unwrap();

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(vk_engine.num_frames)
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let command_buffers_per_frame = vk_engine
            .device
            .allocate_command_buffers(&command_buffer_allocate_info)
            .unwrap();

        let semaphore_create_info = vk::SemaphoreCreateInfo::default();

        let mut image_acquired_semaphores = vec![];
        let mut render_complete_semaphores = vec![];
        let mut frame_fences = vec![];

        for _ in 0..vk_engine.num_frames {
            let image_acquired_semaphore = vk_engine
                .device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap();
            image_acquired_semaphores.push(image_acquired_semaphore);
            let render_complete_semaphore = vk_engine
                .device
                .create_semaphore(&semaphore_create_info, None)
                .unwrap();
            render_complete_semaphores.push(render_complete_semaphore);

            let fence_info = vk::FenceCreateInfo {
                flags: vk::FenceCreateFlags::SIGNALED,
                ..Default::default()
            };
            let fence = vk_engine.device.create_fence(&fence_info, None).unwrap();
            frame_fences.push(fence);
        }
        let per_frame_data = (0..vk_engine.num_frames as usize)
            .map(|i| PerFrameData {
                image_acquired_semaphore: image_acquired_semaphores[i],
                render_complete_semaphore: render_complete_semaphores[i],
                frame_fence: frame_fences[i],
                command_buffer: command_buffers_per_frame[i],
            })
            .collect();

        Self {
            frame_data: per_frame_data,
            command_pool: pool,
        }
    }

    /// Frees all resources held by `self`.
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_command_pool(self.command_pool, None);
        for frame_data in &mut self.frame_data {
            frame_data.destroy(device);
        }
    }
}

/// Data held by `FrameDataManager`.
pub struct PerFrameData {
    pub frame_fence: vk::Fence,
    pub command_buffer: vk::CommandBuffer,
    pub image_acquired_semaphore: vk::Semaphore,
    pub render_complete_semaphore: vk::Semaphore,
}

impl PerFrameData {
    /// Frees all resources held by `self`.
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_semaphore(self.image_acquired_semaphore, None);
        device.destroy_semaphore(self.render_complete_semaphore, None);
        device.destroy_fence(self.frame_fence, None);
    }
}
