use ash::extensions::khr::Surface;
use ash::vk;
use ash::vk::PhysicalDevice;
use ash::vk::{PipelineLayout, ShaderModule, VertexInputRate};
use cgmath::{Vector2, Vector4, Zero};
use favilla::buffer::{StagingBufferWithDedicatedAllocation, VulkanBufferWithDedicatedAllocation};
use favilla::vk_engine::VulkanEngine;
use memoffset::offset_of;
use std::borrow::Cow;
use std::ffi::{CStr, CString};
use tracing::{event, Level};

#[derive(Debug, Copy, Clone)]
pub struct Vertex {
    pub position: Vector2<f32>,
    pub colour: Vector4<f32>,
    pub tex_coords: Vector2<f32>,
}

impl Default for Vertex {
    fn default() -> Self {
        Self {
            position: Zero::zero(),
            colour: Zero::zero(),
            tex_coords: Zero::zero(),
        }
    }
}

/// # Safety
/// Requires sufficient memory to be available.
/// Can only be called on the thread that is able to submit to the given command pool.
pub unsafe fn create_index_buffer<F>(
    vk_engine: &VulkanEngine,
    command_pool: vk::CommandPool,
    length: u32,
    f: F,
) -> VulkanBufferWithDedicatedAllocation<u32>
where
    F: Fn(u32) -> u32,
{
    let mut index_staging_buffer = StagingBufferWithDedicatedAllocation::allocate(
        vk_engine,
        length as _,
        vk::BufferUsageFlags::TRANSFER_SRC,
        vk::SharingMode::EXCLUSIVE,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    );
    let mut index_buffer = VulkanBufferWithDedicatedAllocation::allocate(
        vk_engine,
        length as _,
        vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER,
        vk::SharingMode::EXCLUSIVE,
        vk::MemoryPropertyFlags::DEVICE_LOCAL,
    );
    let index_data: Vec<u32> = (0..length).map(f).collect::<_>();
    index_staging_buffer.buffer.write(&index_data);

    vk_engine.one_time_submit(command_pool, |cmd_buffer| {
        index_staging_buffer
            .buffer
            .buffer
            .copy(
                vk_engine,
                cmd_buffer,
                &mut index_buffer.buffer,
                0,
                0,
                length as _,
            )
            .unwrap();
    });

    index_staging_buffer.destroy(&vk_engine.device);

    index_buffer
}

/// # Safety
/// Requires a valid device.
pub unsafe fn create_render_pass(vk_engine: &VulkanEngine) -> vk::RenderPass {
    let renderpass_attachments = [vk::AttachmentDescription {
        format: vk_engine.surface_format.format,
        samples: vk::SampleCountFlags::TYPE_1,
        load_op: vk::AttachmentLoadOp::CLEAR,
        store_op: vk::AttachmentStoreOp::STORE,
        final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
        ..Default::default()
    }];

    let color_attachment_refs = [vk::AttachmentReference {
        attachment: 0,
        layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
    }];

    let dependencies = [vk::SubpassDependency {
        src_subpass: vk::SUBPASS_EXTERNAL,
        src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
            | vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
        dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ..Default::default()
    }];

    let subpasses = [vk::SubpassDescription::builder()
        .color_attachments(&color_attachment_refs)
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .build()];

    let renderpass_create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&renderpass_attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    vk_engine
        .device
        .create_render_pass(&renderpass_create_info, None)
        .unwrap()
}

/// # Safety
/// Requires a valid device and appropriate shaders.
pub unsafe fn create_graphics_pipeline(
    vk_engine: &VulkanEngine,
    render_pass: vk::RenderPass,
    vertex_shader: ShaderModule,
    fragment_shader: ShaderModule,
    pipeline_layout: PipelineLayout,
) -> vk::Pipeline {
    let shader_entry_name = CString::new("main").unwrap();
    let shader_stage_create_infos = [
        vk::PipelineShaderStageCreateInfo {
            module: vertex_shader,
            p_name: shader_entry_name.as_ptr(),
            stage: vk::ShaderStageFlags::VERTEX,
            ..Default::default()
        },
        vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
            module: fragment_shader,
            p_name: shader_entry_name.as_ptr(),
            stage: vk::ShaderStageFlags::FRAGMENT,
            ..Default::default()
        },
    ];

    let vertex_input_attribute_desc = [
        vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32_SFLOAT,
            offset: offset_of!(Vertex, position) as _,
        },
        vk::VertexInputAttributeDescription {
            location: 1,
            binding: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: offset_of!(Vertex, colour) as _,
        },
        vk::VertexInputAttributeDescription {
            location: 2,
            binding: 0,
            format: vk::Format::R32G32_SFLOAT,
            offset: offset_of!(Vertex, tex_coords) as _,
        },
    ];

    let vertex_binding_descriptions = [vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(std::mem::size_of::<Vertex>() as u32)
        .input_rate(VertexInputRate::VERTEX)
        .build()];

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_attribute_descriptions(&vertex_input_attribute_desc)
        .vertex_binding_descriptions(&vertex_binding_descriptions)
        .build();

    let pipeline_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        ..Default::default()
    };

    let viewports = [vk::Viewport {
        x: 0.,
        y: 0.,
        width: vk_engine.surface_resolution.width as f32,
        height: vk_engine.surface_resolution.height as f32,
        min_depth: 0.0,
        max_depth: 1.0,
    }];
    let scissors = [vk::Rect2D {
        offset: vk::Offset2D { x: 0, y: 0 },
        extent: vk_engine.surface_resolution,
    }];
    let viewport_state_info = vk::PipelineViewportStateCreateInfo::builder()
        .scissors(&scissors)
        .viewports(&viewports);

    let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
        front_face: vk::FrontFace::COUNTER_CLOCKWISE,
        line_width: 1.0,
        polygon_mode: vk::PolygonMode::FILL,
        ..Default::default()
    };
    let multisample_state_info = vk::PipelineMultisampleStateCreateInfo {
        rasterization_samples: vk::SampleCountFlags::TYPE_1,
        ..Default::default()
    };
    let noop_stencil_state = vk::StencilOpState {
        fail_op: vk::StencilOp::KEEP,
        pass_op: vk::StencilOp::KEEP,
        depth_fail_op: vk::StencilOp::KEEP,
        compare_op: vk::CompareOp::ALWAYS,
        ..Default::default()
    };
    let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
        depth_test_enable: 1,
        depth_write_enable: 1,
        depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
        front: noop_stencil_state,
        back: noop_stencil_state,
        max_depth_bounds: 1.0,
        ..Default::default()
    };
    let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
        blend_enable: 0,
        src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
        dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ZERO,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
        color_write_mask: vk::ColorComponentFlags::R
            | vk::ColorComponentFlags::G
            | vk::ColorComponentFlags::B
            | vk::ColorComponentFlags::A,
    }];
    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op(vk::LogicOp::CLEAR)
        .attachments(&color_blend_attachment_states);

    let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
    let dynamic_state_info =
        vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_state);

    let graphic_pipeline_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&shader_stage_create_infos)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&pipeline_input_assembly_state_info)
        .viewport_state(&viewport_state_info)
        .rasterization_state(&rasterization_info)
        .multisample_state(&multisample_state_info)
        .depth_stencil_state(&depth_state_info)
        .color_blend_state(&color_blend_state)
        .dynamic_state(&dynamic_state_info)
        .layout(pipeline_layout)
        .render_pass(render_pass);

    let graphics_pipelines = vk_engine
        .device
        .create_graphics_pipelines(
            vk::PipelineCache::null(),
            &[graphic_pipeline_info.build()],
            None,
        )
        .expect("Unable to create graphics pipeline");

    graphics_pipelines[0]
}

/// # Safety
/// Requires debug callback support.
pub unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    let callback_data = *p_callback_data;
    let message_id_number: i32 = callback_data.message_id_number as i32;

    let message_id_name = if callback_data.p_message_id_name.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy()
    };

    let message = if callback_data.p_message.is_null() {
        Cow::from("")
    } else {
        CStr::from_ptr(callback_data.p_message).to_string_lossy()
    };

    event!(
        Level::DEBUG,
        "{:?}:\n{:?} [{} ({})] : {}\n",
        message_severity,
        message_type,
        message_id_name,
        &message_id_number.to_string(),
        message,
    );

    vk::FALSE
}

/// # Safety
/// The given surface has to be compatible with the given device.
pub unsafe fn find_surface_format(
    surface_loader: &Surface,
    surface: vk::SurfaceKHR,
    physical_device: PhysicalDevice,
) -> vk::SurfaceFormatKHR {
    let surface_formats = surface_loader
        .get_physical_device_surface_formats(physical_device, surface)
        .unwrap();

    event!(Level::DEBUG, "supported surface formats:");
    for surface_format in &surface_formats {
        event!(Level::DEBUG, "{:?}", surface_format);
    }

    surface_formats
        .into_iter()
        .next()
        .expect("Unable to find suitable surface format.")
}
