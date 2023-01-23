#![cfg_attr(target_arch = "spirv", no_std)]
#![feature(asm_experimental_arch)]

// Rust-SpirV shared source

pub use spirv_std::glam;

use glam::{Mat4, Vec3};



/// A [`ResourceHandle`] provides access to a specific resource found in the bindless descriptor set
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[repr(transparent)]
pub struct ResourceHandle(u32);

#[cfg(not(target_arch = "spirv"))]
impl ResourceHandle {
    pub fn new(index: u32) -> Self {
        Self(index)
    }

    pub fn index(&self) -> u32 {
        self.0
    }
}



/// Global push constants for all shaders
#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub struct ShaderConstants {
    pub camera_matrix: Mat4,
    pub object_buffer_handle: ResourceHandle
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
}

// Rust only source
pub mod typed_buffer;

#[cfg(not(target_arch = "spirv"))] use std::mem::size_of;
#[cfg(not(target_arch = "spirv"))] use ash::vk;

#[cfg(not(target_arch = "spirv"))]
impl Vertex {
    pub fn new(position: Vec3, normal: Vec3, color: Vec3) -> Self {
        Self {
            position,
            normal,
            color
        }
    }

    pub fn binding_description() -> vk::VertexInputBindingDescription {
        vk::VertexInputBindingDescription::builder()
            .binding(0)
            .stride(size_of::<Self>() as u32)
            .input_rate(vk::VertexInputRate::VERTEX)
            .build()
    }

    pub fn attribute_descriptions() -> [vk::VertexInputAttributeDescription; 3] {
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

        [position, normal, color]
    }
}
