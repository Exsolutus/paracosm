use crate::window::{ExtractedWindows, WindowSurfaces};

use ash::vk;

use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_window::Window;

use paracosm_gpu::{instance::Instance, device::Device, surface::Surface, raster::RasterPipeline, mesh::Mesh};

use std::slice;

// Types initialized by renderer
type RendererData = (Device, vk::Queue);

pub fn initialize_renderer(
    window: &Window,
    instance: Instance
) -> Result<RendererData, String> {
    // Create Device
    let window_handle = unsafe { window.raw_window_handle().get_handle() };
    let device = match Device::primary(instance.clone(), Some(&window_handle)) {
        Ok(result) => result,
        Err(error) => return Err(format!("Renderer::render_system: {}", error.to_string())),
    };

    // Get first Graphics queue
    let graphics_queue = match device.graphics_queue(0) {
        Ok(result) => result,
        Err(error) => return Err(format!("Renderer::render_system: {}", error.to_string()))
    };

    Ok((device, graphics_queue))
}

// Renderer main loop
pub fn render_system(
    device: Res<Device>,
    queue: Res<vk::Queue>,
    pipeline: Res<RasterPipeline>,
    mesh: NonSend<Mesh>,
    windows: Res<ExtractedWindows>,
    mut window_surfaces: NonSendMut<WindowSurfaces>
) {
    {
        //let _span = info_span!("present_frames").entered();

        // // Remove ViewTarget components to ensure swap chain TextureViews are dropped.
        // // If all TextureViews aren't dropped before present, acquiring the next swap chain texture will fail.
        // let view_entities = world
        //     .query_filtered::<Entity, With<ViewTarget>>()
        //     .iter(world)
        //     .collect::<Vec<_>>();
        // for view_entity in view_entities {
        //     world.entity_mut(view_entity).remove::<ViewTarget>();
        // }

        // TODO: convert window iteration to view rendering and simultaneous presentation
        // Render for each active window surface
        for (_id, window) in windows.iter() {
            // Check window is configured
            if !window.configured {
                continue;
            }

            // Get surface for window
            let surface: &mut Surface = match window_surfaces.surfaces.get_mut(&window.id) {
                Some(result) => result,
                None => continue
            };

            // Get frame data for surface
            let frame_data = surface.frame_data();

            // Wait for current frame-in-flight
            match unsafe { device.wait_for_fences(slice::from_ref(&frame_data.in_flight_fence), true, 1000000000) } {
                Err(error) => return error!("Renderer::render_system: {}", error),
                _ => ()
            };
            match unsafe { device.reset_fences(&[frame_data.in_flight_fence]) } {
                Err(error) => return error!("Renderer::render_system: {}", error),
                _ => ()
            };

            if let Some(image_index) = window.swapchain_image_index {
                // Get swapchain image for window
                let image = match surface.image(image_index) {
                    Ok(result) => result,
                    Err(error) => return error!("Renderer::render_system: {}", error)
                };
                let extent = match surface.extent() {
                    Ok(result) => result,
                    Err(error) => return error!("Renderer::render_system: {}", error)
                };

                // Reset command buffer
                match unsafe { device.reset_command_buffer(frame_data.command_buffer, vk::CommandBufferResetFlags::empty()) } {
                    Err(error) => return error!("Renderer::render_system: {}", error),
                    _ => ()
                };

                // Record commands
                let begin_info = vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                match unsafe { device.begin_command_buffer(frame_data.command_buffer, &begin_info) } {
                    Err(error) => return error!("Renderer::render_system: {}", error),
                    _ => ()
                };

                // Image Layout to Color Attachment Optimal
                let image_memory_barrier = vk::ImageMemoryBarrier::builder()
                    .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .old_layout(vk::ImageLayout::UNDEFINED)
                    .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .image(image)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1)
                            .build()
                    );
                unsafe { device.cmd_pipeline_barrier(
                    frame_data.command_buffer, 
                    vk::PipelineStageFlags::TOP_OF_PIPE, 
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, 
                    vk::DependencyFlags::empty(), &[], &[], 
                    slice::from_ref(&image_memory_barrier)
                ) };

                // Begin rendering
                let clear_value = vk::ClearValue { 
                    color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } 
                };
                let attachment_info = match surface.attachment_info(image_index, clear_value) {
                    Ok(result) => result,
                    Err(error) => return error!("Renderer::render_system: {}", error)
                };
                let rendering_info = vk::RenderingInfo::builder()
                    .render_area(vk::Rect2D::builder()
                        // Leave offset default
                        .extent(extent)
                        .build()
                    )
                    .layer_count(1)
                    .color_attachments(slice::from_ref(&attachment_info));

                unsafe { device.cmd_begin_rendering(frame_data.command_buffer, &rendering_info) };

                // Rendering commands
                unsafe {
                    let viewports = [
                        vk::Viewport::builder()
                            .width(extent.width as f32)
                            .height(extent.height as f32)
                            .build()
                    ];
                    let scissors = [extent.into()];
                    device.cmd_set_viewport(frame_data.command_buffer, 0, &viewports);
                    device.cmd_set_scissor(frame_data.command_buffer, 0, &scissors);
                    device.cmd_bind_pipeline(frame_data.command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);
                    device.cmd_bind_vertex_buffers(frame_data.command_buffer, 0, slice::from_ref(mesh.vertex_buffer()), &[0]);
                    device.cmd_bind_index_buffer(frame_data.command_buffer, *mesh.index_buffer(), 0, vk::IndexType::UINT16);
                    device.cmd_draw_indexed(frame_data.command_buffer, mesh.index_count() as u32, 1, 0, 0, 0);
                }

                // End rendering
                unsafe { device.cmd_end_rendering(frame_data.command_buffer) };

                // Image Layout to Present Optimal
                let image_memory_barrier = vk::ImageMemoryBarrier::builder()
                    .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                    .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                    .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                    .image(image)
                    .subresource_range(
                        vk::ImageSubresourceRange::builder()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .base_mip_level(0)
                            .level_count(1)
                            .base_array_layer(0)
                            .layer_count(1)
                            .build()
                    );
                unsafe { device.cmd_pipeline_barrier(
                    frame_data.command_buffer, 
                    vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, 
                    vk::PipelineStageFlags::BOTTOM_OF_PIPE, 
                    vk::DependencyFlags::empty(), &[], &[], 
                    slice::from_ref(&image_memory_barrier)
                ) };

                // End recording commands
                match unsafe { device.end_command_buffer(frame_data.command_buffer) } {
                    Err(error) => return error!("Renderer::render_system: {}", error),
                    _ => ()
                };

                // Submit command buffer
                let submit_info = vk::SubmitInfo::builder()
                    .wait_dst_stage_mask(slice::from_ref(&vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT))
                    .wait_semaphores(slice::from_ref(&surface.swapchain_semaphore))
                    .signal_semaphores(slice::from_ref(&frame_data.render_semaphore))
                    .command_buffers(slice::from_ref(&frame_data.command_buffer))
                    .build();
                match unsafe { device.queue_submit(*queue, slice::from_ref(&submit_info), frame_data.in_flight_fence) } {
                    Err(error) => return error!("Renderer::render_system: {}", error),
                    _ => ()
                };

                // // Present rendered image to surface
                match surface.queue_present(*queue, slice::from_ref(&image_index)) {
                    Err(error) => return error!("Renderer::render_system: {}", error),
                    _ => ()
                };
            }
        }
    }
}

