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
    indices: Vec<u16>,
    vertex_buffer: Buffer,
    index_buffer: Buffer
}

impl Mesh {
    pub fn new(
        device: Device,
        vertices: Vec<Vertex>,
        indices: Vec<u16>
    ) -> Result<Self> {
        let vertices_size = (size_of::<Vertex>() * vertices.len()) as u64;
        let indices_size = (size_of::<u16>() * indices.len()) as u64;

        // Create staging buffers
        let create_info = vk::BufferCreateInfo::builder()
            .size(vertices_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&[device.queues.transfer_family])
            .build();
        let vertex_staging_buffer = device.create_buffer("Vertex Staging Buffer", create_info, gpu_allocator::MemoryLocation::CpuToGpu)?;

        let create_info = vk::BufferCreateInfo::builder()
            .size(indices_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&[device.queues.transfer_family])
            .build();
        let index_staging_buffer = device.create_buffer("Index Staging Buffer", create_info, gpu_allocator::MemoryLocation::CpuToGpu)?;

        // Copy data to staging buffers
        unsafe {
            let memory = match vertex_staging_buffer.allocation.borrow().mapped_ptr() {
                Some(value) => value.as_ptr(),
                None => bail!("Failed to get allocation memory")
            };
            memcpy(vertices.as_ptr(), memory.cast(), vertices.len());

            let memory = match index_staging_buffer.allocation.borrow().mapped_ptr() {
                Some(value) => value.as_ptr(),
                None => bail!("Failed to get allocation memory")
            };
            memcpy(indices.as_ptr(), memory.cast(), indices.len());
        }

        // Create GPU buffers
        let create_info = vk::BufferCreateInfo::builder()
            .size(vertices_size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::CONCURRENT)
            .queue_family_indices(&[device.queues.graphics_family, device.queues.transfer_family])
            .build();
        let vertex_buffer = device.create_buffer("Vertex Buffer", create_info, gpu_allocator::MemoryLocation::GpuOnly)?;

        let create_info = vk::BufferCreateInfo::builder()
            .size(indices_size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::CONCURRENT)
            .queue_family_indices(&[device.queues.graphics_family, device.queues.transfer_family])
            .build();
        let index_buffer = device.create_buffer("Index Buffer", create_info, gpu_allocator::MemoryLocation::GpuOnly)?;

        // Copy data from staging buffers to GPU buffers
        device.copy_buffer(&vertex_staging_buffer, &vertex_buffer, vertices_size)?;
        device.copy_buffer(&index_staging_buffer, &index_buffer, indices_size)?;

        // Cleanup
        device.destroy_buffer(&vertex_staging_buffer)?;
        device.destroy_buffer(&index_staging_buffer)?;
        
        Ok(Self {
            device,
            vertices,
            indices,
            vertex_buffer,
            index_buffer
        })
    }

    pub fn vertex_buffer(&self) -> &vk::Buffer {
        &self.vertex_buffer.buffer
    }

    pub fn index_buffer(&self) -> &vk::Buffer {
        &self.index_buffer.buffer
    }

    pub fn index_count(&self) -> usize {
        self.indices.len()
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        info!("Dropping Mesh!");

        self.device.destroy_buffer(&self.vertex_buffer).unwrap();
        self.device.destroy_buffer(&self.index_buffer).unwrap();
    }
}