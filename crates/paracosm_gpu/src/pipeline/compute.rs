use crate::device::LogicalDevice;

use anyhow::{bail, Result};

use std::{
    fs::File,
    ffi::CString
};


pub fn create_compute_pipeline(
    device: &LogicalDevice,
    shader_module: &crate::pipeline::ShaderModule, 
    entry_point: &'static str,
    pipeline_layout: &ash::vk::PipelineLayout,
    #[cfg(debug_assertions)] debug_name: &'static str
) -> Result<ash::vk::Pipeline> {
    let spv_file = &mut File::open(shader_module.spv_path.clone())?;
    let spirv = &ash::util::read_spv(spv_file)?;
    let mut shader_module_create_info = ash::vk::ShaderModuleCreateInfo::default()
        .code(&spirv);

    let entry_point = CString::new(entry_point)?;

    let compute_pipeline_create_info = ash::vk::ComputePipelineCreateInfo::default()
        .stage(ash::vk::PipelineShaderStageCreateInfo::default()
            .push_next(&mut shader_module_create_info)
            .stage(ash::vk::ShaderStageFlags::COMPUTE)
            .name(&entry_point)
        )
        .layout(*pipeline_layout);
    let pipeline = unsafe {
        match device.create_compute_pipelines(
            ash::vk::PipelineCache::null(), 
            std::slice::from_ref(&compute_pipeline_create_info), 
            None
        ) {
            Ok(result) => result[0],
            Err((_, error)) => bail!(error)
        }
    };

    #[cfg(debug_assertions)]
    unsafe {
        let pipeline_name = CString::new(format!("Compute Pipeline: {}", debug_name))?;
        let pipeline_name_info = ash::vk::DebugUtilsObjectNameInfoEXT::default()
            .object_handle(pipeline)
            .object_name(&pipeline_name);
        device.debug_utils.set_debug_utils_object_name(&pipeline_name_info)?;
    }

    Ok(pipeline)
}