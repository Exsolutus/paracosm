use bevy::prelude::*;

use paracosm_gpu::GpuPlugin;
// use paracosm_render::RenderPlugin;

use std::env;

fn main() {
    env::set_var("RUST_LOG", "debug");
    env::set_var("RUST_BACKTRACE", "full");

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(GpuPlugin)
        // .add_plugin(RenderPlugin)
        .run();
}
