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
use render_resource::pipeline::PipelineManagerPlugin;

use paracosm_gpu::{instance::Instance};

use bevy_app::{App, Plugin};
use bevy_asset::{Assets, AddAsset, AssetServer, Handle};
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_math::prelude::*;


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
            .add_plugin(PipelineManagerPlugin)
            .add_plugin(MeshPlugin)
            .add_plugin(ImagePlugin);



        // TODO: properly move into Bevy scene
        // Load/create assets
        let vertices = vec![
            Vertex::new(Vec3::new(-0.5, -0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
            Vertex::new(Vec3::new(0.5, -0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
            Vertex::new(Vec3::new(0.5, 0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0)),
            Vertex::new(Vec3::new(-0.5, 0.5, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0)),
        ];
        let indices = vec![0, 1, 2, 2, 3, 0];
        let mut mesh = Mesh::with_geometry(vertices, indices);
        mesh.upload(&device).unwrap();

        let mut mesh_assets = app.world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let square_handle = mesh_assets.add(mesh);

        let asset_server = app.world.get_resource::<AssetServer>().unwrap();
        let monkey_handle: Handle<Mesh> = asset_server.load("models/monkey_flat.obj");
        let image_handle: Handle<Image> = asset_server.load("textures/texture.png");

        // Cache asset handles
        let mut mesh_manager = app.world.get_resource_mut::<MeshManager>().unwrap();
        mesh_manager.meshes.insert("square".to_string(), square_handle);
        mesh_manager.meshes.insert("monkey".to_string(), monkey_handle);

        let mut image_manager = app.world.get_resource_mut::<ImageManager>().unwrap();
        image_manager.images.insert("test".to_string(), image_handle);
        


        app.insert_resource(device.clone());

        app.add_system(render_system.at_end())
            .insert_resource(instance)
            .insert_resource(device)
            .insert_resource(queue);

        // Add supporting plugins
        app.add_plugin(WindowRenderPlugin);
    }
}