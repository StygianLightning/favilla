use ash::extensions::{
    ext::DebugUtils,
    khr::{Surface, Swapchain},
};

use ash::{vk, Device, Entry, Instance};

/// Holds commonly used Vulkan structures.
pub struct Renderer {
    pub num_frames: u32,
    pub entry: Entry,
    pub instance: Instance,
    pub device: Device,
    pub surface_loader: Surface,
    pub swapchain_loader: Swapchain,
    pub debug_utils_loader: DebugUtils,
    pub debug_call_back: vk::DebugUtilsMessengerEXT,

    pub physical_device: vk::PhysicalDevice,
    pub device_memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub queue_family_index: u32,
    pub present_queue: vk::Queue,

    pub renderpass: vk::RenderPass,

    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    pub surface_resolution: vk::Extent2D,

    pub swapchain: vk::SwapchainKHR,
    pub present_images: Vec<vk::Image>,
    pub present_image_views: Vec<vk::ImageView>,
    pub framebuffers: Vec<vk::Framebuffer>,

    pub pool: vk::CommandPool,
    pub command_buffers_per_frame: Vec<vk::CommandBuffer>,

    pub image_acquired_semaphores: Vec<vk::Semaphore>,
    pub render_complete_semaphores: Vec<vk::Semaphore>,
    pub frame_fences: Vec<vk::Fence>,

    pub pipeline_layout: vk::PipelineLayout,
    pub graphics_pipeline: vk::Pipeline,
    pub vertex_shader: vk::ShaderModule,
    pub fragment_shader: vk::ShaderModule,
}

impl Renderer {
    pub unsafe fn destroy(&mut self) {
        self.device.device_wait_idle().unwrap();

        self.device.destroy_shader_module(self.vertex_shader, None);
        self.device
            .destroy_shader_module(self.fragment_shader, None);

        self.device
            .destroy_pipeline_layout(self.pipeline_layout, None);
        self.device.destroy_pipeline(self.graphics_pipeline, None);

        for framebuffer in &self.framebuffers {
            self.device.destroy_framebuffer(*framebuffer, None);
        }
        for i in 0..self.num_frames {
            self.device
                .destroy_semaphore(self.image_acquired_semaphores[i as usize], None);
            self.device
                .destroy_semaphore(self.render_complete_semaphores[i as usize], None);
            self.device
                .destroy_fence(self.frame_fences[i as usize], None);
        }
        self.device.destroy_render_pass(self.renderpass, None);
        /*
        self.device.free_memory(self.depth_image_memory, None);
        self.device.destroy_image_view(self.depth_image_view, None);
        self.device.destroy_image(self.depth_image, None);
        */
        for &image_view in self.present_image_views.iter() {
            self.device.destroy_image_view(image_view, None);
        }
        self.device.destroy_command_pool(self.pool, None);
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None);
        self.device.destroy_device(None);
        self.surface_loader.destroy_surface(self.surface, None);
        self.debug_utils_loader
            .destroy_debug_utils_messenger(self.debug_call_back, None);
        self.instance.destroy_instance(None);
    }
}
