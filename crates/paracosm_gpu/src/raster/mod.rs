use crate::Device;
use crate::Surface;

use ash::util;
use ash::vk;
use ash::vk::Extent2D;

use std::{
    ffi::CStr,
    fs::File,
    path::Path,
    slice
};

pub struct RasterPipeline {
    device: Device,

    shader_modules: Vec<vk::ShaderModule>,

    pub pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout
}

impl RasterPipeline {
    pub fn new(device: Device, vertex_path: &Path, fragment_path: &Path) -> Result<Self, String> {
        let extent = Extent2D::builder()
            .width(1920)
            .height(1080)
            .build(); //surface.extent()?;
        let format = vk::Format::B8G8R8A8_UNORM; //surface.format()?;
        
        // Create shader modules
        let vertex_module = Self::create_shader_module(&device, vertex_path)?;
        let fragment_module = Self::create_shader_module(&device, fragment_path)?;

        let entry_name = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(vertex_module)
                .name(entry_name)
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(fragment_module)
                .name(entry_name)
                .build()
        ];

        // Create fixed function infos
        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder();

        let viewports = [
            vk::Viewport::builder()
                .width(extent.width as f32)
                .height(extent.height as f32)
                .build()
        ];
        let scissors = [extent.into()];
        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissors(&scissors)
            .viewports(&viewports);

        let input_assembly_state_create_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false);

        let rasterization_state_create_info = vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0);

            let multisample_state_create_info = vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachment_states = [
            vk::PipelineColorBlendAttachmentState::builder()
                .blend_enable(false)
                .src_color_blend_factor(vk::BlendFactor::SRC_COLOR)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_DST_COLOR)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ZERO)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .build()
        ];
        let color_blend_state_create_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(&color_blend_attachment_states);

        let create_info = vk::PipelineLayoutCreateInfo::builder();
        let pipeline_layout = unsafe {
            match device.create_pipeline_layout(&create_info, None) {
                Ok(result) => result,
                Err(_) => return Err("Failed to create pipeline layout!".to_string())
            }
        };

        let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(slice::from_ref(&format));

        // Create pipeline
        let create_info = vk::GraphicsPipelineCreateInfo::builder()
            .push_next(&mut pipeline_rendering_create_info)
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_create_info)
            .input_assembly_state(&input_assembly_state_create_info)
            .viewport_state(&viewport_state_create_info)
            .rasterization_state(&rasterization_state_create_info)
            .multisample_state(&multisample_state_create_info)
            .color_blend_state(&color_blend_state_create_info)
            .layout(pipeline_layout);
        let pipeline = unsafe {
            match device.create_graphics_pipelines(vk::PipelineCache::null(), slice::from_ref(&create_info), None) {
                Ok(result) => result,
                Err(_) => return Err("Failed to create pipeline!".to_string())
            }
        }[0];

        Ok(Self {
            device,
            shader_modules: vec![vertex_module, fragment_module],
            pipeline,
            pipeline_layout
        })
    }

    fn create_shader_module(device: &Device, path: &Path) -> Result<vk::ShaderModule, String> {
        let mut file = match File::open(path) {
            Ok(result) => result,
            Err(error) => return Err(format!("Failed to open shader file {}\nError: {}", path.to_str().unwrap(), error))
        };
        let code = match util::read_spv(&mut file) {
            Ok(result) => result,
            Err(error) => return Err(format!("Failed to read shader file {}\nError: {}", path.to_str().unwrap(), error))
        };
        let create_info = vk::ShaderModuleCreateInfo::builder()
            .code(&code);
        let shader_module = unsafe {
            match device.create_shader_module(&create_info, None) {
                Ok(result) => result,
                Err(error) => return Err(format!("Failed to create shader module from file {}\nError: {}", path.to_str().unwrap(), error))
            }
        };

        Ok(shader_module)
    }
}

impl Drop for RasterPipeline {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_shader_module(self.shader_modules[0], None);
            self.device.destroy_shader_module(self.shader_modules[1], None);
        }
    }
}