use crate::device::Device;

use anyhow::{bail, Result};
use ash::vk;
use ash::util;

use std::{
    borrow::Cow,
    fs::File,
    path::Path
};



pub struct Shader {
    device: Device,
    pub(crate) module: vk::ShaderModule,
}

pub enum Source {
    SprirV(Cow<'static, [u32]>)
}


impl Device {
    pub fn create_shader_module(
        &self,
        path: &Path
    ) -> Result<vk::ShaderModule> {
        let mut file = match File::open(path) {
            Ok(result) => result,
            Err(error) => bail!("Failed to open shader file {}\nError: {}", path.to_str().unwrap(), error)
        };
        let code = match util::read_spv(&mut file) {
            Ok(result) => result,
            Err(error) => bail!("Failed to read shader file {}\nError: {}", path.to_str().unwrap(), error)
        };
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(&code);
        let shader_module = unsafe {
            match self.logical_device.create_shader_module(&create_info, None) {
                Ok(result) => result,
                Err(error) => bail!("Failed to create shader module from file {}\nError: {}", path.to_str().unwrap(), error)
            }
        };

        Ok(shader_module)
    }
}
