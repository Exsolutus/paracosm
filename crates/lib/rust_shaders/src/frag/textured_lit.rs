use glam::{Vec2, Vec4};
use spirv_std::{
    glam,
    spirv,
    image::*,
    Sampler,
    RuntimeArray
};

use rust_shaders_shared::{
    ShaderConstants,
    // Binding Constants
    STORAGE_BUFFER_BINDING,
    STORAGE_IMAGE_BINDING,
    SAMPLED_IMAGE_BINDING,
    SAMPLER_BINDING
};



#[spirv(fragment)]
pub fn main(
    #[spirv(push_constant)] constants: &ShaderConstants,
    frag_color: Vec4,
    frag_tex_coord: Vec2,
    out_color: &mut Vec4,
    #[spirv(descriptor_set = 0, binding = 2)] sampled_images: &RuntimeArray<Image!(
        2D,
        format = rgba32f,
        sampled
    )>,
    #[spirv(descriptor_set = 0, binding = 3)] samplers: &RuntimeArray<Sampler>
) {
    let sampler = unsafe {
        samplers.index(0)
    };
    let color: Vec4 = unsafe {
        sampled_images.index(0).sample(*sampler, frag_tex_coord)
    };
    *out_color = color; 
    //*out_color = Vec4::from((frag_tex_coord, 0.5, 0.0));
}
