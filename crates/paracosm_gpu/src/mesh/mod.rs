mod vertex;
pub use vertex::Vertex;

use crate::{resource::buffer::*, device::Device};

use anyhow::{Result, bail};
use ash::vk;
use bevy_log::prelude::*;
use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;
use std::slice;

pub struct Mesh {
    device: Device,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>
}

impl Mesh {
    pub fn new(
        device: Device,
        vertices: Vec<Vertex>,
        indices: Vec<u16>
    ) -> Result<Self> {
        Ok(Self {
            device,
            vertices,
            indices,
            vertex_buffer: None,
            index_buffer: None
        })
    }

    pub fn upload(&mut self) -> Result<()> {
        let vertices_size = (size_of::<Vertex>() * self.vertices.len()) as u64;
        let indices_size = (size_of::<u16>() * self.indices.len()) as u64;

        // Create staging buffers
        let create_info = vk::BufferCreateInfo::builder()
            .size(vertices_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&[self.device.queues.transfer_family])
            .build();
        let vertex_staging_buffer = self.device.create_buffer("Vertex Staging Buffer", create_info, gpu_allocator::MemoryLocation::CpuToGpu)?;

        let create_info = vk::BufferCreateInfo::builder()
            .size(indices_size)
            .usage(vk::BufferUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&[self.device.queues.transfer_family])
            .build();
        let index_staging_buffer = self.device.create_buffer("Index Staging Buffer", create_info, gpu_allocator::MemoryLocation::CpuToGpu)?;

        // Copy data to staging buffers
        unsafe {
            let memory = match vertex_staging_buffer.allocation.borrow().mapped_ptr() {
                Some(value) => value.as_ptr(),
                None => bail!("Failed to get allocation memory")
            };
            memcpy(self.vertices.as_ptr(), memory.cast(), self.vertices.len());

            let memory = match index_staging_buffer.allocation.borrow().mapped_ptr() {
                Some(value) => value.as_ptr(),
                None => bail!("Failed to get allocation memory")
            };
            memcpy(self.indices.as_ptr(), memory.cast(), self.indices.len());
        }

        // Create GPU buffers
        let create_info = vk::BufferCreateInfo::builder()
            .size(vertices_size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::CONCURRENT)
            .queue_family_indices(&[self.device.queues.graphics_family, self.device.queues.transfer_family])
            .build();
        let vertex_buffer = self.device.create_buffer("Vertex Buffer", create_info, gpu_allocator::MemoryLocation::GpuOnly)?;

        let create_info = vk::BufferCreateInfo::builder()
            .size(indices_size)
            .usage(vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::INDEX_BUFFER)
            .sharing_mode(vk::SharingMode::CONCURRENT)
            .queue_family_indices(&[self.device.queues.graphics_family, self.device.queues.transfer_family])
            .build();
        let index_buffer = self.device.create_buffer("Index Buffer", create_info, gpu_allocator::MemoryLocation::GpuOnly)?;

        // Copy from staging buffers to GPU buffers
        self.device.copy_buffer(&vertex_staging_buffer, &vertex_buffer, vertices_size)?;
        self.device.copy_buffer(&index_staging_buffer, &index_buffer, indices_size)?;

        // Cleanup
        self.device.destroy_buffer(&vertex_staging_buffer)?;
        self.device.destroy_buffer(&index_staging_buffer)?;

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);

        Ok(())
    }

    pub fn bind(&self, command_buffer: vk::CommandBuffer) -> Result<()> {
        if self.vertex_buffer.is_none() || self.index_buffer.is_none() {
            bail!("Attempt to bind mesh not uploaded to GPU");
        }

        unsafe {
            let vertex_buffer = self.vertex_buffer.as_ref().unwrap().buffer;
            let index_buffer = self.index_buffer.as_ref().unwrap().buffer;
            
            self.device.cmd_bind_vertex_buffers(command_buffer, 0, slice::from_ref(&vertex_buffer), &[0]);
            self.device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT16);
        }

        Ok(())
    }

    pub fn index_count(&self) -> usize {
        self.indices.len()
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        info!("Dropping Mesh!");

        match &self.vertex_buffer {
            Some(value) => self.device.destroy_buffer(&value).unwrap(),
            None => ()
        }
        match &self.index_buffer {
            Some(value) => self.device.destroy_buffer(&value).unwrap(),
            None => ()
        }
    }
}