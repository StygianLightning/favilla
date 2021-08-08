#[cfg(all(unix, not(target_os = "android"), not(target_os = "macos")))]
use ash::extensions::khr::{Surface, Swapchain};

#[cfg(target_os = "windows")]
use ash::extensions::khr::Win32Surface;
use ash::vk::{DescriptorSet, RenderPass, SurfaceCapabilitiesKHR};
use ash::{vk, Device, Instance};

use crate::app::App;
use crate::find_family::DeviceQueueFamilies;
use ash::prelude::VkResult;
use raw_window_handle::HasRawWindowHandle;
use std::default::Default;

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
    pub unsafe fn new(
        app: &App,
        window: &dyn HasRawWindowHandle,
        num_frames: u32,
        window_width: u32,
        window_height: u32,
    ) -> Self {
        let surface = ash_window::create_surface(&app.entry, &app.instance, window, None).unwrap();
        let DeviceQueueFamilies {
            physical_device,
            queue_family_index,
            surface_loader,
        } = crate::find_family::find(&app.entry, &app.instance, surface);

        let device_extension_names_raw = [Swapchain::name().as_ptr()];
        let features = vk::PhysicalDeviceFeatures {
            shader_clip_distance: 1,
            ..Default::default()
        };
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

        let surface_format =
            crate::surface_formats::find_surface_format(&surface_loader, surface, physical_device);

        println!("using surface format {:?}", surface_format);

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
            std::u32::MAX => vk::Extent2D {
                width: window_width,
                height: window_height,
            },
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

    pub unsafe fn allocate_memory(
        &self,
        memory_req: vk::MemoryRequirements,
        memory_type_index: u32,
    ) -> vk::DeviceMemory {
        let _buffer_allocate_info = vk::MemoryAllocateInfo {
            allocation_size: memory_req.size,
            memory_type_index,
            ..Default::default()
        };

        self.try_allocate_memory(memory_req, memory_type_index)
            .expect("Failed to allocate memory")
    }

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

    pub fn advance_frame(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.num_frames;
    }

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
            std::u32::MAX => new_extent,
            _ => self.surface_capabilities.current_extent,
        };

        swapchain_manager.destroy(&self.device);
        *swapchain_manager = SwapchainManager::new(&instance, self, render_pass);
    }

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

    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_command_pool(self.command_pool, None);
        for frame_data in &mut self.frame_data {
            frame_data.destroy(device);
        }
    }
}

pub struct PerFrameData {
    pub frame_fence: vk::Fence,
    pub command_buffer: vk::CommandBuffer,
    pub image_acquired_semaphore: vk::Semaphore,
    pub render_complete_semaphore: vk::Semaphore,
}

impl PerFrameData {
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_semaphore(self.image_acquired_semaphore, None);
        device.destroy_semaphore(self.render_complete_semaphore, None);
        device.destroy_fence(self.frame_fence, None);
    }
}

pub struct SwapchainManager {
    pub swapchain_loader: Swapchain,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_data: Vec<PerSwapchainImage>,
}

impl SwapchainManager {
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

        println!(
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
    pub unsafe fn destroy(&mut self, device: &Device) {
        for swapchain_data in &mut self.swapchain_data {
            swapchain_data.destroy(device);
        }
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None);
    }
}

pub struct PerSwapchainImage {
    pub present_image: vk::Image,
    pub present_image_view: vk::ImageView,
    pub framebuffer: vk::Framebuffer,
}

impl PerSwapchainImage {
    pub unsafe fn destroy(&mut self, device: &Device) {
        device.destroy_framebuffer(self.framebuffer, None);
        device.destroy_image_view(self.present_image_view, None);
    }
}
