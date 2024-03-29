use crate::device::Device;

use anyhow::{Context, Result};
use ash::vk;


pub struct FrameData {
    device: Device,
    // Frame sync
    pub(crate) render_semaphore: vk::Semaphore,
    pub(crate) in_flight_fence: vk::Fence,
    // Frame commands
    command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer,
}

impl FrameData {
    pub fn new(device: Device) -> Result<Self> {
        // Create sync structures
        let create_info = vk::SemaphoreCreateInfo::builder();
        let render_semaphore = unsafe { 
            device.create_semaphore(&create_info, None)
                .context("FrameData::new: ")?
        };
        
        let create_info = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED);
        let in_flight_fence = unsafe {
            device.create_fence(&create_info, None)
                .context("FrameData::new: ")?
        };

        // Create graphics command pool
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(device.queues.graphics_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = unsafe {
            device.create_command_pool(&create_info, None)
                .context("FrameData::new: ")?
        };

        // Create graphics command buffer
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffer = unsafe {
            device.allocate_command_buffers(&alloc_info)
                .context("FrameData::new: ")?[0]
        };

        Ok(Self {
            device,
            render_semaphore,
            in_flight_fence,
            command_pool,
            command_buffer,
        })
    }
}

impl Drop for FrameData {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.device.destroy_command_pool(self.command_pool, None);

            self.device.destroy_semaphore(self.render_semaphore, None);
            self.device.destroy_fence(self.in_flight_fence, None);
        }
    }
}