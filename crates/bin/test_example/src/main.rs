use bevy::prelude::*;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

use paracosm_gpu::{resource::pipeline::*};
use paracosm_obj::ObjPlugin;
use paracosm_render::{RenderPlugin, RenderContext, image::*, mesh::*, Shader, ShaderManager, Pipeline, PipelineManager};

use std::{
    borrow::Cow,
    env,
    path::Path
};


fn main() {
    env::set_var("RUST_LOG", "debug");
    env::set_var("RUST_BACKTRACE", "full");

    App::new()
        .add_plugins(DefaultPlugins)
        // .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(ObjPlugin)
        .add_plugin(RenderPlugin)
        .add_startup_system(load_assets)
        .run();
}

fn load_assets(
    render_context: Res<RenderContext>,
    asset_server: Res<AssetServer>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut shader_assets: ResMut<Assets<Shader>>,
    mut pipeline_assets: ResMut<Assets<Pipeline>>,
    mut image_manager: ResMut<ImageManager>,
    mut mesh_manager: ResMut<MeshManager>,
    mut shader_manager: ResMut<ShaderManager>,
    mut pipeline_manager: ResMut<PipelineManager>
) {
    let device = &render_context.device;
    let resource_manager = &render_context.resource_manager;
    let pipeline_layout = resource_manager.pipeline_layouts[0];

    // TODO: properly move into Bevy scene
    // Load/create assets
    let image_handle: Handle<Image> = asset_server.load("textures/texture.png");
    let monkey_handle: Handle<Mesh> = asset_server.load("models/monkey_flat.obj");

    let vertices = vec![
        Vertex::new(Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
        Vertex::new(Vec3::new(0.5, -0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
        Vertex::new(Vec3::new(0.5, 0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0)),
        Vertex::new(Vec3::new(-0.5, 0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
    ];
    let indices = vec![0, 1, 2, 2, 3, 0];
    let mesh = Mesh::with_geometry(vertices, indices);

    let square_handle = mesh_assets.add(mesh);

    // Load shaders
    let path = Path::new("assets/shaders/test.spv");
    let module = device.create_shader_module(&path).unwrap();
    let vertex_shader = Shader {
        module: module.clone(),
        entry_point: Cow::from("test::main_vs\0")
    };
    let fragment_shader = Shader {
        module,
        entry_point: Cow::from("test::main_fs\0")
    };

    // Create mesh pipeline
    let pipeline = Pipeline::graphics(
        device.clone(), 
        VertexStageInfo {
            shader: vertex_shader.module.clone(),
            entry_point: vertex_shader.entry_point.clone(),
            vertex_input_desc: VertexInputDescription {
                binding_description: Vertex::binding_description(),
                attribute_descriptions: Vertex::attribute_descriptions().to_vec()
            }
        },
        FragmentStageInfo {
            shader: fragment_shader.module.clone(),
            entry_point: fragment_shader.entry_point.clone(),
            color_blend_states: vec![
                PipelineColorBlendAttachmentState::builder()
                    .blend_enable(false)
                    .src_color_blend_factor(BlendFactor::SRC_COLOR)
                    .dst_color_blend_factor(BlendFactor::ONE_MINUS_DST_COLOR)
                    .color_blend_op(BlendOp::ADD)
                    .src_alpha_blend_factor(BlendFactor::ZERO)
                    .dst_alpha_blend_factor(BlendFactor::ZERO)
                    .alpha_blend_op(BlendOp::ADD)
                    .color_write_mask(ColorComponentFlags::RGBA)
                    .build()
            ],
            target_states: vec![
                Format::B8G8R8A8_UNORM
            ]
        },
        pipeline_layout
    ).expect("Graphics pipeline should exist");

    let vs_handle = shader_assets.add(vertex_shader);
    let fs_handle = shader_assets.add(fragment_shader);
    let pipeline_handle = pipeline_assets.add(pipeline);

    // Cache asset handles
    image_manager.images.insert("test".to_string(), image_handle);
    mesh_manager.meshes.insert("monkey".to_string(), monkey_handle);

    mesh_manager.meshes.insert("square".to_string(), square_handle);

    shader_manager.shaders.insert("main_vs".to_string(), vs_handle);
    shader_manager.shaders.insert("main_fs".to_string(), fs_handle);

    pipeline_manager.pipelines.insert("mesh_pipeline".to_string(), pipeline_handle);
}
