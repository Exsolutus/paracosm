use crate::device::Device;
use crate::resource::shader_module::ShaderModule;

use anyhow::{bail, Result};
use ash::vk;
use bevy_ecs::system::Resource;
use bevy_log::prelude::*;

use std::{
    borrow::Cow,
    ffi::CStr,
    slice, 
};

// Reexport
pub use vk::{
    PipelineColorBlendAttachmentState,
    BlendFactor,
    BlendOp,
    ColorComponentFlags,
    Format,
    PipelineInputAssemblyStateCreateInfo,
    PrimitiveTopology,
    PipelineRasterizationStateCreateInfo,
    PolygonMode,
    CullModeFlags,
    FrontFace,
    PipelineDepthStencilStateCreateInfo,
    CompareOp,
    PipelineMultisampleStateCreateInfo,
    SampleCountFlags,
    PipelineLayout
};



/// A [`GraphicsPipeline`] containing shader stages, resource bindings, and vertex information.
/// 
/// Created by calling [`Device::create_graphics_pipeline`].
#[derive(Clone, Resource)]
pub struct GraphicsPipeline {
    device: Device,
    pub pipeline: vk::Pipeline,
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        info!("Dropping GraphicsPipeline");
        unsafe {
            self.device.device_wait_idle().unwrap();
            
            self.device.destroy_pipeline(self.pipeline, None);
        }
    }
}

// TODO: implement pipeline for Compute shaders
#[derive(Clone, Resource)]
pub struct ComputePipeline {
    device: Device
}



/// Describes the shader stages, resource bindings, vertex input, and fixed function state of a graphics pipeline.
pub struct GraphicsPipelineInfo {
    pub vertex_stage_info: VertexStageInfo,
    pub fragment_stage_info: FragmentStageInfo,
    // TODO: Refactor to hide ash::vk
    pub input_assembly_state: vk::PipelineInputAssemblyStateCreateInfo,
    pub rasterization_state: vk::PipelineRasterizationStateCreateInfo,
    pub depth_stencil_state: Option<vk::PipelineDepthStencilStateCreateInfo>,
    pub multisample_state: vk::PipelineMultisampleStateCreateInfo,
}

// TODO: Refactor to hide ash::vk
pub struct VertexStageInfo {
    pub shader: ShaderModule,
    pub entry_point: Cow<'static, str>,
    pub vertex_input_desc: VertexInputDescription
}

// TODO: Refactor to hide ash::vk
pub struct VertexInputDescription {
    pub binding_description: vk::VertexInputBindingDescription,
    pub attribute_descriptions: Vec<vk::VertexInputAttributeDescription>
}

// TODO: Refactor to hide ash::vk
pub struct FragmentStageInfo {
    pub shader: ShaderModule,
    pub entry_point: Cow<'static, str>,
    pub color_blend_states: Vec<vk::PipelineColorBlendAttachmentState>,
    pub target_states: Vec<vk::Format>
}



// Implement pipeline creation
impl Device {
    /// Create a new [`GraphicsPipeline`] from [`GraphicsPipelineInfo`]
    pub fn create_graphics_pipeline(
        &self,
        info: GraphicsPipelineInfo,
        layout: vk::PipelineLayout
    ) -> Result<GraphicsPipeline> {
        // Create shader stage infos
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(info.vertex_stage_info.shader.module)
                .name(unsafe { CStr::from_bytes_with_nul_unchecked(info.vertex_stage_info.entry_point.as_bytes()) })
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(info.fragment_stage_info.shader.module)
                .name(unsafe { CStr::from_bytes_with_nul_unchecked(info.fragment_stage_info.entry_point.as_bytes()) })
                .build()
        ];

        // Create vertex input state info
        let vertex_input_state_create_info = vk::PipelineVertexInputStateCreateInfo::builder()
            .vertex_binding_descriptions(slice::from_ref(&info.vertex_stage_info.vertex_input_desc.binding_description))
            .vertex_attribute_descriptions(info.vertex_stage_info.vertex_input_desc.attribute_descriptions.as_slice());

        // Create dynamic state infos
        let viewport_state_create_info = vk::PipelineViewportStateCreateInfo::builder()
            .scissor_count(1)
            .viewport_count(1);
        let dynamic_states = [
            vk::DynamicState::VIEWPORT,
            vk::DynamicState::SCISSOR
        ];
        let dynamic_state_create_info = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&dynamic_states);

        // Create fixed function infos
        let input_assembly_state_create_info = info.input_assembly_state;
        let rasterization_state_create_info = info.rasterization_state;
        let multisample_state_create_info = info.multisample_state;

        // Create attachment state infos
        let color_blend_attachment_states = info.fragment_stage_info.color_blend_states.as_slice();
        let color_blend_state_create_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(color_blend_attachment_states);
        let depth_stencil_state_create_info = info.depth_stencil_state.unwrap();
        let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(info.fragment_stage_info.target_states.as_slice())
            .depth_attachment_format(vk::Format::D24_UNORM_S8_UINT);



        // Create pipeline
        let create_info = vk::GraphicsPipelineCreateInfo::builder()
            .push_next(&mut pipeline_rendering_create_info)
            .stages(&shader_stage_create_infos)
            .vertex_input_state(&vertex_input_state_create_info)
            .input_assembly_state(&input_assembly_state_create_info)
            .viewport_state(&viewport_state_create_info)
            .dynamic_state(&dynamic_state_create_info)
            .rasterization_state(&rasterization_state_create_info)
            .multisample_state(&multisample_state_create_info)
            .color_blend_state(&color_blend_state_create_info)
            .depth_stencil_state(&depth_stencil_state_create_info)
            .layout(layout);
        let pipeline = unsafe {
            match self.create_graphics_pipelines(vk::PipelineCache::null(), slice::from_ref(&create_info), None) {
                Ok(result) => result,
                Err(_) => bail!("Failed to create pipeline!".to_string())
            }
        }[0];

        Ok(GraphicsPipeline {
            device: self.clone(),
            pipeline,
        })
    }
}