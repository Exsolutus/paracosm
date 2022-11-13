mod vertex;
pub use vertex::Vertex;

use crate::{resource::Buffer, device::Device};

use anyhow::Result;
use ash::vk;

use std::mem::size_of;
use std::ptr::copy_nonoverlapping as memcpy;


pub struct Mesh {
    device: Device,
    vertices: Vec<Vertex>,
    vertex_buffer: Buffer,
}

impl Mesh {
    pub fn new(device: Device, vertices: Vec<Vertex>) -> Result<Self> {
        // Create staging buffer for vertices
        let create_info = vk::BufferCreateInfo::builder()
            .size((size_of::<Vertex>() * vertices.len()) as u64)
            .usage(vk::BufferUsageFlags::VERTEX_BUFFER)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .build();
            
        let vertex_buffer = device.create_buffer("Vertex Buffer", create_info)?;

        unsafe {
            let memory = device.map_memory(vertex_buffer.allocation.memory(), 0, create_info.size, vk::MemoryMapFlags::empty())?;
            memcpy(vertices.as_ptr(), memory.cast(), vertices.len());
            device.unmap_memory(vertex_buffer.allocation.memory());
        }
        
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
        self.device.destroy_buffer(&self.vertex_buffer);
    }
}