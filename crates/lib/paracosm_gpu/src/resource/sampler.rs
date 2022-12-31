use crate::device::Device;

use anyhow::Result;
use ash::vk;

// re-export
pub use vk::{
    Filter,
    SamplerAddressMode,
    BorderColor,
    CompareOp,
    SamplerMipmapMode
};

pub struct SamplerInfo {
    pub filter: (Filter, Filter),
    pub address_mode: (SamplerAddressMode, SamplerAddressMode, SamplerAddressMode),
    pub anisotropy: Option<f32>,
    pub border_color: BorderColor,
    pub unnormalized_coordinates: bool,
    pub compare_op: Option<CompareOp>,
    pub mipmap_mode: SamplerMipmapMode,
    pub mipmap_lod: (f32, f32, f32)
}


pub struct Sampler {
    device: Device,
    sampler: vk::Sampler
}

impl Drop for Sampler {
    fn drop(&mut self) {
        unsafe {
            // TODO: look into waiting on queue idle instead
            self.device.device_wait_idle().unwrap();

            self.device.destroy_sampler(self.sampler, None);
        }
    }
}

impl Device {
    pub fn create_sampler(
        &self,
        info: SamplerInfo
    ) -> Result<Sampler> {
        let create_info = vk::SamplerCreateInfo::builder()
            .mag_filter(info.filter.0)
            .min_filter(info.filter.1)
            .address_mode_u(info.address_mode.0)
            .address_mode_v(info.address_mode.1)
            .address_mode_w(info.address_mode.2)
            .anisotropy_enable(info.anisotropy.is_some())
            .max_anisotropy(info.anisotropy.unwrap_or(0.0))
            .border_color(info.border_color)
            .unnormalized_coordinates(info.unnormalized_coordinates)
            .compare_enable(info.compare_op.is_some())
            .compare_op(info.compare_op.unwrap_or(CompareOp::ALWAYS))
            .mipmap_mode(info.mipmap_mode)
            .mip_lod_bias(info.mipmap_lod.0)
            .min_lod(info.mipmap_lod.1)
            .max_lod(info.mipmap_lod.2);

        let sampler = unsafe { self.logical_device.create_sampler(&create_info, None)? };

        Ok(Sampler {
            device: self.clone(),
            sampler
        })
    }
}