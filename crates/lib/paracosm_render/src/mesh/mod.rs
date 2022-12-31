use anyhow::{Result, bail};
use ash::vk;

use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, AssetEvent, AssetLoader, Assets, AssetServer, Handle, LoadContext, LoadedAsset};
use bevy_ecs::{system::Resource};
use bevy_log::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_utils::{HashMap};

use paracosm_gpu::{resource::buffer::*, device::Device};
pub use rust_shaders_shared::Vertex;

use std::mem::size_of;
use std::slice;



#[derive(Clone, Debug, Resource)]
pub struct MeshManager {
    pub meshes: HashMap<String, Handle<Mesh>>
}

/// Adds the [`Mesh`] as an asset.
pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Mesh>();

        app.insert_resource(MeshManager {
            meshes: HashMap::new()
        });
    }
}



#[derive(TypeUuid)]
#[uuid = "c6b21835-2c1b-431e-bf23-806a01591a7c"]
// #[derive(Resource)]
pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: vec![],
            indices: vec![],
            vertex_buffer: None,
            index_buffer: None
        }
    }

    pub fn with_geometry(
        vertices: Vec<Vertex>,
        indices: Vec<u32>
    ) -> Self {
        Self {
            vertices,
            indices,
            vertex_buffer: None,
            index_buffer: None
        }
    }

    pub fn insert_vertex(&mut self, vertex: Vertex) {
        self.vertices.push(vertex);
    }

    pub fn set_indices(&mut self, indices: Vec<u32>) {
        self.indices = indices;
    }

    pub fn upload(&mut self, device: &Device) -> Result<()> {
        if self.vertex_buffer.is_some() {
            return Ok(())
        }

        let vertices_size = size_of::<Vertex>() * self.vertices.len();
        let indices_size = size_of::<u32>() * self.indices.len();

        // Create staging buffers
        let info = BufferInfo::new(vertices_size, BufferUsageFlags::TRANSFER_SRC, MemoryLocation::CpuToGpu);
        let vertex_staging_buffer = device.create_buffer("Vertex Staging Buffer", info, None)?;

        let info = BufferInfo::new(indices_size, BufferUsageFlags::TRANSFER_SRC, MemoryLocation::CpuToGpu);
        let index_staging_buffer = device.create_buffer("Index Staging Buffer", info, None)?;

        // Copy data to staging buffers
        vertex_staging_buffer.write_buffer(&self.vertices)?;
        index_staging_buffer.write_buffer(&self.indices)?;

        // Create GPU buffers
        let info = BufferInfo::new(vertices_size, BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::VERTEX_BUFFER, MemoryLocation::GpuOnly);
        let vertex_buffer = device.create_buffer("Vertex Buffer", info, None)?;

        let info = BufferInfo::new(indices_size, BufferUsageFlags::TRANSFER_DST  | BufferUsageFlags::INDEX_BUFFER, MemoryLocation::GpuOnly);
        let index_buffer = device.create_buffer("Index Buffer", info, None)?;

        // Copy from staging buffers to GPU buffers
        device.copy_buffer(&vertex_staging_buffer, &vertex_buffer, vertices_size)?;
        device.copy_buffer(&index_staging_buffer, &index_buffer, indices_size)?;

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);

        Ok(())
    }

    pub fn bind(&self, device: &Device, command_buffer: vk::CommandBuffer) -> Result<()> {
        if self.vertex_buffer.is_none() || self.index_buffer.is_none() {
            bail!("Attempt to bind mesh not uploaded to GPU");
        }

        unsafe {
            let vertex_buffer = self.vertex_buffer.as_ref().unwrap().buffer;
            let index_buffer = self.index_buffer.as_ref().unwrap().buffer;
            
            device.cmd_bind_vertex_buffers(command_buffer, 0, slice::from_ref(&vertex_buffer), &[0]);
            device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);
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
    }
}