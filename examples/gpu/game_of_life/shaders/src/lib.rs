#![cfg_attr(target_arch = "spirv", no_std)]
#![deny(warnings)]

use spirv_std::{glam, image::StorageImage2d, spirv, RuntimeArray};
use glam::{
    ivec2, IVec2, 
    uvec2, 
    UVec3, 
    vec4, Vec4
};

use game_of_life_shared::PushConstant;


fn hash(mut value: u32) -> u32 {
    value = value ^ 2747636419;
    value = value * 2654435769;
    value = value ^ (value >> 16);
    value = value * 2654435769;
    value = value ^ (value >> 16);
    value = value * 2654435769;

    value
}

fn random_float(value: u32) -> f32 {
    hash(value) as f32 / 4294967295.0
}

// LocalSize/numthreads of (x = 64, y = 1, z = 1)
#[spirv(compute(threads(8, 8)))]
pub fn init(
    #[spirv(global_invocation_id)]
    id: UVec3,
    #[spirv(push_constant)]
    push_constant: &PushConstant,
    #[spirv(descriptor_set = 0, binding = 1)] 
    storage_images: &RuntimeArray<StorageImage2d>,
) {
    let game = unsafe { storage_images.index(push_constant.descriptor_index as usize) };

    let location = uvec2(id.x, id.y);

    let random_number = random_float((id.y << 16) | id.x);
    let alive = (random_number > 0.9) as u32 as f32;
    let color = vec4(alive, 0., 0., 0.);

    unsafe { game.write(location, color) };
}

fn is_alive(image: &StorageImage2d, location: IVec2, offset_x: i32, offset_y: i32) -> u32 {
    let value: Vec4 = image.read::<u32>(uvec2((location.x + offset_x) as u32, (location.y + offset_y) as u32));

    value.x as u32
}

fn count_alive(image: &StorageImage2d, location: IVec2) -> u32 {
    is_alive(image, location, -1, -1) +
    is_alive(image, location, -1,  0) +
    is_alive(image, location, -1,  1) +
    is_alive(image, location,  0, -1) +
    is_alive(image, location,  0,  1) +
    is_alive(image, location,  1, -1) +
    is_alive(image, location,  1,  0) +
    is_alive(image, location,  1,  1)
}

// LocalSize/numthreads of (x = 64, y = 1, z = 1)
#[spirv(compute(threads(8, 8)))]
pub fn update(
    #[spirv(global_invocation_id)]
    id: UVec3,
    #[spirv(push_constant)]
    push_constant: &PushConstant,
    #[spirv(descriptor_set = 0, binding = 1)] 
    storage_images: &RuntimeArray<StorageImage2d>,
) {
    let game = unsafe { storage_images.index(push_constant.descriptor_index as usize) };

    let location = ivec2(id.x as i32, id.y as i32);

    let alive = match count_alive(game, location) {
        3 => 1.,
        2 => {
            is_alive(game, location, 0, 0) as f32
        },
        _ => 0.
    };
    let color = vec4(alive, 0., 0., 0.);
    
    unsafe { game.write(location, color) };
}