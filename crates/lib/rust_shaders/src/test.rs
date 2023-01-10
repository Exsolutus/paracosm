#![cfg_attr(target_arch = "spirv", no_std)]

use glam::{Vec4, Vec3, Vec4Swizzles};
use spirv_std::{glam, spirv};
use spirv_std::{self, Image, RuntimeArray, Sampler};

use rust_shaders_shared::{
    ShaderConstants, 
    // Vertex
};

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(push_constant)] constants: &ShaderConstants,
    in_position: Vec3,
    in_normal: Vec3,
    in_color: Vec3,
    #[spirv(position)] out_pos: &mut Vec4,
    out_color: &mut Vec4
) {
    *out_pos = constants.render_matrix * Vec4::from((in_position, 1.0));
    *out_color = Vec4::from((in_color, 0.0));
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
