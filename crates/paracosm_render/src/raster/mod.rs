use crate::window::{ExtractedWindows, WindowSurfaces};

use ash::vk;

use bevy_ecs::world::World;
use bevy_log::prelude::*;
use bevy_window::Window;

use paracosm_gpu::{Instance, Device, Surface};

use std::slice;


pub struct Renderer {
    //window: Window,

    _instance: Instance,
    pub device: Device,

    graphics_queue: vk::Queue,
    graphics_command_pool: vk::CommandPool,
    graphics_command_buffer: vk::CommandBuffer,

    render_fence: vk::Fence,
    render_semaphore: vk::Semaphore,
    pub present_semaphore: vk::Semaphore
}

impl Renderer {
    pub fn new(
        window: &Window,
        instance: Instance
    ) -> Result<Self, String> {
        // Create Device
        let device = match Device::primary(instance.clone(), Some(window)) {
            Ok(result) => result,
            Err(error) => return Err(error.to_string()),
        };
    
        // Get first Graphics queue
        let graphics_queue = match device.graphics_queue(0) {
            Ok(result) => result,
            Err(error) => return Err(error.to_string())
        };
    
        // Create sync structures
        let create_info = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED);
        let render_fence = match unsafe { device.create_fence(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(error.to_string())
        };
        let create_info = vk::SemaphoreCreateInfo::builder()
            .flags(vk::SemaphoreCreateFlags::empty());
        let render_semaphore = match unsafe { device.create_semaphore(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(error.to_string())
        };
        let present_semaphore = match unsafe { device.create_semaphore(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(error.to_string())
        };

        // Create graphics command pool
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(device.queues.graphics_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let graphics_command_pool = match unsafe { device.create_command_pool(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(error.to_string())
        };

        // Create graphics command buffer
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(graphics_command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        let graphics_command_buffer = match unsafe { device.allocate_command_buffers(&alloc_info) } {
            Ok(result) => result[0],
            Err(error) => return Err(error.to_string())
        };
    
        Ok(Self {
            //window,
            _instance,
            device,
            graphics_queue,
            graphics_command_pool,
            graphics_command_buffer,
            render_fence,
            render_semaphore,
            present_semaphore
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
                Err(error) => return error!("{}", error),
                _ => ()
            };
            match unsafe { device.reset_fences(&[renderer.render_fence]) } {
                Err(error) => return error!("{}", error),
                _ => ()
            };
    
            // Render for each active window surface
            let windows = world.resource::<ExtractedWindows>();
            for window in windows.values() {
                // Check window is configured
                if window.configured {
                    continue;
                }

                // Get surface and swapchain for window
                let surface: &Surface = match window_surfaces.surfaces.get(&window.id) {
                    Some(result) => result,
                    None => continue
                };

                if let Some(image_index) = window.swapchain_image_index {
                    // Reset command buffer
                    match unsafe { device.reset_command_buffer(main_command_buffer, vk::CommandBufferResetFlags::empty()) } {
                        Err(error) => return error!("{}", error),
                        _ => ()
                    };
    
                    // Record commands
                    let begin_info = vk::CommandBufferBeginInfo::builder()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
                    match unsafe { device.begin_command_buffer(main_command_buffer, &begin_info) } {
                        Err(error) => return error!("{}", error),
                        _ => ()
                    }
    
                    // let clear_values = [
                    //     vk::ClearValue { color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] } }
                    // ];
                    // let begin_info = vk::RenderPassBeginInfo::builder()
                    //     .render_pass(pipeline.render_pass)
                    //     .render_area(vk::Rect2D::builder()
                    //         // Leave offset default
                    //         .extent(window.extent)
                    //         .build()
                    //     )
                    //     .framebuffer(pipeline.framebuffers[image_index as usize])
                    //     .clear_values(&clear_values);
                    // unsafe { device.cmd_begin_render_pass(main_command_buffer, &begin_info, vk::SubpassContents::INLINE) };
    
                    // unsafe { device.cmd_bind_pipeline(main_command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline) };
                    // unsafe { device.cmd_draw(main_command_buffer, 3, 1, 0, 0) };
                    
                    //unsafe { device.cmd_end_render_pass(main_command_buffer) };
                    match unsafe { device.end_command_buffer(main_command_buffer) } {
                        Err(error) => return error!("{}", error),
                        _ => ()
                    };
    
                    // Submit command buffer
                    let submit_info = vk::SubmitInfo::builder()
                        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                        .wait_semaphores(&[renderer.present_semaphore])
                        .signal_semaphores(&[renderer.render_semaphore])
                        .command_buffers(&[main_command_buffer])
                        .build();
                    match unsafe { device.queue_submit(renderer.graphics_queue, &[submit_info], renderer.render_fence) } {
                        Err(error) => return error!("{}", error),
                        _ => ()
                    };
    
                    // // Present rendered image to surface
                    match surface.queue_present(renderer.graphics_queue, slice::from_ref(&image_index)) {
                        Err(error) => return error!("{}", error),
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
