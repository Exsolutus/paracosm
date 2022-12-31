use crate::render_resource::shader::*;

use bevy_utils::{HashMap};
use paracosm_gpu::{
    device::Device,
    resource::pipeline::*,
};

use ash::vk;
use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, AssetEvent, Assets, Handle};
use bevy_ecs::{prelude::*};
use bevy_log::prelude::*;
use bevy_reflect::{TypeUuid};
use std::{
    borrow::Cow
};

use rust_shaders_shared::Vertex;

#[derive(Clone, TypeUuid)]
#[uuid = "22957743-5bc2-47f8-a6ff-a357c1e6dbe4"]
pub enum Pipeline {
    Graphics(GraphicsPipeline),
    Compute(ComputePipeline)
}

#[derive(Clone, Debug, Resource)]
pub struct PipelineManager {
    pub pipelines: HashMap<String, Handle<Pipeline>>
}


pub struct PipelineManagerPlugin;

impl Plugin for PipelineManagerPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Pipeline>()
            .add_debug_asset::<Pipeline>();

        app.add_system(
            generate_pipelines
                .at_end()
                .after(process_shader_events)
                .with_run_criteria(run_if_shader_events)
        );

        app.world.insert_resource(PipelineManager {
            pipelines: HashMap::new()
        })
    }
}


fn generate_pipelines(
    mut ev_asset: EventReader<AssetEvent<Shader>>,
    shader_assets: Res<Assets<Shader>>,
    mut pipeline_assets: ResMut<Assets<Pipeline>>,
    device: Res<Device>,
    mut pipeline_manager: ResMut<PipelineManager>
) {
    // TODO: load pipeline definitions from files
    debug!("Checking for pipelines to create!");
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } => {
                debug!("Creating a pipeline!");
                let shader = shader_assets.get(handle).unwrap();
                let pipeline = create_graphics_pipeline(device.clone(), shader);
                let handle = pipeline_assets.add(Pipeline::Graphics(pipeline));
                pipeline_manager.pipelines.insert(shader.name.to_string(), handle);
            },
            _ => ()
        }
    }
}

// TODO: load pipeline definitions from files
fn create_graphics_pipeline(device: Device, shader: &Shader) -> GraphicsPipeline {
    let module = match shader.module {
        Some(value) => value,
        None => panic!("Shader module not found.")
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
        descriptor_bindings: vec![
            vk::DescriptorSetLayoutBinding::builder()  // Vertex Input Attributes
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()   // Combined Image Sampler
                .binding(1)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .descriptor_count(1)
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .build()
        ]
    };
    
    let mesh_pipeline = match device.create_graphics_pipeline(pipeline_info) {
        Ok(result) => result,
        Err(error) => panic!("Pipeline creation failed: {}", error.to_string())
    };

    return mesh_pipeline
}

