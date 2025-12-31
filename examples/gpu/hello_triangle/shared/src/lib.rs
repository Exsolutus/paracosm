#![cfg_attr(target_arch = "spirv", no_std)]
#![deny(warnings)]


#[repr(C)]
pub struct PushConstant {
    pub vertex_data: u32
}

#[repr(C)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3]
}