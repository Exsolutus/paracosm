#![cfg_attr(target_arch = "spirv", no_std)]
#![deny(warnings)]

#[repr(C)]
pub struct PushConstant {
    pub descriptor_index: u32
}