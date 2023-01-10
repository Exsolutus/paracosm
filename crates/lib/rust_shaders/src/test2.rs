#![cfg_attr(target_arch = "spirv", no_std)]
#![feature(asm_experimental_arch)]
// compile-flags: -C target-feature=+RuntimeDescriptorArray,+ext:SPV_EXT_descriptor_indexing

use core::arch::asm;
use core::ops::Deref;

use spirv_std::spirv;
use spirv_std::{glam, RuntimeArray};

#[repr(C)]
pub struct Buffer<T>(RuntimeArray<T>);

impl<T> Deref for Buffer<T> {
    type Target = RuntimeArray<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


#[spirv(fragment)]
pub fn main(
    frag_color: &mut glam::Vec4,
) {
    unsafe {
        let mut buffers: *mut RuntimeArray<Buffer<u32>>;
        
        asm!(
            "OpDecorate {1} DescriptorSet 0",
            "OpDecorate {1} Binding 0",
            "%_runtimearr_Buffer        = OpTypeRuntimeArray {0}",
            "%_ptr_runtimearr_Buffer    = OpTypePointer Generic %_runtimearr_Buffer",
            "{1}                        = OpVariable &_ptr_runtimearr_Buffer StorageBuffer",
            sym Buffer::<u32>,
            out(reg) buffers
        );

        let value = *(*buffers).index(0).index(0) as f32;
        *frag_color = glam::Vec4::from((value, value, value, value));
    }
}