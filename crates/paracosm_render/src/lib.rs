mod extract_param;
mod raster;
mod window;

use paracosm_gpu::Instance;
use raster::Renderer;

use crate::window::WindowRenderPlugin;

pub use extract_param::Extract;

use bevy_app::{App, AppLabel, Plugin};
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_utils;

use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

// TODO: make sure you understand the usage of this main world
/// The simulation [`World`] of the application, stored as a resource.
/// This resource is only available during [`RenderStage::Extract`] and not
/// during command application of that stage.
/// See [`Extract`] for more details.
#[derive(Default)]
pub struct MainWorld(World);

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
        let renderer = match Renderer::new(window, instance.clone()) {
            Ok(result) => result,
            Err(error) => panic!("Renderer initialization failed: {}", error.to_string())
        };

        app.init_resource::<ScratchMainWorld>();

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
                    .with_system(Renderer::render_system.exclusive_system().at_end())
            )
            .add_stage(RenderStage::Cleanup, SystemStage::parallel())
            .insert_resource(instance)
            .insert_resource(renderer);
            
        app.add_sub_app(RenderApp, render_app, move |app_world, render_app| {
            #[cfg(not(feature = "trace"))]
            let _render_span = bevy_utils::tracing::info_span!("renderer subapp").entered();

            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "extract").entered();

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
                    .get_stage_mut::<SystemStage>(&RenderStage::Prepare)
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
                    .get_stage_mut::<SystemStage>(&RenderStage::Render)
                    .unwrap();
                render.run(&mut render_app.world);
            }

        });

        // Add supporting plugins
        app.add_plugin(WindowRenderPlugin);
    }
}

// TODO: make sure you understand usage of this scratch world
/// A "scratch" world used to avoid allocating new worlds every frame when
/// swapping out the [`MainWorld`] for [`RenderStage::Extract`].
#[derive(Default)]
struct ScratchMainWorld(World);

/// Executes the [`Extract`](RenderStage::Extract) stage of the renderer.
/// This updates the render world with the extracted ECS data of the current frame.
fn extract(app_world: &mut World, render_app: &mut App) {
    let extract = render_app
        .schedule
        .get_stage_mut::<SystemStage>(&RenderStage::Extract)
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