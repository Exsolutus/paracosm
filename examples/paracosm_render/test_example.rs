use bevy::prelude::*;

// use paracosm_gpu::GPUPlugin;
// use paracosm_render::RenderPlugin;

use std::env;

fn main() {
    env::set_var("RUST_LOG", "debug");
    env::set_var("RUST_BACKTRACE", "full");

    App::new()
        .add_plugins(DefaultPlugins)
        // .add_plugin(GPUPlugin)
        // .add_plugin(RenderPlugin)
        .run();
}
