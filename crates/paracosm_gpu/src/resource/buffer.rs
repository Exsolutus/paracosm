use crate::device::Device;

use anyhow::Result;
use ash::vk;

use gpu_allocator::{vulkan::{
    Allocation,
    AllocationCreateDesc
}, MemoryLocation};

use std::{slice, cell::RefCell};



pub use vk::{BufferCreateInfo, BufferUsageFlags, SharingMode};

#[derive(Debug)]
pub struct Buffer {
    pub(crate) buffer: vk::Buffer,
    pub(crate) allocation: RefCell<Allocation>,
}

impl Device {
    pub fn create_buffer(
        &self, 
        name: &str, 
        info: vk::BufferCreateInfo,
        location: MemoryLocation
    ) -> Result<Buffer> {
        let buffer = unsafe { self.logical_device.create_buffer(&info, None)? };
        let requirements = unsafe { self.get_buffer_memory_requirements(buffer) };

        let allocation = self.allocator
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .allocate(&AllocationCreateDesc {
                name,
                requirements,
                location,
                linear: true
            })?;

        unsafe { self.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())? };

        Ok(Buffer {
            buffer,
            allocation: RefCell::new(allocation)
        })
    }

    pub fn copy_buffer(
        &self,
        source: &Buffer,
        destination: &Buffer,
        size: vk::DeviceSize
    ) -> Result<()> {
        // Create temporary transfer command buffer
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.transfer_pool)
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY);
        let command_buffer = unsafe { self.allocate_command_buffers(&alloc_info)?[0] };

        // Record commands for data transfer
        unsafe {
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.begin_command_buffer(command_buffer, &begin_info)?;

            let regions = vk::BufferCopy::builder().size(size);
            self.cmd_copy_buffer(command_buffer, source.buffer, destination.buffer, slice::from_ref(&regions));

            self.end_command_buffer(command_buffer)?;
        }

        // Execute transfer command buffer
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(slice::from_ref(&command_buffer))
            .build();
        unsafe { 
            self.queue_submit(*self.transfer_queue, slice::from_ref(&submit_info), vk::Fence::null())?;
            self.queue_wait_idle(*self.transfer_queue)?;
        }

        // Cleanup
        unsafe {
            self.free_command_buffers(self.transfer_pool, &[command_buffer]);
        }
        

        Ok(())
    }

    pub fn destroy_buffer(
        &self, 
        buffer: &Buffer
    ) -> Result<()> {
        unsafe {
            self.allocator
                .as_ref()
                .unwrap()
                .lock()
                .unwrap()
                .free(buffer.allocation.take())?;
            self.logical_device.destroy_buffer(buffer.buffer, None);
        }

        Ok(())
    }
}