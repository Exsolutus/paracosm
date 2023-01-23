use glam::{Vec4, Vec3};
use spirv_std::{
    glam,
    RuntimeArray,
    spirv,
};

use rust_shaders_shared::{
    ObjectData,
    ShaderConstants,
    typed_buffer::TypedBuffer
    // Vertex
};

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(push_constant)] constants: &ShaderConstants,
    #[spirv(descriptor_set = 0, binding = 0, storage_buffer)] storage_buffers: &RuntimeArray<TypedBuffer<[ObjectData]>>,
    in_position: Vec3,
    in_normal: Vec3,
    in_color: Vec3,
    #[spirv(instance_index)] instance_index: u32,
    #[spirv(position)] out_pos: &mut Vec4,
    out_color: &mut Vec4
) {
    let model_matrix = unsafe { storage_buffers.index(0)[instance_index as usize].model_matrix };
    *out_pos = constants.camera_matrix * model_matrix * Vec4::from((in_position, 1.0));
    *out_color = Vec4::from((in_normal, 0.0));
}

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(push_constant)] constants: &ShaderConstants,
    frag_color: Vec4,
    #[spirv(frag_coord)] frag_coord: Vec4,
    out_color: &mut Vec4,
) {
    *out_color = frag_color;
}
