#![cfg_attr(target_arch = "spirv", no_std)]


use glam::{Vec4};
use spirv_std::{glam, spirv};

use rust_shaders_shared::{
    ShaderConstants, 
    // Vertex
};

#[spirv(vertex)]
pub fn main_vs(
    #[spirv(push_constant)] constants: &ShaderConstants,
    //in_vertex: Vertex,
    in_position: Vec4,
    in_normal: Vec4,
    in_color: Vec4,
    #[spirv(position)] out_pos: &mut Vec4,
    out_color: &mut Vec4
) {
    // *out_pos = constants.render_matrix * in_vertex.position;
    // *out_color = in_vertex.color;
    *out_pos = constants.render_matrix * in_position;
    *out_color = in_color;
}

#[spirv(fragment)]
pub fn main_fs(
    #[spirv(push_constant)] constants: &ShaderConstants,
    frag_color: Vec4,
    out_color: &mut Vec4,
) {
    *out_color = frag_color;
}