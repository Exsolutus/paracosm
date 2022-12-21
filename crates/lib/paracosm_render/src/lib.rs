pub mod mesh;
mod raster;
mod render_resource;
mod window;

use mesh::*;
use raster::*;
pub use render_resource::{
    pipeline::*,
    shader::*
};
use window::WindowRenderPlugin;
use render_resource::pipeline::PipelineManagerPlugin;

use paracosm_gpu::{instance::Instance};
use rust_shaders_shared::{Vertex, Vec4};

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;



#[derive(Default)]
pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    /// Initializes the renderer, sets up the [`RenderStage`](RenderStage) and creates the rendering sub-app.
    fn build(&self, app: &mut App) {
        // Get Vulkan instance from main app
        let instance = app
            .world
            .get_resource::<Instance>()
            .cloned()
            .unwrap_or_else(|| {
                // 'paracosm_gpu::GpuPlugin' wasn't loaded before this plugin
                app.add_plugin(paracosm_gpu::GpuPlugin);
                app.world.resource::<Instance>().clone()
            });

        // Initialize renderer
        let windows = app.world.resource::<bevy_window::Windows>();
        let window = match windows.get_primary() {
            Some(result) => result,
            None => return error!("No windows found for application!")
        };
        let (device, queue) = match initialize_renderer(window, instance.clone()) {
            Ok(result) => result,
            Err(error) => panic!("Renderer initialization failed: {}", error.to_string())
        };

        // Add render resource plugins
        app.add_plugin(ShaderPlugin)
            .add_plugin(PipelineManagerPlugin);


        // TODO: add proper asset management
        // Create triangle mesh
        let vertices = vec![
            Vertex::new(Vec4::new(-0.5, -0.5, 0.0, 0.0), Vec4::new(0.0, 0.0, 0.0, 0.0), Vec4::new(1.0, 0.0, 0.0, 0.0)),
            Vertex::new(Vec4::new(0.5, -0.5, 0.0, 0.0), Vec4::new(0.0, 0.0, 0.0, 0.0), Vec4::new(0.0, 1.0, 0.0, 0.0)),
            Vertex::new(Vec4::new(0.5, 0.5, 0.0, 0.0), Vec4::new(0.0, 0.0, 0.0, 0.0), Vec4::new(0.0, 0.0, 1.0, 0.0)),
            Vertex::new(Vec4::new(-0.5, 0.5, 0.0, 0.0), Vec4::new(0.0, 0.0, 0.0, 0.0), Vec4::new(1.0, 1.0, 1.0, 0.0)),
        ];
        let indices = vec![0, 1, 2, 2, 3, 0];
        let mut mesh = Mesh::with_geometry(vertices, indices);
        mesh.upload(device.clone()).unwrap();

        
        app.insert_resource(device.clone());

        app.add_system(render_system.at_end())
            .insert_resource(instance)
            .insert_resource(device)
            .insert_resource(queue)
            .insert_resource(mesh);

        // Add supporting plugins
        app.add_plugin(WindowRenderPlugin);
    }
}