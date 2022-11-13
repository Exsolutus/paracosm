mod vertex;
pub use vertex::Vertex;

use crate::{resource::Buffer, device::Device};

use anyhow::{Result, bail};
use ash::vk;
use bevy_log::prelude::*;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;


pub struct Mesh {
    device: Device,
    vertices: Vec<Vertex>,
    vertex_buffer: Buffer,
}

impl Mesh {
    pub fn new(device: Device, vertices: Vec<Vertex>) -> Result<Self> {
        let size = (size_of::<Vertex>() * vertices.len()) as u64;

        // Create staging buffer
        let create_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::CONCURRENT)
            .queue_family_indices(&[device.queues.graphics_family, device.queues.transfer_family])
            .build();
        let staging_buffer = device.create_buffer("Staging Buffer", create_info, gpu_allocator::MemoryLocation::CpuToGpu)?;

        // Copy data to staging buffer
        unsafe {
            let allocation = staging_buffer.allocation.borrow();
            let memory = match allocation.mapped_ptr() {
                Some(value) => value.as_ptr(),
                None => bail!("Failed to get allocation memory")
            };
            memcpy(vertices.as_ptr(), memory.cast(), vertices.len());
        }

        // Create GPU vertex buffer
        let create_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::CONCURRENT)
            .queue_family_indices(&[device.queues.graphics_family, device.queues.transfer_family])
            .build();
        let vertex_buffer = device.create_buffer("Vertex Buffer", create_info, gpu_allocator::MemoryLocation::GpuOnly)?;

        // Copy data from staging buffer to GPU vertex buffer
        device.copy_buffer(&staging_buffer, &vertex_buffer, size)?;

        // Cleanup
        device.destroy_buffer(&staging_buffer)?;
        
        Ok(Self {
            device,
            vertices,
            vertex_buffer
        })
    }

    pub fn vertex_buffer(&self) -> &vk::Buffer {
        &self.vertex_buffer.buffer
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        info!("Dropping Mesh!");

        self.device.destroy_buffer(&self.vertex_buffer).unwrap();
    }
}