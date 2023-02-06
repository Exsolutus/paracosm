
use glam::{Vec2, Vec4};
use spirv_std::{
    glam,
    spirv,
};

use rust_shaders_shared::{
    ShaderConstants,
};

#[spirv(fragment)]
pub fn main(
    #[spirv(push_constant)] constants: &ShaderConstants,
    frag_color: Vec4,
    frag_tex_coord: Vec2,
    out_color: &mut Vec4,
) {
    *out_color = frag_color;
}
