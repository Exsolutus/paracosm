#![cfg_attr(target_arch = "spirv", no_std)]
#![deny(warnings)]


use spirv_std::{glam, spirv, RuntimeArray, TypedBuffer};
use glam::UVec3;

use hello_compute_shared::PushConstant;

// Adapted from the wgpu hello-compute example


pub fn collatz(mut n: u32) -> Option<u32> {
    let mut i = 0;
    if n == 0 {
        return None;
    }
    while n != 1 {
        n = if n % 2 == 0 {
            n / 2
        } else {
            // Overflow? (i.e. 3*n + 1 > 0xffff_ffff)
            if n >= 0x5555_5555 {
                return None;
            }
            // TODO: Use this instead when/if checked add/mul can work: n.checked_mul(3)?.checked_add(1)?
            3 * n + 1
        };
        i += 1;
    }
    Some(i)
}

// LocalSize/numthreads of (x = 64, y = 1, z = 1)
#[spirv(compute(threads(64)))]
pub fn main_cs(
    #[spirv(global_invocation_id)]
    id: UVec3,
    #[spirv(push_constant)]
    push_constant: &PushConstant,
    #[spirv(storage_buffer, descriptor_set = 0, binding = 0)] 
    storage_buffers: &mut RuntimeArray<TypedBuffer<[u32]>>,
) {
    let prime_indices = unsafe { storage_buffers.index_mut(push_constant.descriptor_index as usize) };

    let index = id.x as usize;
    prime_indices[index] = collatz(prime_indices[index]).unwrap_or(u32::MAX);
}