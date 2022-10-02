use crate::window::{ExtractedWindows, WindowSurfaces};

use ash::vk;

use bevy_ecs::world::World;
use bevy_log::prelude::*;
use bevy_window::Window;

use paracosm_gpu::{Instance, Device, Surface, RasterPipeline};

use std::{
    path::Path,
    slice
};


pub struct Renderer {
    //window: Window,

    _instance: Instance,
    pub device: Device,

    graphics_queue: vk::Queue,
    graphics_command_pool: vk::CommandPool,
    graphics_command_buffer: vk::CommandBuffer,

    render_fence: vk::Fence,
    render_semaphore: vk::Semaphore,
    pub present_semaphore: vk::Semaphore,

    raster_pipelines: Vec<RasterPipeline>
}

impl Renderer {
    pub fn new(
        window: &Window,
        instance: Instance
    ) -> Result<Self, String> {
        // Create Device
        let device = match Device::primary(instance.clone(), Some(window)) {
            Ok(result) => result,
            Err(error) => return Err(format!("Renderer::render_system: {}", error.to_string())),
        };
    
        // Get first Graphics queue
        let graphics_queue = match device.graphics_queue(0) {
            Ok(result) => result,
            Err(error) => return Err(format!("Renderer::render_system: {}", error.to_string()))
        };
        
        // Create sync structures
        let create_info = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED);
        let render_fence = match unsafe { device.create_fence(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(format!("Renderer::render_system: {}", error.to_string()))
        };
        let create_info = vk::SemaphoreCreateInfo::builder()
            .flags(vk::SemaphoreCreateFlags::empty());
        let render_semaphore = match unsafe { device.create_semaphore(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(format!("Renderer::render_system: {}", error.to_string()))
        };
        let present_semaphore = match unsafe { device.create_semaphore(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(format!("Renderer::render_system: {}", error.to_string()))
        };

        // Create graphics command pool
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(device.queues.graphics_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let graphics_command_pool = match unsafe { device.create_command_pool(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(format!("Renderer::render_system: {}", error.to_string()))
        };

        // Create graphics command buffer
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(graphics_command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        let graphics_command_buffer = match unsafe { device.allocate_command_buffers(&alloc_info) } {
            Ok(result) => result[0],
            Err(error) => return Err(format!("Renderer::render_system: {}", error.to_string()))
        };

        // Create triangle pipeline
        let vertex_spv_path = Path::new("./shaders/colored_triangle_vert.spv");
        let fragment_spv_path = Path::new("./shaders/colored_triangle_frag.spv");
        let triangle_pipeline = match RasterPipeline::new(device.clone(), vertex_spv_path, fragment_spv_path) {
            Ok(result) => result,
            Err(error) => return Err(format!("Renderer::render_system: {}", error.to_string()))
        };
    
        Ok(Self {
            //window,
            _instance: instance,
            device,
            graphics_queue,
            graphics_command_pool,
            graphics_command_buffer,
            render_fence,
            render_semaphore,
            present_semaphore,
            raster_pipelines: vec![triangle_pipeline]
        })
    }

    pub fn render_system(world: &mut World) {
        let renderer = world.resource::<Self>();
        let window_surfaces = world.non_send_resource::<WindowSurfaces>();

        let device = &renderer.device;
        let main_command_buffer = renderer.graphics_command_buffer;
    
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
    
            // Wait for GPU to finish rendering previous frame
            match unsafe { device.wait_for_fences(slice::from_ref(&renderer.render_fence), true, 1000000000) } {
                Err(error) => return error!("Renderer::render_system: {}", error),
                _ => ()
            };
            match unsafe { device.reset_fences(&[renderer.render_fence]) } {
                Err(error) => return error!("Renderer::render_system: {}", error),
                _ => ()
            };
    
            // Render for each active window surface
            let windows = world.resource::<ExtractedWindows>();
            for (id, window) in windows.iter() {
                // Check window is configured
                if !window.configured {
                    continue;
                }

                // Get surface for window
                let surface: &Surface = match window_surfaces.surfaces.get(&window.id) {
                    Some(result) => result,
                    None => continue
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
                    match unsafe { device.reset_command_buffer(main_command_buffer, vk::CommandBufferResetFlags::empty()) } {
                        Err(error) => return error!("Renderer::render_system: {}", error),
                        _ => ()
                    };
    
                    // Record commands
                    let begin_info = vk::CommandBufferBeginInfo::builder()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                    match unsafe { device.begin_command_buffer(main_command_buffer, &begin_info) } {
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
                        main_command_buffer, 
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

                    unsafe { device.cmd_begin_rendering(main_command_buffer, &rendering_info) };

                    // Rendering commands
                    unsafe {
                        let viewports = [
                            vk::Viewport::builder()
                                .width(extent.width as f32)
                                .height(extent.height as f32)
                                .build()
                        ];
                        let scissors = [extent.into()];
                        device.cmd_set_viewport(main_command_buffer, 0, &viewports);
                        device.cmd_set_scissor(main_command_buffer, 0, &scissors);
                        device.cmd_bind_pipeline(main_command_buffer, vk::PipelineBindPoint::GRAPHICS, renderer.raster_pipelines[0].pipeline); 
                        device.cmd_draw(main_command_buffer, 3, 1, 0, 0);
                    }

                    // End rendering
                    unsafe { device.cmd_end_rendering(main_command_buffer) };

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
                        main_command_buffer, 
                        vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT, 
                        vk::PipelineStageFlags::BOTTOM_OF_PIPE, 
                        vk::DependencyFlags::empty(), &[], &[], 
                        slice::from_ref(&image_memory_barrier)
                    ) };

                    // End recording commands
                    match unsafe { device.end_command_buffer(main_command_buffer) } {
                        Err(error) => return error!("Renderer::render_system: {}", error),
                        _ => ()
                    };
    
                    // Submit command buffer
                    let submit_info = vk::SubmitInfo::builder()
                        .wait_dst_stage_mask(slice::from_ref(&vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT))
                        .wait_semaphores(slice::from_ref(&renderer.present_semaphore))
                        .signal_semaphores(slice::from_ref(&renderer.render_semaphore))
                        .command_buffers(slice::from_ref(&main_command_buffer))
                        .build();
                    match unsafe { device.queue_submit(renderer.graphics_queue, slice::from_ref(&submit_info), renderer.render_fence) } {
                        Err(error) => return error!("Renderer::render_system: {}", error),
                        _ => ()
                    };
    
                    // // Present rendered image to surface
                    match surface.queue_present(renderer.graphics_queue, slice::from_ref(&image_index)) {
                        Err(error) => return error!("Renderer::render_system: {}", error),
                        _ => ()
                    };
                }
            }
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.device.destroy_command_pool(self.graphics_command_pool, None);

            self.device.destroy_semaphore(self.present_semaphore, None);
            self.device.destroy_semaphore(self.render_semaphore, None);
            self.device.destroy_fence(self.render_fence, None);
        }
    }
}
