use crate::device::Device;

use anyhow::{bail, Result};
use ash::vk;
use bevy_ecs::system::Resource;
use bevy_log::prelude::*;
use nalgebra_glm as glm;

use std::{
    borrow::Cow,
    ffi::CStr,
    mem::size_of,
    slice
};

/// A [`GraphicsPipeline`] containing shader stages, resource bindings, and vertex information.
/// 
/// Created by calling [`Device::create_graphics_pipeline`].
#[derive(Clone, Resource)]
pub struct GraphicsPipeline {
    device: Device,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pub pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout
}

impl Drop for GraphicsPipeline {
    fn drop(&mut self) {
        info!("Dropping GraphicsPipeline");
        unsafe {
            self.device.device_wait_idle().unwrap();
            
            self.device.destroy_pipeline(self.pipeline, None);
            self.device.destroy_pipeline_layout(self.pipeline_layout, None);
            self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

// TODO: implement pipeline for Compute shaders
#[derive(Clone, Resource)]
pub struct ComputePipeline {
    device: Device
}

/// Describes the shader stages, resource bindings, vertex information, and fixed function state of a graphics pipeline.
pub struct GraphicsPipelineInfo {
    pub vertex_stage_info: VertexStageInfo,
    pub fragment_stage_info: FragmentStageInfo,
    // TODO: Refactor to hide ash::vk
    pub input_assembly_state: vk::PipelineInputAssemblyStateCreateInfo,
    pub rasterization_state: vk::PipelineRasterizationStateCreateInfo,
    pub multisample_state: vk::PipelineMultisampleStateCreateInfo,
    pub descriptor_sets: Vec<vk::DescriptorSetLayout>
}

// TODO: Refactor to hide ash::vk
pub struct VertexStageInfo {
    pub shader: vk::ShaderModule,
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
    pub shader: vk::ShaderModule,
    pub entry_point: Cow<'static, str>,
    pub color_blend_states: Vec<vk::PipelineColorBlendAttachmentState>,
    pub target_states: Vec<vk::Format>
}



// Implement pipeline creation
impl Device {
    /// Create a new [`GraphicsPipeline`] from [`GraphicsPipelineInfo`]
    pub fn create_graphics_pipeline(
        &self,
        info: GraphicsPipelineInfo
    ) -> Result<GraphicsPipeline> {
        // Create shader stage infos
        let shader_stage_create_infos = [
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::VERTEX)
                .module(info.vertex_stage_info.shader)
                .name(unsafe { CStr::from_bytes_with_nul_unchecked(info.vertex_stage_info.entry_point.as_bytes()) })
                .build(),
            vk::PipelineShaderStageCreateInfo::builder()
                .stage(vk::ShaderStageFlags::FRAGMENT)
                .module(info.fragment_stage_info.shader)
                .name(unsafe { CStr::from_bytes_with_nul_unchecked(info.fragment_stage_info.entry_point.as_bytes()) })
                .build()
        ];
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
        // vk::PipelineInputAssemblyStateCreateInfo::builder()
        //     .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
        //     .primitive_restart_enable(false);

        let rasterization_state_create_info = info.rasterization_state;
        // vk::PipelineRasterizationStateCreateInfo::builder()
        //     .depth_clamp_enable(false)
        //     .rasterizer_discard_enable(false)
        //     .polygon_mode(vk::PolygonMode::FILL)
        //     .line_width(1.0)
        //     .cull_mode(vk::CullModeFlags::NONE)
        //     .front_face(vk::FrontFace::CLOCKWISE)
        //     .depth_bias_enable(false)
        //     .depth_bias_constant_factor(0.0)
        //     .depth_bias_clamp(0.0)
        //     .depth_bias_slope_factor(0.0);

        let multisample_state_create_info = info.multisample_state;
        // vk::PipelineMultisampleStateCreateInfo::builder()
        //     .rasterization_samples(vk::SampleCountFlags::TYPE_1);

        let color_blend_attachment_states = info.fragment_stage_info.color_blend_states.as_slice();
        // [
        //     vk::PipelineColorBlendAttachmentState::builder()
        //         .blend_enable(false)
        //         .src_color_blend_factor(vk::BlendFactor::SRC_COLOR)
        //         .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_DST_COLOR)
        //         .color_blend_op(vk::BlendOp::ADD)
        //         .src_alpha_blend_factor(vk::BlendFactor::ZERO)
        //         .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
        //         .alpha_blend_op(vk::BlendOp::ADD)
        //         .color_write_mask(vk::ColorComponentFlags::RGBA)
        //         .build()
        // ];
        let color_blend_state_create_info = vk::PipelineColorBlendStateCreateInfo::builder()
            .logic_op(vk::LogicOp::CLEAR)
            .attachments(color_blend_attachment_states);
        let mut pipeline_rendering_create_info = vk::PipelineRenderingCreateInfo::builder()
            .color_attachment_formats(info.fragment_stage_info.target_states.as_slice());

        // Create pipeline layouts
        // TODO: expose push constant configuration
        let push_constant = vk::PushConstantRange::builder()
            .offset(0)
            .size(size_of::<glm::Mat4>() as u32)
            .stage_flags(vk::ShaderStageFlags::VERTEX);

        // TODO: expose vertex binding configuration
        let vertex_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .descriptor_count(1)
            .stage_flags(vk::ShaderStageFlags::VERTEX);
        let create_info = vk::DescriptorSetLayoutCreateInfo::builder()
            .bindings(slice::from_ref(&vertex_binding));
        let descriptor_set_layout = unsafe { self.create_descriptor_set_layout(&create_info, None)? };

        let create_info = vk::PipelineLayoutCreateInfo::builder()
            .push_constant_ranges(slice::from_ref(&push_constant))
            .set_layouts(slice::from_ref(&descriptor_set_layout));
        let pipeline_layout = unsafe {
            match self.create_pipeline_layout(&create_info, None) {
                Ok(result) => result,
                Err(_) => bail!("Failed to create pipeline layout!".to_string())
            }
        };

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
            .layout(pipeline_layout);
        let pipeline = unsafe {
            match self.create_graphics_pipelines(vk::PipelineCache::null(), slice::from_ref(&create_info), None) {
                Ok(result) => result,
                Err(_) => bail!("Failed to create pipeline!".to_string())
            }
        }[0];

        Ok(GraphicsPipeline {
            device: self.clone(),
            descriptor_set_layout,
            pipeline,
            pipeline_layout
        })
    }
}