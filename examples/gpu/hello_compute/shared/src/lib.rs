#![cfg_attr(target_arch = "spirv", no_std)]
// HACK(eddyb) can't easily see warnings otherwise from `spirv-builder` builds.
#![deny(warnings)]

#[repr(C)]
pub struct PushConstant {
    pub descriptor_index: u32
}