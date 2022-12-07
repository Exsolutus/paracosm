mod extract_param;
mod extract_resource;
pub mod mesh;
mod raster;
mod render_resource;
mod window;

pub use extract_param::Extract;
use mesh::*;
use raster::*;
use window::WindowRenderPlugin;

use paracosm_gpu::{glm, instance::Instance, resource::pipeline::*};

use ash::vk;
use bevy_app::{App, AppLabel, Plugin};
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_time::prelude::*;
use bevy_utils;

use std::{
    any::TypeId,
    borrow::Cow,
    ops::{Deref, DerefMut},
    path::Path,
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

        // TODO: add proper pipeline management
        // Create mesh pipeline
        let vertex_spv_path = Path::new("./shaders/vert.spv");
        let fragment_spv_path = Path::new("./shaders/frag.spv");
        let vertex_module = match device.create_shader_module(vertex_spv_path) {
            Ok(result) => result,
            Err(error) => panic!("Failed to create vertex shader module: {}", error.to_string())
        };
        let fragment_module = match device.create_shader_module(fragment_spv_path) {
            Ok(result) => result,
            Err(error) => panic!("Failed to create fragment shader module: {}", error.to_string())
        };
        let binding_description = Vertex::binding_description();
        let attribute_descriptions = Vertex::attribute_descriptions().to_vec();

        let pipeline_info = GraphicsPipelineInfo {
            vertex_stage_info: VertexStageInfo {
                shader: vertex_module,
                entry_point: Cow::from("main\0"),
                vertex_input_desc: VertexInputDescription {
                    binding_description,
                    attribute_descriptions
                }
            },
            fragment_stage_info: FragmentStageInfo {
                shader: fragment_module,
                entry_point: Cow::from("main\0"),
                color_blend_states: vec![
                    vk::PipelineColorBlendAttachmentState::builder()
                        .blend_enable(false)
                        .src_color_blend_factor(vk::BlendFactor::SRC_COLOR)
                        .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_DST_COLOR)
                        .color_blend_op(vk::BlendOp::ADD)
                        .src_alpha_blend_factor(vk::BlendFactor::ZERO)
                        .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                        .alpha_blend_op(vk::BlendOp::ADD)
                        .color_write_mask(vk::ColorComponentFlags::RGBA)
                        .build()
                ],
                target_states: vec![
                    vk::Format::B8G8R8A8_UNORM
                ]
            },
            input_assembly_state: vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST)
                .primitive_restart_enable(false)
                .build(),
            rasterization_state: vk::PipelineRasterizationStateCreateInfo::builder()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::CLOCKWISE)
                .depth_bias_enable(false)
                .depth_bias_constant_factor(0.0)
                .depth_bias_clamp(0.0)
                .depth_bias_slope_factor(0.0)
                .build(),
            multisample_state: vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1)
                .build(),
            descriptor_sets: vec![]
        };
        
        let mesh_pipeline = match device.create_graphics_pipeline(pipeline_info) {
            Ok(result) => result,
            Err(error) => panic!("Pipeline creation failed: {}", error.to_string())
        };

        unsafe {
            device.destroy_shader_module(vertex_module, None);
            device.destroy_shader_module(fragment_module, None);
        }

        // TODO: add proper asset management
        // Create triangle mesh
        let mut mesh = Mesh::new();
        mesh.insert_vertex(Vertex::new(glm::vec3(-0.5, -0.5, 0.0), glm::vec3(0.0, 0.0, 0.0), glm::vec3(1.0, 0.0, 0.0)));
        mesh.insert_vertex(Vertex::new(glm::vec3(0.5, -0.5, 0.0), glm::vec3(0.0, 0.0, 0.0), glm::vec3(0.0, 1.0, 0.0)));
        mesh.insert_vertex(Vertex::new(glm::vec3(0.5, 0.5, 0.0), glm::vec3(0.0, 0.0, 0.0), glm::vec3(0.0, 0.0, 1.0)));
        mesh.insert_vertex(Vertex::new(glm::vec3(-0.5, 0.5, 0.0), glm::vec3(0.0, 0.0, 0.0), glm::vec3(1.0, 1.0, 1.0)));
        mesh.set_indices(vec![0, 1, 2, 2, 3, 0]);
        mesh.upload(device.clone()).unwrap();



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
                    .with_system(render_system.at_end())
            )
            .add_stage(RenderStage::Cleanup, SystemStage::parallel())
            .insert_resource(instance)
            .insert_resource(device)
            .insert_resource(queue)
            .insert_resource(mesh_pipeline)
            .insert_non_send_resource(mesh);
            
        app.add_sub_app(RenderApp, render_app, move |app_world, render_app| {
            #[cfg(not(feature = "trace"))]
            let _render_span = bevy_utils::tracing::info_span!("renderer subapp").entered();

            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "extract").entered();

                let time = app_world.get_resource::<Time>().unwrap().clone();
                render_app.insert_non_send_resource(time);

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