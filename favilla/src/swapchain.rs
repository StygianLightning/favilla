use crate::vk_engine::VulkanEngine;
use ash::extensions::khr::Swapchain;
use ash::vk::RenderPass;
use ash::{vk, Device, Instance};
use tracing::{event, info, Level};

/// Helper for swapchain management.
pub struct SwapchainManager {
    pub swapchain_loader: Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_data: Vec<PerSwapchainImage>,
}

impl SwapchainManager {
    /// Create a new swapchain manager including swapchain-related resources.
    /// This will create one framebuffer for every in-flight frame.
    /// Imageless framebuffers are not supported yet.
    /// Called by `VulkanEngine::recreate_swapchain`.
    pub unsafe fn new(instance: &Instance, engine: &VulkanEngine, render_pass: RenderPass) -> Self {
        let swapchain_loader = Swapchain::new(instance, &engine.device);

        let present_modes = engine
            .surface_loader
            .get_physical_device_surface_present_modes(engine.physical_device, engine.surface)
            .unwrap();
        let present_mode = present_modes
            .iter()
            .find(|mode| **mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(&vk::PresentModeKHR::FIFO)
            .clone();

        info!("{:?}", present_mode);

        event!(
            Level::DEBUG,
            "image extent in SwapchainManger::new = {:?}",
            engine.surface_resolution
        );
        // We might want to check if present and graphics queue are the same... might need to use concurrent sharing mode here
        // However, no current hardware seems to support only one of the two but not both.
        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(engine.surface)
            .min_image_count(engine.desired_swapchain_image_count)
            .image_color_space(engine.surface_format.color_space)
            .image_format(engine.surface_format.format)
            .image_extent(engine.surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(engine.surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1);

        let swapchain = swapchain_loader
            .create_swapchain(&swapchain_create_info, None)
            .unwrap();

        let present_images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
        let present_image_views: Vec<vk::ImageView> = present_images
            .iter()
            .map(|&image| {
                let create_view_info = vk::ImageViewCreateInfo::builder()
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .format(engine.surface_format.format)
                    .components(vk::ComponentMapping {
                        r: vk::ComponentSwizzle::R,
                        g: vk::ComponentSwizzle::G,
                        b: vk::ComponentSwizzle::B,
                        a: vk::ComponentSwizzle::A,
                    })
                    .subresource_range(vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: 1,
                        base_array_layer: 0,
                        layer_count: 1,
                    })
                    .image(image);
                engine
                    .device
                    .create_image_view(&create_view_info, None)
                    .unwrap()
            })
            .collect();

        let framebuffers = present_image_views
            .iter()
            .map(|present_image_view| {
                let attachmments = [*present_image_view];
                let framebuffer_create_info = vk::FramebufferCreateInfo::builder()
                    .attachments(&attachmments)
                    .render_pass(render_pass)
                    .width(engine.surface_resolution.width)
                    .height(engine.surface_resolution.height)
                    .layers(1);
                engine
                    .device
                    .create_framebuffer(&framebuffer_create_info, None)
                    .unwrap()
            })
            .collect::<Vec<_>>();

        let swapchain_data = framebuffers
            .iter()
            .enumerate()
            .map(|(i, _)| PerSwapchainImage {
                present_image: present_images[i],
                present_image_view: present_image_views[i],
                framebuffer: framebuffers[i],
            })
            .collect();

        Self {
            swapchain_loader,
            swapchain,
            swapchain_data,
        }
    }
}

impl SwapchainManager {
    /// Frees all resources held by `self`.
    pub unsafe fn destroy(&mut self, device: &Device) {
        for swapchain_data in &mut self.swapchain_data {
            swapchain_data.destroy(device);
        }
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None);
    }
}

/// Data held by `SwapchainManager`.
pub struct PerSwapchainImage {
    pub present_image: vk::Image,
    pub present_image_view: vk::ImageView,
    pub framebuffer: vk::Framebuffer,
}

impl PerSwapchainImage {
    /// Frees the resources held by `self`.
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_framebuffer(self.framebuffer, None);
        device.destroy_image_view(self.present_image_view, None);
    }
}
