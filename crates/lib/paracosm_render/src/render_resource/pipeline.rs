
use crate::render_resource::shader::*;
use crate::{RenderStage, RenderApp};

use paracosm_gpu::{
    device::Device,
    resource::pipeline::*,
};

use ash::vk;
use bevy_app::{App, Plugin};
use bevy_asset::{Assets, AssetEvent};
use bevy_ecs::{prelude::*, schedule::ShouldRun};
use bevy_log::prelude::*;
use std::{
    borrow::Cow
};

use rust_shaders_shared::Vertex;

pub struct PipelineManagerPlugin;

impl Plugin for PipelineManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
                SystemSet::new()
                    .with_run_criteria(run_if_pending_shaders)
                    .with_system(generate_pipelines)
            );
    }
}

fn run_if_pending_shaders(
    ev_asset: EventReader<AssetEvent<Shader>>
) -> ShouldRun {
    match ev_asset.is_empty() {
        true => ShouldRun::No,
        false => ShouldRun::Yes
    }
}

fn generate_pipelines(
    mut ev_asset: EventReader<AssetEvent<Shader>>,
    assets: Res<Assets<Shader>>,
    device: Res<Device>,
    mut commands: Commands,
) {
    debug!("Checking for pipelines to create!");
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } => {
                let shader = assets.get(handle).unwrap();
                let pipeline = create_graphics_pipeline(device.clone(), shader);
                commands.insert_resource(pipeline);
            },
            _ => ()
        }
    }
}

fn create_graphics_pipeline(device: Device, shader: &Shader) -> GraphicsPipeline {
    let module = match device.create_shader_module(&shader.path) {
        Ok(result) => result,
        Err(error) => panic!("Failed to create shader module: {}", error.to_string())
    };

    let binding_description = Vertex::binding_description();
    let attribute_descriptions = Vertex::attribute_descriptions().to_vec();

    let pipeline_info = GraphicsPipelineInfo {
        vertex_stage_info: VertexStageInfo {
            shader: module,
            entry_point: Cow::from("main_vs\0"),
            vertex_input_desc: VertexInputDescription {
                binding_description,
                attribute_descriptions
            }
        },
        fragment_stage_info: FragmentStageInfo {
            shader: module,
            entry_point: Cow::from("main_fs\0"),
            color_blend_states: vec![
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
            ],
            target_states: vec![
                vk::Format::B8G8R8A8_UNORM
            ]
        },
        input_assembly_state: vk::PipelineInputAssemblyStateCreateInfo::builder()
            .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
            .primitive_restart_enable(false)
            .build(),
        rasterization_state: vk::PipelineRasterizationStateCreateInfo::builder()
            .depth_clamp_enable(false)
            .rasterizer_discard_enable(false)
            .polygon_mode(vk::PolygonMode::FILL)
            .line_width(1.0)
            .cull_mode(vk::CullModeFlags::NONE)
            .front_face(vk::FrontFace::CLOCKWISE)
            .depth_bias_enable(false)
            .depth_bias_constant_factor(0.0)
            .depth_bias_clamp(0.0)
            .depth_bias_slope_factor(0.0)
            .build(),
        multisample_state: vk::PipelineMultisampleStateCreateInfo::builder()
            .rasterization_samples(vk::SampleCountFlags::TYPE_1)
            .build(),
        descriptor_sets: vec![]
    };
    
    let mesh_pipeline = match device.create_graphics_pipeline(pipeline_info) {
        Ok(result) => result,
        Err(error) => panic!("Pipeline creation failed: {}", error.to_string())
    };

    unsafe {
        device.destroy_shader_module(module, None);
    }

    return mesh_pipeline
}

