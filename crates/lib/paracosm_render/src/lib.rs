mod extract_param;
mod extract_resource;
pub mod mesh;
mod raster;
mod render_resource;
mod window;

pub use extract_param::Extract;
use mesh::*;
use raster::*;
pub use render_resource::{
    pipeline::*,
    shader::*
};
use window::WindowRenderPlugin;
use render_resource::pipeline::PipelineManagerPlugin;

use paracosm_gpu::{instance::Instance, resource::pipeline::GraphicsPipeline};
use rust_shaders_shared::{Vertex, Vec4};

use bevy_app::{App, AppLabel, Plugin};
use bevy_asset::{Assets, AssetServer};
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_time::prelude::*;
use bevy_utils;

use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};


/// The simulation [`World`] of the application, stored as a resource.
/// This resource is only available during [`RenderStage::Extract`] and not
/// during command application of that stage.
/// See [`Extract`] for more details.
#[derive(Resource, Default)]
pub struct MainWorld(World);

/// The Render App World. This is only available as a resource during the Extract step.
#[derive(Resource, Default)]
pub struct RenderWorld(World);

impl Deref for MainWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MainWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}


/// A Label for the rendering sub-app.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct RenderApp;

/// The labels of the default App rendering stages.
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum RenderStage {
    /// Extract data from the "app world" and insert it into the "render world".
    /// This step should be kept as short as possible to increase the "pipelining potential" for
    /// running the next frame while rendering the current frame.
    Extract,

    /// Prepare render resources from the extracted data for the GPU.
    Prepare,

    /// Create [`BindGroups`](crate::render_resource::BindGroup) that depend on
    /// [`Prepare`](RenderStage::Prepare) data and queue up draw calls to run during the
    /// [`Render`](RenderStage::Render) stage.
    Queue,

    // TODO: This could probably be moved in favor of a system ordering abstraction in Render or Queue
    /// Sort the [`RenderPhases`](crate::render_phase::RenderPhase) here.
    PhaseSort,

    /// Actual rendering happens here.
    /// In most cases, only the render backend should insert resources here.
    Render,

    /// Cleanup render resources here.
    Cleanup,
}

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
        app.add_plugin(ShaderManagerPlugin)
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



        app.init_resource::<ScratchMainWorld>();
        
        app.insert_resource(device.clone());

        // Create render app
        let mut render_app = App::empty();

        // Get the ComponentId for MainWorld. This does technically 'waste' a `WorldId`, but that's probably fine
        render_app.init_resource::<MainWorld>();
        render_app.world.remove_resource::<MainWorld>();
        let _main_world_in_render = render_app
            .world
            .components()
            .get_resource_id(TypeId::of::<MainWorld>());

        render_app
            .add_stage(RenderStage::Extract, SystemStage::parallel())
            .add_stage(RenderStage::Prepare, SystemStage::parallel())
            .add_stage(RenderStage::Queue, SystemStage::parallel())
            .add_stage(RenderStage::PhaseSort, SystemStage::parallel())
            .add_stage(
                RenderStage::Render, 
                SystemStage::parallel()
                    .with_system(render_system.at_end())
            )
            .add_stage(RenderStage::Cleanup, SystemStage::parallel())
            .insert_resource(instance)
            .insert_resource(device)
            .insert_resource(queue)
            .insert_resource(mesh);
            
        app.add_sub_app(RenderApp, render_app, move |app_world, render_app| {
            #[cfg(not(feature = "trace"))]
            let _render_span = bevy_utils::tracing::info_span!("renderer subapp").entered();

            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "extract").entered();

                let time = app_world.get_resource::<Time>().unwrap().clone();
                render_app.insert_non_send_resource(time);

                let pipelines = app_world.get_resource::<Assets<Pipeline>>().unwrap();
                let pipeline_manager = app_world.get_resource::<PipelineManager>().unwrap();
                if let Some(pipeline_handle) = pipeline_manager.pipelines.get("test.rs") {
                    if let Some(Pipeline::Graphics(pipeline)) = pipelines.get(pipeline_handle){
                        if let None = render_app.world.get_resource::<GraphicsPipeline>() {
                            render_app.insert_resource(pipeline.clone());
                        }
                    };
                };

                

                // extract
                extract(app_world, render_app);
            }

            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "prepare").entered();

                // prepare
                let prepare = render_app
                    .schedule
                    .get_stage_mut::<SystemStage>(RenderStage::Prepare)
                    .unwrap();
                prepare.run(&mut render_app.world);
            }

            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "render").entered();

                // render
                let render = render_app
                    .schedule
                    .get_stage_mut::<SystemStage>(RenderStage::Render)
                    .unwrap();
                render.run(&mut render_app.world);
            }

        });

        // Add supporting plugins
        app.add_plugin(WindowRenderPlugin);
    }
}


/// A "scratch" world used to avoid allocating new worlds every frame when
/// swapping out the [`MainWorld`] for [`RenderStage::Extract`].
#[derive(Resource, Default)]
struct ScratchMainWorld(World);

/// Executes the [`Extract`](RenderStage::Extract) stage of the renderer.
/// This updates the render world with the extracted ECS data of the current frame.
fn extract(app_world: &mut World, render_app: &mut App) {
    let extract = render_app
        .schedule
        .get_stage_mut::<SystemStage>(RenderStage::Extract)
        .unwrap();

    // temporarily add the app world to the render world as a resource
    let scratch_world = app_world.remove_resource::<ScratchMainWorld>().unwrap();
    let inserted_world = std::mem::replace(app_world, scratch_world.0);
    let running_world = &mut render_app.world;
    running_world.insert_resource(MainWorld(inserted_world));

    extract.run(running_world);
    // move the app world back, as if nothing happened.
    let inserted_world = running_world.remove_resource::<MainWorld>().unwrap();
    let scratch_world = std::mem::replace(app_world, inserted_world.0);
    app_world.insert_resource(ScratchMainWorld(scratch_world));

    // Note: We apply buffers (read, Commands) after the `MainWorld` has been removed from the render app's world
    // so that in future, pipelining will be able to do this too without any code relying on it.
    // see <https://github.com/bevyengine/bevy/issues/5082>
    extract.apply_buffers(running_world);
}