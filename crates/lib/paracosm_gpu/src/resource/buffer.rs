use crate::device::Device;

use anyhow::{Result, bail};
use ash::vk;

use bevy_log::prelude::*;

use gpu_allocator::vulkan::*;

use std::slice;
use std::ptr::copy_nonoverlapping as memcpy;

// re-export
pub use vk::BufferUsageFlags;
pub use gpu_allocator::MemoryLocation;



pub struct BufferInfo {
    pub size: usize,
    pub usage: vk::BufferUsageFlags,
    pub memory_location: MemoryLocation,
    pub alignment: Option<u64>
}

//#[derive(Debug)]
pub struct Buffer {
    device: Device,
    info: BufferInfo,
    pub buffer: vk::Buffer,
    pub(crate) allocation: Option<Allocation>,
}

impl BufferInfo {
    pub fn new(size: usize, usage: BufferUsageFlags, memory_location: MemoryLocation) -> Self {
        Self {
            size,
            usage,
            memory_location,
            alignment: None
        }
    }
}

impl Buffer {
    pub fn write_buffer<T>(
        &self,
        data: &Vec<T>
    ) -> Result<()> {
        let allocation = match &self.allocation {
            Some(value) => value,
            None => bail!("Buffer has no active allocation")
        };
        let memory = match allocation.mapped_ptr() {
            Some(value) => value.as_ptr(),
            None => bail!("Buffer allocation is not host visible")
        };
        unsafe { memcpy(data.as_ptr(), memory.cast(), data.len()) };

        Ok(())
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            // TODO: look into waiting on queue idle instead
            self.device.device_wait_idle().unwrap();

            match self.allocation.take() {
                Some(value) => {
                    match self.device.allocator
                        .as_ref()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .free(value)
                    {
                        Ok(_) => (),
                        Err(error) => debug!("{}", error.to_string())
                    };
                },
                None => ()
            };

            self.device.destroy_buffer(self.buffer, None);
        }
    }
}

impl Device {
    pub fn create_buffer(
        &self, 
        name: &str, 
        info: BufferInfo,
        data: Option<&[u8]>
    ) -> Result<Buffer> {
        let create_info = &vk::BufferCreateInfo::builder()
            .size(info.size as u64)
            .usage(info.usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();

        let buffer = unsafe { self.logical_device.create_buffer(create_info, None)? };
        let requirements = unsafe { self.get_buffer_memory_requirements(buffer) };

        let allocation = self.allocator
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .allocate(&AllocationCreateDesc {
                name,
                requirements,
                location: info.memory_location,
                linear: true
            })?;

        unsafe { self.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())? };

        Ok(Buffer {
            device: self.clone(),
            info,
            buffer,
            allocation: Some(allocation)
        })
    }

    pub fn copy_buffer(
        &self,
        source: &Buffer,
        destination: &Buffer,
        size: usize
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

            let regions = vk::BufferCopy::builder().size(size as u64);
            self.cmd_copy_buffer(command_buffer, source.buffer, destination.buffer, slice::from_ref(&regions));

            self.end_command_buffer(command_buffer)?;
        }

        // Execute transfer command buffer
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(slice::from_ref(&command_buffer))
            .build();
        unsafe { 
            self.queue_submit(self.transfer_queue, slice::from_ref(&submit_info), vk::Fence::null())?;
            self.queue_wait_idle(self.transfer_queue)?;
        }

        // Cleanup
        unsafe {
            self.free_command_buffers(self.transfer_pool, &[command_buffer]);
        }
        

        Ok(())
    }
}