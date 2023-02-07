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
    asset_server: Res<AssetServer>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut image_manager: ResMut<ImageManager>,
    mut mesh_manager: ResMut<MeshManager>,
) {
    // TODO: properly move into Bevy scene
    // Load/create assets
    let image_handle: Handle<Image> = asset_server.load("textures/texture.png");
    let monkey_handle: Handle<Mesh> = asset_server.load("models/monkey_flat.obj");

    let vertices = vec![
        Vertex::new(Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec2::new(1.0, 1.0)),
        Vertex::new(Vec3::new(0.5, -0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec2::new(0.0, 1.0)),
        Vertex::new(Vec3::new(0.5, 0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec2::new(0.0, 0.0)),
        Vertex::new(Vec3::new(-0.5, 0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0), Vec2::new(1.0, 0.0)),
    ];
    let indices = vec![0, 1, 2, 2, 3, 0];
    let mesh = Mesh::with_geometry(vertices, indices);

    let square_handle = mesh_assets.add(mesh);

    // Cache asset handles
    image_manager.images.insert("statue".to_string(), image_handle);
    mesh_manager.meshes.insert("monkey".to_string(), monkey_handle);

    mesh_manager.meshes.insert("square".to_string(), square_handle);
}
