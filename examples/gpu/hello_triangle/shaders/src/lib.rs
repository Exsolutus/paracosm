#![cfg_attr(target_arch = "spirv", no_std)]
#![deny(warnings)]

use spirv_std::{glam, spirv, RuntimeArray, TypedBuffer, arch::set_mesh_outputs_ext};
use glam::{
    vec4, Vec4, UVec3
};

use hello_triangle_shared::{PushConstant, Vertex};


#[spirv(fragment)]
pub fn main_fs(
    color: Vec4, 
    output: &mut Vec4
) {
    *output = color.clone();
}

#[spirv(mesh_ext(
    threads(1),
    output_vertices = 3,
    output_primitives_ext = 1,
    output_triangles_ext
))]
pub fn main_ms(
    #[spirv(push_constant)] push_constant: &PushConstant,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] storage_buffers: &mut RuntimeArray<TypedBuffer<[Vertex]>>,
    #[spirv(position)] positions: &mut [Vec4; 3],
    #[spirv(primitive_triangle_indices_ext)] indices: &mut [UVec3; 1],
    colors: &mut [Vec4; 3]
) {
    let vertices = unsafe { storage_buffers.index_mut(push_constant.vertex_data as usize) };

    unsafe {
        set_mesh_outputs_ext(3, 1);
    }

    *positions = [
        vec4(
            vertices[0].position[0],
            vertices[0].position[1],
            vertices[0].position[2],
            1.0
        ),
        vec4(
            vertices[1].position[0],
            vertices[1].position[1],
            vertices[1].position[2],
            1.0
        ),
        vec4(
            vertices[2].position[0],
            vertices[2].position[1],
            vertices[2].position[2],
            1.0
        ),
    ];

    *indices = [
        UVec3::new(0, 1, 2)
    ];

    *colors = [
        vec4(
            vertices[0].color[0],
            vertices[0].color[1],
            vertices[0].color[2],
            1.0
        ),
        vec4(
            vertices[1].color[0],
            vertices[1].color[1],
            vertices[1].color[2],
            1.0
        ),
        vec4(
            vertices[2].color[0],
            vertices[2].color[1],
            vertices[2].color[2],
            1.0
        ),
    ]
}
