use crate::device::Device;

use anyhow::Context;
use anyhow::Result;
use ash::vk;
use ash::util;

use std::{
    borrow::Cow,
    fs::File,
    ops::Deref,
    path::Path,
    sync::Arc,
};


/// Internal data for a [`ShaderModule`]
pub struct ShaderModuleInternal {
    device: Device,
    pub path: Cow<'static, Path>,
    //pub entry_points: Vec<String>,
    pub module: vk::ShaderModule
}

impl Drop for ShaderModuleInternal {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_shader_module(self.module, None);
        }
    }
}


/// A [`ShaderModule`] containing a Vulkan shader module and related information.
/// 
/// Created by calling [`Device::create_shader_module`].
#[derive(Clone)]
pub struct ShaderModule(Arc<ShaderModuleInternal>);

impl Deref for ShaderModule {
    type Target = ShaderModuleInternal;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}



impl Device {
    pub fn create_shader_module(
        &self,
        path: &Path
    ) -> Result<ShaderModule> {
        let mut file = File::open(&path).context(format!("Failed to open shader file {:?}", &path))?;
        let code = util::read_spv(&mut file).context(format!("Failed to read shader file {:?}", &path))?;

        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(&code);
        let shader_module = unsafe {
            self.logical_device.create_shader_module(&create_info, None)
                .context(format!("Failed to create shader module from file {:?}", &path))?
        };

        Ok(ShaderModule(Arc::new(ShaderModuleInternal {
            device: self.clone(),
            path: Cow::from(path.to_path_buf()),
            module: shader_module
        })))
    }
}
