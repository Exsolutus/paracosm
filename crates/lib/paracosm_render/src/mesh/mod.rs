use crate::{
    render_asset::*,
    RenderContext
};

use anyhow::{Result, bail};

use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, Handle};
use bevy_ecs::{
    system::{
        lifetimeless::SRes,
        Resource,
        SystemParamItem
    }
};
use bevy_log::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_utils::{HashMap};

use paracosm_gpu::{
    resource:: buffer::*, 
};
pub use rust_shaders_shared::{
    ResourceHandle,
    Vertex,
};

use std::mem::size_of;



#[derive(Clone, Debug, Resource)]
pub struct MeshManager {
    pub meshes: HashMap<String, Handle<Mesh>>
}

/// Adds the [`Mesh`] as an asset.
pub struct MeshPlugin;

impl Plugin for MeshPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Mesh>()
            .add_plugin(RenderAssetPlugin::<Mesh>::default());

        app.insert_resource(MeshManager {
            meshes: HashMap::new()
        });
    }
}


// TODO: Split Asset and GPU resource, convert in prepare phase
#[derive(TypeUuid)]
#[uuid = "c6b21835-2c1b-431e-bf23-806a01591a7c"]
// #[derive(Resource)]
pub struct Mesh {
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
}

impl Mesh {
    pub fn new() -> Self {
        Self {
            vertices: vec![],
            indices: vec![],
        }
    }

    pub fn with_geometry(
        vertices: Vec<Vertex>,
        indices: Vec<u32>
    ) -> Self {
        Self {
            vertices,
            indices,
        }
    }

    pub fn insert_vertex(&mut self, vertex: Vertex) {
        self.vertices.push(vertex);
    }

    pub fn set_indices(&mut self, indices: Vec<u32>) {
        self.indices = indices;
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



pub struct GpuMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32
}

impl RenderAsset for Mesh {
    type PreparedAsset = GpuMesh;
    type Param = SRes<RenderContext>;

    fn prepare_asset(
        source_asset: &Self,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, crate::render_asset::PrepareAssetError> {
        let device = &param.device;

        let vertices_size = size_of::<Vertex>() * source_asset.vertices.len();
        let indices_size = size_of::<u32>() * source_asset.indices.len();

        // Create staging buffers
        let info = BufferInfo::new(vertices_size, BufferUsageFlags::TRANSFER_SRC, MemoryLocation::CpuToGpu);
        let vertex_staging_buffer = device.create_buffer("Vertex Staging Buffer", info, None);

        let info = BufferInfo::new(indices_size, BufferUsageFlags::TRANSFER_SRC, MemoryLocation::CpuToGpu);
        let index_staging_buffer = device.create_buffer("Index Staging Buffer", info, None);

        // Copy data to staging buffers
        vertex_staging_buffer.write_buffer(&source_asset.vertices);
        index_staging_buffer.write_buffer(&source_asset.indices);

        // Create GPU buffers
        let info = BufferInfo::new(
            vertices_size,
            BufferUsageFlags::TRANSFER_DST | BufferUsageFlags::VERTEX_BUFFER | BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::GpuOnly
        );
        let vertex_buffer = device.create_buffer("Vertex Buffer", info, None);

        let info = BufferInfo::new(
            indices_size,
            BufferUsageFlags::TRANSFER_DST  | BufferUsageFlags::INDEX_BUFFER | BufferUsageFlags::STORAGE_BUFFER,
            MemoryLocation::GpuOnly
        );
        let index_buffer = device.create_buffer("Index Buffer", info, None);

        // Copy from staging buffers to GPU buffers
        device.copy_buffer(&vertex_staging_buffer, &vertex_buffer, vertices_size);
        device.copy_buffer(&index_staging_buffer, &index_buffer, indices_size);

        Ok(GpuMesh {
            vertex_buffer,
            index_buffer,
            index_count: source_asset.index_count() as u32
        })
    }
}