use crate::device::Device;

use anyhow::{Result, anyhow};
use ash::vk;

use gpu_allocator::vulkan::{
    Allocation,
    AllocationCreateDesc
};

use std::mem;


#[derive(Debug)]
pub struct Buffer {
    pub(crate) buffer: vk::Buffer,
    pub(crate) allocation: Allocation,
}

impl Device {
    pub fn create_buffer(
        &self, 
        name: &str, 
        info: vk::BufferCreateInfo
    ) -> Result<Buffer> {
        let buffer = unsafe { self.logical_device.create_buffer(&info, None)? };
        let requirements = unsafe { self.get_buffer_memory_requirements(buffer) };
        // let memory_properties = unsafe { self.instance.get_physical_device_memory_properties(self.physical_device) };

        // let memory_type_index = (0..memory_properties.memory_type_count)
        //     .find(|&i| {
        //         let is_suitable = (requirements.memory_type_bits & (1 << i)) != 0;
        //         let memory_type = memory_properties.memory_types[i as usize];
        //         is_suitable && memory_type.property_flags.contains(vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE)
        //     })
        //     .ok_or_else(|| anyhow!("Failed to find suitable memory type!"));

        let allocation = self.allocator
            .lock()
            .unwrap()
            .allocate(&AllocationCreateDesc {
                name,
                requirements,
                location: gpu_allocator::MemoryLocation::CpuToGpu,
                linear: true
            })?;

        unsafe { self.bind_buffer_memory(buffer, allocation.memory(), allocation.offset())? };

        Ok(Buffer {
            buffer,
            allocation
        })
    }

    pub fn destroy_buffer(
        &mut self, 
        buffer: &Buffer
    ) -> Result<()> {
        unsafe {
            self.logical_device.destroy_buffer(buffer.buffer, None);
            // self.allocator
            //     .lock()
            //     .unwrap()
            //     .free(buffer.allocation)?;
        }

        Ok(())
    }
}