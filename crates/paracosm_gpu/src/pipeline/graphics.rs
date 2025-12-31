use crate::device::LogicalDevice;
use super::ShaderModule;

use anyhow::{bail, Result};
use ash::vk::Extent2D;

use std::{
    ffi::CString, str::FromStr,
};

pub use ash::vk::{
    Viewport,
    PolygonMode,
    CullModeFlags as CullMode,
    FrontFace,
    SampleCountFlags as SampleCount,
    CompareOp,
    StencilOpState,
    BlendFactor,
    BlendOp,
    ColorComponentFlags as ColorComponent
};



pub struct ViewportInfo {
    pub viewports: Box<[ash::vk::Viewport]>,
    pub scissors: Box<[ash::vk::Rect2D]>
}

impl Default for ViewportInfo {
    fn default() -> Self {
        Self {
            viewports: [
                ash::vk::Viewport::default()
                    .width(1.0)
                    .height(1.0)
            ].into(),
            scissors: [
                ash::vk::Rect2D::default()
                    .extent(Extent2D { width: 1, height: 1 })
            ].into()
        }
    }
}

pub struct RasterizationInfo {
    pub depth_clamp_enable: bool,
    pub rasterizer_discard_enable: bool,
    pub polygon_mode: PolygonMode,
    pub cull_mode: CullMode,
    pub front_face: FrontFace,
    pub depth_bias_enable: bool,
    pub depth_bias_constant_factor: f32,
    pub depth_bias_clamp: f32,
    pub depth_bias_slope_factor: f32,
    pub line_width: f32
}

pub struct MultisampleInfo {
    pub rasterization_samples: SampleCount,
    pub sample_shading_enable: bool,
    pub min_sample_shading: f32,
    pub sample_mask: Box<[ash::vk::SampleMask]>,
    pub alpha_to_coverage_enable: bool,
    pub alpha_to_one_enable: bool
}

pub struct DepthTestInfo {
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: CompareOp,
}

pub struct ColorBlendInfo {
    source_color_blend_factor: BlendFactor,
    destination_color_blend_factor: BlendFactor,
    color_blend_op: BlendOp,
    source_alpha_blend_factor: BlendFactor,
    destination_alpha_blend_factor: BlendFactor,
    alpha_blend_op: BlendOp,
    color_write_mask: ColorComponent
}

impl Default for ColorBlendInfo {
    fn default() -> Self {
        Self {
            source_color_blend_factor: BlendFactor::ONE,
            destination_color_blend_factor: BlendFactor::ZERO,
            color_blend_op: BlendOp::ADD,
            source_alpha_blend_factor: BlendFactor::ONE,
            destination_alpha_blend_factor: BlendFactor::ZERO,
            alpha_blend_op: BlendOp::ADD,
            color_write_mask: ColorComponent::RGBA
        }
    }
}

#[derive(Default)]
pub struct AttachmentInfo {
    pub color_attachments: Box<[(ash::vk::Format, Option<ColorBlendInfo>)]>,
    pub depth_format: ash::vk::Format,
    pub stencil_format: ash::vk::Format
}

pub struct DynamicStateInfo {
    
}

pub(crate) fn create_graphics_pipeline(
    device: &LogicalDevice,
    task_shader: Option<(&ShaderModule, &'static str)>,
    mesh_shader: (&ShaderModule, &'static str),
    fragment_shader: (&ShaderModule, &'static str),
    viewport: &ViewportInfo,
    rasterization: &RasterizationInfo,
    multisample: &MultisampleInfo,
    depth_test: &DepthTestInfo,
    attachment: &AttachmentInfo,
    pipeline_layout: ash::vk::PipelineLayout,
    #[cfg(debug_assertions)] debug_name: &'static str
) -> Result<ash::vk::Pipeline> {
    let mesh_entry_point = CString::from_str(mesh_shader.1)?;
    let fragment_entry_point = CString::from_str(fragment_shader.1)?;

    let mut stages = vec![
        ash::vk::PipelineShaderStageCreateInfo::default()
            .module(**mesh_shader.0)
            .stage(ash::vk::ShaderStageFlags::MESH_EXT)
            .name(&mesh_entry_point),
        ash::vk::PipelineShaderStageCreateInfo::default()
            .module(**fragment_shader.0)
            .stage(ash::vk::ShaderStageFlags::FRAGMENT)
            .name(&fragment_entry_point)
    ];

    let task_entry_point;
    if let Some(task_shader) = task_shader {
        task_entry_point = CString::from_str(task_shader.1)?;
        stages.push(
            ash::vk::PipelineShaderStageCreateInfo::default()
                .module(**task_shader.0)
                .stage(ash::vk::ShaderStageFlags::TASK_EXT)
                .name(&task_entry_point)
        );
    }

    let viewport_state = ash::vk::PipelineViewportStateCreateInfo::default()
        .viewports(&viewport.viewports)
        .scissors(&viewport.scissors);
    let rasterization_state = ash::vk::PipelineRasterizationStateCreateInfo::default()
        .depth_clamp_enable(rasterization.depth_clamp_enable)
        .rasterizer_discard_enable(rasterization.rasterizer_discard_enable)
        .polygon_mode(rasterization.polygon_mode)
        .cull_mode(rasterization.cull_mode)
        .front_face(rasterization.front_face)
        .depth_bias_enable(rasterization.depth_bias_enable)
        .depth_bias_constant_factor(rasterization.depth_bias_constant_factor)
        .depth_bias_clamp(rasterization.depth_bias_clamp)
        .depth_bias_slope_factor(rasterization.depth_bias_slope_factor)
        .line_width(rasterization.line_width);
    let multisample_state = ash::vk::PipelineMultisampleStateCreateInfo::default()
        .rasterization_samples(multisample.rasterization_samples)
        .sample_shading_enable(multisample.sample_shading_enable)
        .min_sample_shading(multisample.min_sample_shading)
        .sample_mask(&multisample.sample_mask)
        .alpha_to_coverage_enable(multisample.alpha_to_coverage_enable)
        .alpha_to_one_enable(multisample.alpha_to_one_enable);
    let depth_stencil_state = ash::vk::PipelineDepthStencilStateCreateInfo::default()
        .depth_test_enable(depth_test.depth_test_enable)
        .depth_write_enable(depth_test.depth_write_enable)
        .depth_compare_op(depth_test.depth_compare_op);

    let color_blend_attachments = attachment.color_attachments.iter()
        .filter_map(|(_, blend)| {
            match blend {
                Some(blend) => Some(ash::vk::PipelineColorBlendAttachmentState::default()
                    .blend_enable(true)
                    .src_color_blend_factor(blend.source_color_blend_factor)
                    .dst_color_blend_factor(blend.destination_color_blend_factor)
                    .color_blend_op(blend.color_blend_op)
                    .src_alpha_blend_factor(blend.source_alpha_blend_factor)
                    .dst_alpha_blend_factor(blend.destination_alpha_blend_factor)
                    .alpha_blend_op(blend.alpha_blend_op)
                    .color_write_mask(blend.color_write_mask)),
                None => None
            }
        })
        .collect::<Box<[_]>>();
    let color_blend_state = ash::vk::PipelineColorBlendStateCreateInfo::default()
        .attachments(&color_blend_attachments);

    let dynamic_states = vec![
        ash::vk::DynamicState::VIEWPORT,
        ash::vk::DynamicState::SCISSOR
    ];
    let dynamic_state = ash::vk::PipelineDynamicStateCreateInfo::default()
        .dynamic_states(&dynamic_states);

    let color_formats = &attachment.color_attachments.iter()
        .map(|(format, _)| *format )
        .collect::<Box<[_]>>();
    let mut pipeline_rendering_create_info = ash::vk::PipelineRenderingCreateInfo::default()
        .view_mask(0)
        .color_attachment_formats(&color_formats)
        .depth_attachment_format(attachment.depth_format)
        .stencil_attachment_format(attachment.stencil_format);

    let graphics_pipeline_create_info = [
        ash::vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .depth_stencil_state(&depth_stencil_state)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state)
            .layout(pipeline_layout)
            .push_next(&mut pipeline_rendering_create_info)
    ];
    let pipeline = unsafe {
        match device.create_graphics_pipelines(
            ash::vk::PipelineCache::null(), 
            &graphics_pipeline_create_info, 
            None
        ) {
            Ok(result) => result[0],
            Err((_, error)) => bail!(error)
        }
    };

    #[cfg(debug_assertions)]
    unsafe {
        let pipeline_name = CString::new(format!("Graphics Pipeline: {}", debug_name))?;
        let pipeline_name_info = ash::vk::DebugUtilsObjectNameInfoEXT::default()
            .object_handle(pipeline)
            .object_name(&pipeline_name);
        device.debug_utils.set_debug_utils_object_name(&pipeline_name_info)?;
    }

    Ok(pipeline)
}
