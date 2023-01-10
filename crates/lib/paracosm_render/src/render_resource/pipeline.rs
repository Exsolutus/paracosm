
use anyhow::Result;

use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, Handle};
use bevy_ecs::{prelude::*};
use bevy_reflect::{TypeUuid};
use bevy_utils::{HashMap};

use paracosm_gpu::{
    device::Device,
    resource::pipeline::*,
};



#[derive(Clone, TypeUuid)]
#[uuid = "22957743-5bc2-47f8-a6ff-a357c1e6dbe4"]
pub enum Pipeline {
    Graphics(GraphicsPipeline),
    Compute(ComputePipeline)
}

impl Pipeline {
    pub fn graphics(
        device: Device,
        vertex_stage_info: VertexStageInfo,
        fragment_stage_info: FragmentStageInfo,
        pipeline_layout: PipelineLayout
    ) -> Result<Self> {
        let pipeline_info = GraphicsPipelineInfo {
            vertex_stage_info,
            fragment_stage_info,
            input_assembly_state: PipelineInputAssemblyStateCreateInfo::builder()
                .topology(PrimitiveTopology::TRIANGLE_LIST)
                .primitive_restart_enable(false)
                .build(),
            rasterization_state: PipelineRasterizationStateCreateInfo::builder()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(PolygonMode::FILL)
                .line_width(1.0)
                .cull_mode(CullModeFlags::BACK)
                .front_face(FrontFace::COUNTER_CLOCKWISE)
                .depth_bias_enable(false)
                .depth_bias_constant_factor(0.0)
                .depth_bias_clamp(0.0)
                .depth_bias_slope_factor(0.0)
                .build(),
            depth_stencil_state: Some(PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(true)
                .depth_write_enable(true)
                .depth_compare_op(CompareOp::GREATER_OR_EQUAL)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .build()),
            multisample_state: PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(SampleCountFlags::TYPE_1)
                .build(),
        };

        Ok(Pipeline::Graphics(device.create_graphics_pipeline(pipeline_info, pipeline_layout)?))
    }
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

        app.world.insert_resource(PipelineManager {
            pipelines: HashMap::new()
        })
    }
}
