pub mod image;
pub mod mesh;
mod raster;
mod render_resource;
mod window;

use crate::image::*;
use mesh::*;
use raster::*;
pub use render_resource::{
    pipeline::*,
    shader::*
};
use window::WindowRenderPlugin;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;

use paracosm_gpu::GpuPlugin;



#[derive(Default)]
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    /// Initializes the renderer and sets up renderer systems and supporting plugins.
    fn build(&self, app: &mut App) {
        // Ensure paracosm_gpu::GpuPlugin is enabled
        if !app.is_plugin_added::<GpuPlugin>() {
            app.add_plugin(GpuPlugin);
        }

        // Add renderer systems
        app.add_startup_system(initialize_renderer.at_start())
            .add_system(render_system.at_end());

        // Add supporting plugins
        app.add_plugin(WindowRenderPlugin)
            .add_plugin(ShaderPlugin)
            .add_plugin(PipelineManagerPlugin)
            .add_plugin(MeshPlugin)
            .add_plugin(ImagePlugin);
    }
}