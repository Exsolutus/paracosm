use bevy::prelude::*;
use bevy::asset::Assets;
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

use paracosm_render::RenderPlugin;
use paracosm_render::{Shader, ShaderHandle};

use std::env;


fn main() {
    env::set_var("RUST_LOG", "debug");
    env::set_var("RUST_BACKTRACE", "full");

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(RenderPlugin)
        //.add_startup_system(test_system)
        .run();
}

// fn test_system(
//     mut commands: Commands,
//     asset_server: Res<AssetServer>,
// ) {


//     commands.insert_resource(shader_handle)
// }
