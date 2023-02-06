#![cfg_attr(target_arch = "spirv", no_std)]
#![feature(asm_experimental_arch)]

mod typed_buffer;

pub mod vert;
pub mod frag;