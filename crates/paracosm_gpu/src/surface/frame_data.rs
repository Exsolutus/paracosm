use super::Device;

use ash::vk;


pub(super) struct FrameData {
    device: Device,

    pub present_semaphore: vk::Semaphore,
    pub render_semaphore: vk::Semaphore,
    pub render_fence: vk::Fence,

    command_pool: vk::CommandPool,
    pub command_buffer: vk::CommandBuffer
}

impl FrameData {
    pub fn new(device: Device) -> Result<Self, String> {
        // Create sync structures
        let create_info = vk::SemaphoreCreateInfo::builder();
        let present_semaphore = match unsafe { device.create_semaphore(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(format!("FrameData::new: {}", error.to_string()))
        };
        let render_semaphore = match unsafe { device.create_semaphore(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(format!("FrameData::new: {}", error.to_string()))
        };
        
        let create_info = vk::FenceCreateInfo::builder()
            .flags(vk::FenceCreateFlags::SIGNALED);
        let render_fence = match unsafe { device.create_fence(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(format!("FrameData::new: {}", error.to_string()))
        };

        // Create graphics command pool
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(device.queues.graphics_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let command_pool = match unsafe { device.create_command_pool(&create_info, None) } {
            Ok(result) => result,
            Err(error) => return Err(format!("FrameData::new: {}", error.to_string()))
        };

        // Create graphics command buffer
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffer = match unsafe { device.allocate_command_buffers(&alloc_info) } {
            Ok(result) => result[0],
            Err(error) => return Err(format!("FrameData::new: {}", error.to_string()))
        };

        Ok(Self {
            device,
            present_semaphore,
            render_semaphore,
            render_fence,
            command_pool,
            command_buffer
        })
    }
}

impl Drop for FrameData {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.device.destroy_command_pool(self.command_pool, None);

            self.device.destroy_semaphore(self.present_semaphore, None);
            self.device.destroy_semaphore(self.render_semaphore, None);
            self.device.destroy_fence(self.render_fence, None);
        }
    }
}