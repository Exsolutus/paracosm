use crate::typed_buffer::TypedBuffer;

use glam::{Vec2, Vec3, Vec4};
use spirv_std::{
    glam,
    RuntimeArray,
    spirv,
};

use rust_shaders_shared::{
    ObjectData,
    ShaderConstants,
};

#[spirv(vertex)]
pub fn main(
    // Input Parameters
    #[spirv(push_constant)] constants: &ShaderConstants,
    #[spirv(descriptor_set = 0, binding = 0, storage_buffer)] storage_buffers: &RuntimeArray<TypedBuffer<[ObjectData]>>,
    in_position: Vec3,
    in_normal: Vec3,
    in_color: Vec3,
    in_tex_coord: Vec2,
    #[spirv(instance_index)] instance_index: u32,
    // Output Parameters
    #[spirv(position)] out_pos: &mut Vec4,
    out_color: &mut Vec4,
    out_tex_coord: &mut Vec2
) {
    let model_matrix = unsafe { storage_buffers.index(constants.object_buffer_handle.index() as usize)[instance_index as usize].model_matrix };
    *out_pos = constants.camera_matrix * model_matrix * Vec4::from((in_position, 1.0));
    *out_color = Vec4::from((in_color, 0.0));
    *out_tex_coord = in_tex_coord;
}