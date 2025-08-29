use paracosm_gpu::{
    pipeline::ShaderSource, prelude::*, resource::{image::{Format, ImageInfo, SampleCountFlags}, TransferMode}
};

use bevy::{
    prelude::*, winit::{DisplayHandleWrapper}
};

use game_of_life_shared::PushConstant;


const APPNAME: &str = "Paracosm GPU Game of Life";

const DISPLAY_FACTOR: u32 = 4;
const SIZE: UVec2 = UVec2::new(1280 / DISPLAY_FACTOR, 720 / DISPLAY_FACTOR);
const WORKGROUP_SIZE: u32 = 8;

#[derive(ImageLabel)] struct GameOfLifeImage;
#[derive(SurfaceLabel)] struct PrimarySurface;
#[derive(PipelineLabel)] struct GameOfLifeInit;
#[derive(PipelineLabel)] struct GameOfLifeUpdate;

#[derive(Clone, Copy, Default)]
enum GameOfLifeState {
    #[default]
    Init,
    Update
}


fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                        primary_window: Some(Window {
                            resolution: (SIZE * DISPLAY_FACTOR).as_vec2().into(),
                            // uncomment for unthrottled FPS
                            // present_mode: bevy::window::PresentMode::AutoNoVsync,
                            ..default()
                        }),
                        ..default()
                    }),
            GameOfLifeComputePlugin,
        ))
        .add_systems(Startup, startup)
        .add_systems(Update, update)
        .add_systems(PostUpdate, shutdown)
        .run();
}

struct GameOfLifeComputePlugin;

impl Plugin for GameOfLifeComputePlugin {
    fn build(&self, app: &mut App) { }

    fn finish(&self, app: &mut App) {
        // Create GPU context
        let display_handle = app.world().resource::<DisplayHandleWrapper>();
        let context = Context::new(
            ContextInfo {
                application_name: APPNAME.into(),
                ..Default::default()
            }, 
            display_handle.0.clone()
        ).unwrap();
        app.insert_resource(context);
    }
}

fn startup(
    mut context: ResMut<Context>,
    primary_window: Query<(Entity, &Window, &bevy::window::RawHandleWrapper, &bevy::window::PrimaryWindow)>
) {
    let window = primary_window.single().unwrap();

    // Create primary window surface
    let window_handle = unsafe { window.2.get_handle() };
    context.create_surface(
        PrimarySurface, 
        window_handle, 
        SurfaceConfig::default()
    ).unwrap();

    // Create game image
    context.create_image(GameOfLifeImage, ImageInfo {
        format: Format::R32_SFLOAT,
        extent: [SIZE.x, SIZE.y, 0],
        mip_levels: 1,
        array_layers: 1,
        samples: SampleCountFlags::TYPE_1,
        shared: false,
        transfer_mode: TransferMode::Auto
    }).unwrap();

    // Load shaders
    let shader_module = context.load_shader_module(ShaderSource::Crate("examples/gpu/game_of_life/shaders".into())).unwrap();
    context.create_pipeline(GameOfLifeInit, paracosm_gpu::pipeline::PipelineInfo::Compute { 
        shader_module: shader_module.clone(), 
        entry_point: "init" 
    }).unwrap();
    context.create_pipeline(GameOfLifeUpdate, paracosm_gpu::pipeline::PipelineInfo::Compute { 
        shader_module, 
        entry_point: "update" 
    }).unwrap();

    

    context.add_nodes(Queue::Graphics, (
        |mut interface: ComputeInterface, mut state: Local<GameOfLifeState>, game: Write<GameOfLifeImage>| {
            match *state {
                GameOfLifeState::Init => {
                    interface.bind_pipeline(GameOfLifeInit).unwrap();
                    interface.set_push_constant(PushConstant { descriptor_index: *game }).unwrap();
                    interface.dispatch(SIZE.x / WORKGROUP_SIZE, SIZE.y / WORKGROUP_SIZE, 1);

                    *state = GameOfLifeState::Update;
                },
                GameOfLifeState::Update => {
                    interface.bind_pipeline(GameOfLifeUpdate).unwrap();
                    interface.set_push_constant(PushConstant { descriptor_index: *game }).unwrap();
                    interface.dispatch(SIZE.x / WORKGROUP_SIZE, SIZE.y / WORKGROUP_SIZE, 1);
                }
            }
        },
        |mut interface: GraphicsInterface, game: Read<GameOfLifeImage>, surface: Write<PrimarySurface>| {
            interface.blit_image_to_surface(game, surface).unwrap();
        }
    ).chain()).unwrap();

    context.add_submit(Queue::Graphics, None).unwrap();
}

fn update(
    mut context: ResMut<Context>,
) {
    context.execute().unwrap();
}

fn shutdown(
    mut context: ResMut<Context>,
    app_exit: EventReader<AppExit>
) {
    if !app_exit.is_empty() {
        context.wait_idle();
        context.destroy_image(GameOfLifeImage).unwrap();
    }
}
