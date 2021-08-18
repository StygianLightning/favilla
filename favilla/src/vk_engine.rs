use ash::extensions::khr::{Surface, Swapchain};

use ash::vk::{DescriptorSet, RenderPass, SurfaceCapabilitiesKHR, SurfaceKHR};
use ash::{vk, Device, Instance};

use crate::app::App;
use crate::queue_families::DeviceQueueFamilies;
use crate::swapchain::SwapchainManager;
use ash::prelude::VkResult;
use raw_window_handle::HasRawWindowHandle;
use std::default::Default;
use std::os::raw::c_char;
use tracing::{event, Level};

/// Holds commonly used Vulkan structures and provides some utility methods.
pub struct VulkanEngine {
    pub num_frames: u32,
    pub current_frame: u32,
    pub device: Device,

    pub desired_swapchain_image_count: u32,
    pub physical_device: vk::PhysicalDevice,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub queue_family_index: u32,
    pub present_queue: vk::Queue,

    pub surface_loader: Surface,
    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,
    pub surface_capabilities: SurfaceCapabilitiesKHR,
}

impl VulkanEngine {
    /// Create a new `VulkanEngine`.
    pub unsafe fn new(
        app: &App,
        surface: SurfaceKHR,
        device_extension_names_raw: &[*const c_char],
        features: vk::PhysicalDeviceFeatures,
        queue_families: DeviceQueueFamilies,
        surface_format: vk::SurfaceFormatKHR,
        num_frames: u32,
        window_extent: vk::Extent2D,
    ) -> Self {
        let DeviceQueueFamilies {
            physical_device,
            queue_family_index,
            surface_loader,
        } = queue_families;

        let priorities = [1.0];

        let queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(queue_family_index)
            .queue_priorities(&priorities)
            .build()];

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_info)
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features);

        let device: Device = app
            .instance
            .create_device(physical_device, &device_create_info, None)
            .unwrap();

        // We might want to add support for separate present and graphics queues,
        // but it seems there's currently no hardware that supports graphics but not presenting.
        let present_queue = device.get_device_queue(queue_family_index as u32, 0);

        let device_memory_properties = app
            .instance
            .get_physical_device_memory_properties(physical_device);

        let surface_capabilities = surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface)
            .unwrap();
        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0
            && desired_image_count > surface_capabilities.max_image_count
        {
            desired_image_count = surface_capabilities.max_image_count;
        }

        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => window_extent,
            _ => surface_capabilities.current_extent,
        };
        Self {
            num_frames,
            current_frame: 0,
            surface_capabilities,
            device,
            surface_loader,
            physical_device,
            device_memory_properties,
            queue_family_index,
            present_queue,
            surface,
            surface_format,
            surface_resolution,
            desired_swapchain_image_count: desired_image_count,
        }
    }

    pub unsafe fn destroy(&mut self) {
        self.device.device_wait_idle().unwrap();
        self.device.destroy_device(None);
        self.surface_loader.destroy_surface(self.surface, None);
    }

    /// Perform a new memory allocation. Panics if the allocation fails.
    pub unsafe fn allocate_memory(
        &self,
        memory_req: vk::MemoryRequirements,
        memory_type_index: u32,
    ) -> vk::DeviceMemory {
        self.try_allocate_memory(memory_req, memory_type_index)
            .expect("Failed to allocate memory")
    }

    /// Perform a new memory allocation.
    pub unsafe fn try_allocate_memory(
        &self,
        memory_req: vk::MemoryRequirements,
        memory_type_index: u32,
    ) -> Result<vk::DeviceMemory, vk::Result> {
        let buffer_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: memory_req.size,
            memory_type_index,
            ..Default::default()
        };
        self.device.allocate_memory(&buffer_allocate_info, None)
    }

    /// Advance to the next frame.
    /// This has to be called every frame in order for the synchronisation support to work properly.
    pub fn advance_frame(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.num_frames;
    }

    /// Recreate the swapchain. This will wait until the device is idle. Uses `SwapchainManager` under the hood.
    pub unsafe fn recreate_swapchain(
        &mut self,
        instance: &Instance,
        new_extent: vk::Extent2D,
        swapchain_manager: &mut SwapchainManager,
        render_pass: RenderPass,
    ) {
        self.device.device_wait_idle().unwrap();

        self.surface_capabilities = self
            .surface_loader
            .get_physical_device_surface_capabilities(self.physical_device, self.surface)
            .unwrap();

        self.surface_resolution = match self.surface_capabilities.current_extent.width {
            u32::MAX => new_extent,
            _ => self.surface_capabilities.current_extent,
        };

        swapchain_manager.destroy(&self.device);
        *swapchain_manager = SwapchainManager::new(&instance, self, render_pass);
    }

    /// Allocates descriptor sets.
    pub unsafe fn allocate_descriptor_sets(
        &self,
        set_layouts: &[vk::DescriptorSetLayout],
        descriptor_pool: vk::DescriptorPool,
    ) -> VkResult<Vec<DescriptorSet>> {
        self.device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder()
                .set_layouts(set_layouts)
                .descriptor_pool(descriptor_pool)
                .build(),
        )
    }

    /// Utility function for executing commands with a one time use command buffer (allocated every time).
    pub unsafe fn one_time_submit<F>(&self, command_pool: vk::CommandPool, f: F) -> ()
    where
        F: FnOnce(vk::CommandBuffer),
    {
        let tmp_command_buffer = self
            .device
            .allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_pool(command_pool)
                    .command_buffer_count(1)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .build(),
            )
            .expect("couldn't allocate command buffer")[0];

        self.device
            .begin_command_buffer(
                tmp_command_buffer,
                &vk::CommandBufferBeginInfo {
                    flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                    ..Default::default()
                },
            )
            .expect("Failed to begin command buffer");

        f(tmp_command_buffer);

        self.device
            .end_command_buffer(tmp_command_buffer)
            .expect("Ending command buffer in one_time_submit failed.");
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[tmp_command_buffer])
            .build();

        let fence_info = vk::FenceCreateInfo {
            ..Default::default()
        };
        let fence = self.device.create_fence(&fence_info, None).unwrap();

        self.device
            .queue_submit(self.present_queue, &[submit_info], fence)
            .expect("Failed to submit temporary command buffer");
        self.device
            .wait_for_fences(&[fence], true, u64::MAX)
            .unwrap();

        self.device.destroy_fence(fence, None);

        self.device
            .free_command_buffers(command_pool, &[tmp_command_buffer]);
    }
}
