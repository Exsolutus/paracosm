use crate::{
    Instance,
    Device,
    Surface,
    RasterPipeline
};

use ash::vk;

use bevy_log::prelude::*;
use bevy_window::Window;

use std::{
    path::Path,
    slice
};


pub struct Renderer {
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
        let create_info = vk::SemaphoreCreateInfo::builder();
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
