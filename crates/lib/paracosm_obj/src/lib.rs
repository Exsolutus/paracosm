mod loader;
use bevy_render::mesh;
pub use loader::*;

use bevy_app::prelude::*;
use bevy_asset::{AddAsset, AssetServer};

use paracosm_render::mesh::*;


/// Adds support for Obj file loading to Apps
#[derive(Default)]
pub struct ObjPlugin;

impl Plugin for ObjPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset_loader::<ObjLoader>();

        // Load mesh assets
        let asset_server = app.world.get_resource::<AssetServer>().unwrap();
        let assets = match asset_server.load_folder("models/") {
            Ok(result) => result,
            Err(error) => panic!("Failed to load models: {}", error.to_string())
        };
        let models = assets.iter().map(|handle| {
            ("test", handle.clone().typed::<Mesh>())
        });
        let mut mesh_manager = app.world.get_resource_mut::<MeshManager>().unwrap();
        mesh_manager.meshes.extend(models);
    }
}