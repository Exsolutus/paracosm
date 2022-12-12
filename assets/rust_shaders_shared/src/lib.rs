#![cfg_attr(target_arch = "spirv", no_std)]

// Rust-SpirV shared source

pub use spirv_std::glam::{Mat4, Vec4};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ShaderConstants {
    pub render_matrix: Mat4
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: Vec4,
    pub normal: Vec4,
    pub color: Vec4,
}

// Rust only source

#[cfg(not(target_arch = "spirv"))] use std::mem::size_of;
#[cfg(not(target_arch = "spirv"))] use ash::vk;

#[cfg(not(target_arch = "spirv"))]
impl Vertex {
    pub fn new(position: Vec4, normal: Vec4, color: Vec4) -> Self {
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
            .offset(size_of::<Vec4>() as u32)
            .build();
        let color = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(2 * size_of::<Vec4>() as u32)
            .build();

        [position, normal, color]
    }
}
