use vk::{DependencyFlags, PipelineStageFlags};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};

#[cfg(target_os = "windows")]
use ash::extensions::khr::Win32Surface;
use ash::vk;
use std::default::Default;


use ash::vk::{
    DeviceSize, ImageViewCreateInfo, IndexType, MemoryPropertyFlags, PipelineLayout, ShaderModule,
    SharingMode, VertexInputRate,
};
use vk_shader_macros::include_glsl;

use favilla::vk_engine::{FrameDataManager, SwapchainManager, VulkanEngine};

use cgmath::{vec2, vec4, Matrix4};

use favilla::app::{App, AppSettings};
use favilla::buffer::{StagingBufferWithDedicatedAllocation, VulkanBufferWithDedicatedAllocation};
use favilla::camera::Camera;
use favilla::cleanup_queue::CleanupQueue;

use favilla::memory::find_memorytype_index;
use favilla::push_buffer::PushBuffer;


use favilla_examples::*;

const NUM_FRAMES: u32 = 2;

const VERT: &[u32] = include_glsl!("shaders/tri.vert");
const FRAG: &[u32] = include_glsl!("shaders/tri.frag", kind: frag);
const FRAG_INVERTED: &[u32] = include_glsl!("shaders/inverted.frag", kind: frag);

fn main() -> anyhow::Result<()> {
    let window_height = 600;
    let window_width = 800;

    let event_loop = EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .with_title("StygVK - Example")
        .with_inner_size(winit::dpi::LogicalSize::new(
            f64::from(window_width),
            f64::from(window_height),
        ))
        .build(&event_loop)
        .expect("Could not build window");

    unsafe {
        let mut app = App::new(AppSettings {
            name: "Styg VK Sample",
            layer_names: &[favilla::layer_names::VK_LAYER_KHRONOS_VALIDATION],
            add_debug_utils: true,
            vk_api_version: vk::make_api_version(0, 1, 1, 0),
            extensions: ash_window::enumerate_required_extensions(&window)
                .expect("")
                .into_iter()
                .map(|n| n.to_owned())
                .collect::<_>(),
        })
        .unwrap_or_else(|err| panic!("Failed to construct app: {}", err));

        let mut vk_engine =
            VulkanEngine::new(&app, &window, NUM_FRAMES, window_width, window_height);
        let mut frame_manager = FrameDataManager::new(&vk_engine);

        let render_pass = create_render_pass(&vk_engine);
        let mut swapchain_manager = SwapchainManager::new(&app.instance, &vk_engine, render_pass);

        let mut cleanup_queue = CleanupQueue::new(vk_engine.num_frames as _);

        let mut staging_buffer_per_frame = (0..vk_engine.num_frames)
            .map(|_| {
                StagingBufferWithDedicatedAllocation::allocate(
                    &vk_engine,
                    3,
                    vk::BufferUsageFlags::TRANSFER_SRC,
                    vk::SharingMode::EXCLUSIVE,
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                )
            })
            .collect::<Vec<_>>();

        let mut vertex_buffer = VulkanBufferWithDedicatedAllocation::allocate(
            &vk_engine,
            3,
            vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER,
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let mut index_buffer =
            create_index_buffer(&vk_engine, frame_manager.command_pool, 3, |i| i);

        let mut push_buffer = PushBuffer::new(3);

        let mut exiting = false;

        let cam_buffer_size = std::mem::size_of::<Matrix4<f32>>() as u64;

        let mut cam = Camera::new(vec2(window_width as _, window_height as _), 0.0, 1.0);

        // DESCRIPTOR FUN START

        let descriptor_pool = vk_engine
            .device
            .create_descriptor_pool(
                &vk::DescriptorPoolCreateInfo::builder()
                    .max_sets(10)
                    .pool_sizes(&[
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::UNIFORM_BUFFER)
                            .descriptor_count(1)
                            .build(),
                        vk::DescriptorPoolSize::builder()
                            .ty(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                            .descriptor_count(2)
                            .build(),
                    ])
                    .build(),
                None,
            )
            .expect("Failed to allocate descriptor pool");

        let cam_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .stage_flags(vk::ShaderStageFlags::VERTEX)
            .build();

        let camera_descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(&[cam_binding])
            .build();

        let camera_descriptor_set_layout = vk_engine
            .device
            .create_descriptor_set_layout(&camera_descriptor_set_layout_create_info, None)
            .expect("Failed to allocate descriptor set layout");

        let mut camera_buffer_per_frame = vec![];
        let mut camera_descriptors_per_frame = vec![];
        for _ in 0..vk_engine.num_frames {
            // uniform buffer
            let camera_buffer = StagingBufferWithDedicatedAllocation::allocate(
                &vk_engine,
                cam_buffer_size,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::SharingMode::EXCLUSIVE,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );

            // descriptor
            let descriptor_sets = vk_engine
                .allocate_descriptor_sets(&[camera_descriptor_set_layout], descriptor_pool)
                .expect("Failed to allocate descriptor");

            let descriptor_set = descriptor_sets[0];

            vk_engine.device.update_descriptor_sets(
                &[vk::WriteDescriptorSet::builder()
                    .dst_binding(0)
                    .dst_set(descriptor_set)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .buffer_info(&[vk::DescriptorBufferInfo::builder()
                        .buffer(camera_buffer.buffer.buffer.buffer)
                        .offset(0)
                        .range(cam_buffer_size)
                        .build()])
                    .build()],
                &[],
            );

            camera_buffer_per_frame.push(camera_buffer);
            camera_descriptors_per_frame.push(descriptor_set);
        }

        // DESCRIPTOR FUN END

        // TEXTURE FUN START

        let texture_one_data = &[0xFF0000FFu32];
        let texture_two_data = &[0xFFFFFFFFu32, 0xFFFFFFFFu32, 0xFFFFFFFFu32, 0xFFFFFFFFu32];

        let image_format = vk::Format::R8G8B8A8_SRGB;
        let num_texture_array_layers = 1;

        // Image one: 1x1

        let mut image_one = favilla::texture::Texture::new(
            &vk_engine,
            image_format,
            vk::ImageType::TYPE_2D,
            favilla::texture::TextureExtent {
                width: 1,
                height: 1,
            },
            1,
        )?;
        let texture_one_data_size = std::mem::size_of_val(texture_one_data) as DeviceSize;
        let image_one_mem_req = image_one.get_memory_requirements(&vk_engine.device);

        let total_texture_size: vk::DeviceSize = 32 * 32 * 4 * 2;

        let image_memory_type_index = find_memorytype_index(
            &image_one_mem_req,
            &vk_engine.device_memory_properties,
            MemoryPropertyFlags::DEVICE_LOCAL,
        );

        let mut texture_memory_allocator = favilla::linear_allocator::LinearAllocator::new(
            &vk_engine.device,
            total_texture_size,
            image_memory_type_index,
        )
        .expect("Failed to create linear allocator");

        println!("Texture memory allocator: {:#?}", texture_memory_allocator);

        let mut image_one_staging_buffer = StagingBufferWithDedicatedAllocation::allocate(
            &vk_engine,
            texture_one_data_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            SharingMode::EXCLUSIVE,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        );
        image_one_staging_buffer.buffer.write(texture_one_data);

        let image_one_memory = texture_memory_allocator
            .allocate(image_one_mem_req)
            .expect("Failed to allocate sub memory for image one");

        println!("sub allocation for image one: {:?}", image_one_memory);

        image_one
            .bind_memory(&vk_engine, image_one_memory.memory, image_one_memory.offset)
            .expect("Failed to bind image memory");

        image_one.copy_staging_to_image(
            &vk_engine,
            frame_manager.command_pool,
            &image_one_staging_buffer.buffer,
        );

        image_one_staging_buffer.destroy(&vk_engine);

        let image_view_one = vk_engine
            .device
            .create_image_view(
                &ImageViewCreateInfo::builder()
                    .image(image_one.image)
                    .format(image_format)
                    .components(vk::ComponentMapping::default())
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(num_texture_array_layers)
                            .build(),
                    ),
                None,
            )
            .expect("Couldn't create image view.");

        // Image two: 2x2

        let mut image_two = favilla::texture::Texture::new(
            &vk_engine,
            image_format,
            vk::ImageType::TYPE_2D,
            favilla::texture::TextureExtent {
                width: 2,
                height: 2,
            },
            1,
        )?;
        let texture_two_data_size = std::mem::size_of_val(texture_two_data) as DeviceSize;
        let image_two_mem_req = image_two.get_memory_requirements(&vk_engine.device);
        let mut image_two_staging_buffer = StagingBufferWithDedicatedAllocation::allocate(
            &vk_engine,
            texture_two_data_size,
            vk::BufferUsageFlags::TRANSFER_SRC,
            SharingMode::EXCLUSIVE,
            MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
        );
        image_two_staging_buffer.buffer.write(texture_two_data);

        let image_two_memory = texture_memory_allocator
            .allocate(image_two_mem_req)
            .expect("Failed to allocate sub memory for image one");

        println!("sub allocation for image one: {:?}", image_two_memory);

        image_two
            .bind_memory(&vk_engine, image_two_memory.memory, image_two_memory.offset)
            .expect("Failed to bind image memory");

        image_two.copy_staging_to_image(
            &vk_engine,
            frame_manager.command_pool,
            &image_two_staging_buffer.buffer,
        );

        image_two_staging_buffer.destroy(&vk_engine);

        let image_view_two = vk_engine
            .device
            .create_image_view(
                &ImageViewCreateInfo::builder()
                    .image(image_two.image)
                    .format(image_format)
                    .components(vk::ComponentMapping::default())
                    .view_type(vk::ImageViewType::TYPE_2D)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(num_texture_array_layers)
                            .build(),
                    ),
                None,
            )
            .expect("Couldn't create image view.");

        let texture_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            // Two descriptors :D
            .descriptor_count(2)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
            .build();

        let texture_descriptor_set_layout_create_info =
            vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&[texture_binding])
                .build();

        let texture_descriptor_set_layout = vk_engine
            .device
            .create_descriptor_set_layout(&texture_descriptor_set_layout_create_info, None)
            .expect("Failed to allocate descriptor set layout");

        let sampler = vk_engine
            .device
            .create_sampler(
                &vk::SamplerCreateInfo::builder()
                    .mag_filter(vk::Filter::NEAREST)
                    .min_filter(vk::Filter::NEAREST)
                    .address_mode_u(vk::SamplerAddressMode::REPEAT)
                    .address_mode_v(vk::SamplerAddressMode::REPEAT)
                    .address_mode_w(vk::SamplerAddressMode::REPEAT)
                    .build(),
                None,
            )
            .expect("Failed to create image sampler");

        // texture sampler descriptor
        let texture_descriptor_sets = vk_engine
            .device
            .allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .set_layouts(&[texture_descriptor_set_layout])
                    .descriptor_pool(descriptor_pool)
                    .build(),
            )
            .expect("Failed to allocate descriptor");

        let texture_descriptor_set = texture_descriptor_sets[0];

        vk_engine.device.update_descriptor_sets(
            &[vk::WriteDescriptorSet::builder()
                .dst_binding(0)
                .dst_set(texture_descriptor_set)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .image_info(&[
                    // Texture one
                    vk::DescriptorImageInfo::builder()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .sampler(sampler)
                        .image_view(image_view_one)
                        .build(),
                    // Texture two
                    vk::DescriptorImageInfo::builder()
                        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .sampler(sampler)
                        .image_view(image_view_two)
                        .build(),
                ])
                .build()],
            &[],
        );

        println!("all texture stuff done");
        // TEXTURE FUN END

        let vertex_shader_info = vk::ShaderModuleCreateInfo::builder().code(VERT);
        let vertex_shader = vk_engine
            .device
            .create_shader_module(&vertex_shader_info, None)
            .unwrap();

        let fragment_shader_info = vk::ShaderModuleCreateInfo::builder().code(FRAG);
        let fragment_shader = vk_engine
            .device
            .create_shader_module(&fragment_shader_info, None)
            .unwrap();

        let pipeline_layout = vk_engine
            .device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&[camera_descriptor_set_layout, texture_descriptor_set_layout])
                    .build(),
                None,
            )
            .unwrap();

        let graphics_pipeline = create_graphics_pipeline(
            &vk_engine,
            render_pass,
            vertex_shader,
            fragment_shader,
            pipeline_layout,
        );

        let inverted_fragment_shader_info =
            vk::ShaderModuleCreateInfo::builder().code(FRAG_INVERTED);
        let inverted_fragment_shader = vk_engine
            .device
            .create_shader_module(&inverted_fragment_shader_info, None)
            .unwrap();

        let inverted_graphics_pipeline = create_graphics_pipeline(
            &vk_engine,
            render_pass,
            vertex_shader,
            inverted_fragment_shader,
            pipeline_layout,
        );

        let mut recreate_swapchain = false;

        event_loop.run(move |event, _, control_flow| {
            // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
            // dispatched any events. This is ideal for games and similar applications.
            *control_flow = ControlFlow::Poll;

            // ControlFlow::Wait pauses the event loop if no events are available to process.
            // This is ideal for non-game applications that only update in response to user
            // input, and uses significantly less power/CPU time than ControlFlow::Poll.
            //*control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::Resized(new_size),
                    ..
                } => {
                    recreate_swapchain = true;
                    cam.set_extent(vec2(new_size.width as _, new_size.height as _));
                }
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    println!("Exiting");
                    exiting = true;
                    *control_flow = ControlFlow::Exit;

                    vk_engine.device.device_wait_idle().unwrap();

                    vk_engine.device.destroy_image_view(image_view_one, None);
                    image_one.destroy(&vk_engine.device);

                    vk_engine.device.destroy_image_view(image_view_two, None);
                    image_two.destroy(&vk_engine.device);

                    texture_memory_allocator.destroy(&vk_engine.device);

                    index_buffer.destroy(&vk_engine);

                    for staging_buffer in &mut staging_buffer_per_frame {
                        staging_buffer.destroy(&vk_engine);
                    }
                    vertex_buffer.destroy(&vk_engine);

                    cleanup_queue.destroy(&vk_engine.device);

                    vk_engine.device.destroy_shader_module(vertex_shader, None);
                    vk_engine
                        .device
                        .destroy_shader_module(fragment_shader, None);
                    vk_engine
                        .device
                        .destroy_shader_module(inverted_fragment_shader, None);

                    vk_engine
                        .device
                        .destroy_pipeline_layout(pipeline_layout, None);

                    vk_engine
                        .device
                        .destroy_descriptor_pool(descriptor_pool, None);

                    for buffer in &mut camera_buffer_per_frame {
                        buffer.destroy(&vk_engine);
                    }

                    vk_engine
                        .device
                        .destroy_descriptor_set_layout(camera_descriptor_set_layout, None);

                    vk_engine
                        .device
                        .destroy_descriptor_set_layout(texture_descriptor_set_layout, None);

                    vk_engine.device.destroy_sampler(sampler, None);

                    vk_engine.device.destroy_pipeline(graphics_pipeline, None);
                    vk_engine
                        .device
                        .destroy_pipeline(inverted_graphics_pipeline, None);

                    swapchain_manager.destroy(&vk_engine.device);

                    vk_engine.device.destroy_render_pass(render_pass, None);

                    frame_manager.destroy(&vk_engine.device);
                    vk_engine.destroy();
                    app.destroy();
                }
                Event::MainEventsCleared => {
                    if exiting {
                        return;
                    }
                    // Application update code.

                    let frame = vk_engine.current_frame;

                    // DRAW STUFF

                    let current_frame_data = &mut frame_manager.frame_data[frame as usize];

                    let image_acquired_semaphore = current_frame_data.image_acquired_semaphore;
                    let render_complete_semaphore = current_frame_data.render_complete_semaphore;

                    let present_index = loop {
                        let mut recreate = recreate_swapchain;

                        if !recreate {
                            match swapchain_manager.swapchain_loader.acquire_next_image(
                                swapchain_manager.swapchain,
                                u64::MAX,
                                image_acquired_semaphore,
                                vk::Fence::null(),
                            ) {
                                Ok((present_index, _)) => break present_index,
                                Err(vk::Result::ERROR_OUT_OF_DATE_KHR)
                                | Err(vk::Result::SUBOPTIMAL_KHR) => {
                                    recreate = true;
                                }
                                Err(e) => {
                                    panic!("Unexpected error during image acquisition: {:?}", e);
                                }
                            }
                        }

                        if recreate {
                            recreate_swapchain = false;

                            vk_engine.recreate_swapchain(
                                &app.instance,
                                vk::Extent2D {
                                    width: window.inner_size().width,
                                    height: window.inner_size().height,
                                },
                                &mut swapchain_manager,
                                render_pass,
                            );
                        }
                    };

                    let current_swapchain_data =
                        &swapchain_manager.swapchain_data[present_index as usize];

                    let fence = &current_frame_data.frame_fence;
                    vk_engine
                        .device
                        .wait_for_fences(&[*fence], true, u64::MAX)
                        .unwrap();

                    vk_engine.device.reset_fences(&[*fence]).unwrap();

                    cleanup_queue.tick(&vk_engine.device);

                    let command_buffer = current_frame_data.command_buffer;
                    vk_engine
                        .device
                        .reset_command_buffer(
                            command_buffer,
                            vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                        )
                        .unwrap();

                    vk_engine
                        .device
                        .begin_command_buffer(
                            command_buffer,
                            &vk::CommandBufferBeginInfo {
                                flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
                                ..Default::default()
                            },
                        )
                        .unwrap();

                    let vertices = [
                        Vertex {
                            position: vec2(100., 100.),
                            colour: vec4(1.0, 0.0, 0.0, 1.0),
                            tex_coords: vec2(0.5, 1.0),
                        },
                        Vertex {
                            position: vec2(200., 0.),
                            colour: vec4(0.0, 1.0, 0.0, 1.0),
                            tex_coords: vec2(0.0, 0.0),
                        },
                        Vertex {
                            position: vec2(0., 0.),
                            colour: vec4(0.0, 0.0, 1.0, 1.0),
                            tex_coords: vec2(1.0, 0.0),
                        },
                        Vertex {
                            position: vec2(200., 150.),
                            colour: vec4(1.0, 0.0, 0.0, 1.0),
                            tex_coords: vec2(0.5, 1.0),
                        },
                        Vertex {
                            position: vec2(300., 50.),
                            colour: vec4(0.0, 1.0, 0.0, 1.0),
                            tex_coords: vec2(0.0, 0.0),
                        },
                        Vertex {
                            position: vec2(100., 50.),
                            colour: vec4(0.0, 0.0, 1.0, 1.0),
                            tex_coords: vec2(1.0, 0.0),
                        },
                    ];

                    let mut push_buffer_pass = push_buffer.start_pass(0).unwrap();

                    for v in &vertices {
                        push_buffer_pass.push(*v);
                    }

                    push_buffer_pass.finish();

                    let staging_buffer = &mut staging_buffer_per_frame[frame as usize];

                    if staging_buffer.buffer.buffer.length < push_buffer.data.len() as _ {
                        let new_staging_buffer = StagingBufferWithDedicatedAllocation::allocate(
                            &vk_engine,
                            push_buffer.capacity() as _,
                            vk::BufferUsageFlags::TRANSFER_SRC,
                            vk::SharingMode::EXCLUSIVE,
                            vk::MemoryPropertyFlags::HOST_VISIBLE
                                | vk::MemoryPropertyFlags::HOST_COHERENT,
                        );

                        println!(
                            "old staging buffer memory: {:?}, new staging buffer memory: {:?}",
                            staging_buffer.memory, new_staging_buffer.memory
                        );

                        let old_staging_buffer =
                            std::mem::replace(staging_buffer, new_staging_buffer);
                        println!("replaced staging buffer");
                        cleanup_queue.queue(old_staging_buffer);
                    }

                    if vertex_buffer.buffer.length < push_buffer.data.len() as _ {
                        let new_vertex_buffer = VulkanBufferWithDedicatedAllocation::allocate(
                            &vk_engine,
                            push_buffer.capacity() as _,
                            vk::BufferUsageFlags::TRANSFER_DST
                                | vk::BufferUsageFlags::VERTEX_BUFFER,
                            vk::SharingMode::EXCLUSIVE,
                            vk::MemoryPropertyFlags::DEVICE_LOCAL,
                        );

                        let old_vertex_buffer =
                            std::mem::replace(&mut vertex_buffer, new_vertex_buffer);
                        println!("replaced vertex buffer");
                        cleanup_queue.queue(old_vertex_buffer);
                    }

                    if index_buffer.buffer.length < push_buffer.data.len() as _ {
                        let new_index_buffer = create_index_buffer(
                            &vk_engine,
                            frame_manager.command_pool,
                            push_buffer.capacity() as _,
                            |i| i,
                        );

                        let old_index_buffer =
                            std::mem::replace(&mut index_buffer, new_index_buffer);
                        println!("replaced index buffer");
                        cleanup_queue.queue(old_index_buffer);
                    }

                    staging_buffer.buffer.write(&push_buffer.data);

                    // Execution barrier *before* copying from the staging buffer to the vertex buffer:
                    // the vertex buffer might still be used by last frame's rendering process.

                    vk_engine.device.cmd_pipeline_barrier(
                        command_buffer,
                        PipelineStageFlags::VERTEX_INPUT, // Finish previous frame's vertex shader
                        PipelineStageFlags::TRANSFER,     // Wait before uploading new vertices
                        DependencyFlags::empty(),
                        &[],
                        &[],
                        &[],
                    );

                    staging_buffer
                        .buffer
                        .buffer
                        .copy(
                            &vk_engine,
                            command_buffer,
                            &mut vertex_buffer.buffer,
                            0,
                            0,
                            push_buffer.data.len() as _,
                        )
                        .unwrap();

                    // Update camera buffer (not strictly necessary since the camera is completely static right now)
                    let camera_buffer = &mut camera_buffer_per_frame[frame as usize];
                    camera_buffer.buffer.write(&[cam.view_projection_matrix()]);

                    let memory_barrier_transfer_render = vk::MemoryBarrier::builder()
                        .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                        .dst_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ)
                        .build();

                    vk_engine.device.cmd_pipeline_barrier(
                        command_buffer,
                        PipelineStageFlags::TRANSFER, // Finish vertex upload
                        PipelineStageFlags::VERTEX_INPUT, // Wait before vertices are used
                        DependencyFlags::empty(),
                        &[memory_barrier_transfer_render],
                        &[],
                        &[],
                    );

                    let clear_values = [vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 0.0],
                        },
                    }];

                    let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
                        .render_pass(render_pass)
                        .framebuffer(current_swapchain_data.framebuffer)
                        .render_area(vk::Rect2D {
                            offset: vk::Offset2D { x: 0, y: 0 },
                            extent: vk_engine.surface_resolution,
                        })
                        .clear_values(&clear_values);

                    vk_engine.device.cmd_begin_render_pass(
                        command_buffer,
                        &render_pass_begin_info,
                        vk::SubpassContents::INLINE,
                    );

                    vk_engine.device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        graphics_pipeline,
                    );

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
                    vk_engine
                        .device
                        .cmd_set_viewport(command_buffer, 0, &viewports);

                    vk_engine
                        .device
                        .cmd_set_scissor(command_buffer, 0, &scissors);

                    let camera_descriptor_set = camera_descriptors_per_frame[frame as usize];
                    vk_engine.device.cmd_bind_descriptor_sets(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        pipeline_layout,
                        0,
                        &[camera_descriptor_set, texture_descriptor_set],
                        &[],
                    );

                    vk_engine.device.cmd_bind_vertex_buffers(
                        command_buffer,
                        0,
                        &[vertex_buffer.buffer.buffer],
                        &[0],
                    );

                    vk_engine.device.cmd_bind_index_buffer(
                        command_buffer,
                        index_buffer.buffer.buffer,
                        0,
                        IndexType::UINT32,
                    );

                    vk_engine
                        .device
                        .cmd_draw_indexed(command_buffer, 3, 1, 0, 0, 0);

                    vk_engine.device.cmd_bind_pipeline(
                        command_buffer,
                        vk::PipelineBindPoint::GRAPHICS,
                        inverted_graphics_pipeline,
                    );

                    vk_engine
                        .device
                        .cmd_draw_indexed(command_buffer, 3, 1, 3, 0, 0);

                    vk_engine.device.cmd_end_render_pass(command_buffer);
                    vk_engine.device.end_command_buffer(command_buffer).unwrap();

                    let submit_info = vk::SubmitInfo::builder()
                        .command_buffers(&[command_buffer])
                        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                        .wait_semaphores(&[image_acquired_semaphore])
                        .signal_semaphores(&[render_complete_semaphore])
                        .build();
                    vk_engine
                        .device
                        .queue_submit(vk_engine.present_queue, &[submit_info], *fence)
                        .expect("Queue submit failed");

                    let wait_semaphores = [render_complete_semaphore];
                    let swapchains = [swapchain_manager.swapchain];
                    let image_indices = [present_index];

                    let present_info = vk::PresentInfoKHR::builder()
                        .wait_semaphores(&wait_semaphores) // &base.rendering_complete_semaphore)
                        .swapchains(&swapchains)
                        .image_indices(&image_indices);

                    match swapchain_manager
                        .swapchain_loader
                        .queue_present(vk_engine.present_queue, &present_info)
                    {
                        Ok(false) => {}
                        Ok(true)
                        | Err(vk::Result::ERROR_OUT_OF_DATE_KHR)
                        | Err(vk::Result::SUBOPTIMAL_KHR) => {
                            recreate_swapchain = true;
                        }
                        Err(e) => {
                            panic!("Unexpected error during presentation: {:?}", e);
                        }
                    }
                    vk_engine.advance_frame();
                }
                _ => {}
            }
        })
    }
}
