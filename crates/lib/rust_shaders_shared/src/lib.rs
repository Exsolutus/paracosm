#![cfg_attr(target_arch = "spirv", no_std)]

// Rust-SpirV shared source
pub use spirv_std::glam;
use glam::{Vec2, Vec3, Mat4};


pub const STORAGE_BUFFER_BINDING: u32 = 0;
pub const STORAGE_IMAGE_BINDING: u32 = 1;
pub const SAMPLED_IMAGE_BINDING: u32 = 2;
pub const SAMPLER_BINDING: u32 = 3;



/// A [`ResourceHandle`] provides access to a specific resource found in the bindless descriptor set
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(transparent)]
pub struct ResourceHandle(u32);

impl ResourceHandle {
    pub fn index(&self) -> u32 {
        self.0
    }
}

/// Global push constants for all shaders
#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub struct ShaderConstants {
    pub camera_matrix: Mat4,
    pub object_buffer_handle: ResourceHandle,
    // pub test_image_handle: ResourceHandle
}

/// Object data for instanced rendering
#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub struct ObjectData {
    pub model_matrix: Mat4
}

#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Vertex {
    pub position: Vec3,
    pub normal: Vec3,
    pub color: Vec3,
    pub uv: Vec2
}



// Rust only source
#[cfg(not(target_arch = "spirv"))] use std::mem::size_of;
#[cfg(not(target_arch = "spirv"))] use ash::vk;



#[cfg(not(target_arch = "spirv"))]
impl ResourceHandle {
    pub fn new(index: u32) -> Self {
        Self(index)
    }
}

#[cfg(not(target_arch = "spirv"))]
impl Vertex {
    pub fn new(position: Vec3, normal: Vec3, color: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            color,
            uv
        }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 4] {
        let position = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();
        let normal = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(size_of::<Vec3>() as u32)
            .build();
        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(2 * size_of::<Vec3>() as u32)
            .build();
        let uv = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(3)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(3 * size_of::<Vec3>() as u32)
            .build();

        [position, normal, color, uv]
    }
}
