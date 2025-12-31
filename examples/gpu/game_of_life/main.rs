use paracosm_gpu::{
    pipeline::ShaderSource, prelude::*, resource::{image::{Format, ImageInfo}, TransferMode}
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
            Some(&display_handle.0)
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

    // Load shaders
    let shader_source = ShaderSource::Crate("examples/gpu/game_of_life/shaders".into());
    context.create_pipeline(GameOfLifeInit, paracosm_gpu::pipeline::PipelineInfo::Compute { 
        shader_source: shader_source.clone(), 
        entry_point: "init" 
    }).unwrap();
    context.create_pipeline(GameOfLifeUpdate, paracosm_gpu::pipeline::PipelineInfo::Compute { 
        shader_source, 
        entry_point: "update" 
    }).unwrap();

    // Create game image
    let game_of_life_image = context.create_image(ImageInfo {
        format: Format::R32_SFLOAT,
        extent: [SIZE.x, SIZE.y, 0],
        mip_levels: 1,
        array_layers: 1,
        samples: SampleCount::TYPE_1,
        shared: false,
        transfer_mode: TransferMode::Auto,
        shader_mutable: true,
        #[cfg(debug_assertions)] debug_name: "GameOfLifeImage"
    }).unwrap();
    context.set_image_label(GameOfLifeImage, &game_of_life_image).unwrap();

    context.add_nodes(Queue::Graphics, (
        |mut interface: ComputeInterface, mut state: Local<GameOfLifeState>, game: Write<GameOfLifeImage>| {
            match *state {
                GameOfLifeState::Init => {
                    interface.bind_pipeline(GameOfLifeInit).unwrap();
                    interface.set_push_constant(PushConstant { 
                        descriptor_index: game.image().view(0).descriptor_index
                    }).unwrap();
                    interface.dispatch(SIZE.x / WORKGROUP_SIZE, SIZE.y / WORKGROUP_SIZE, 1);

                    *state = GameOfLifeState::Update;
                },
                GameOfLifeState::Update => {
                    interface.bind_pipeline(GameOfLifeUpdate).unwrap();
                    interface.set_push_constant(PushConstant {
                        descriptor_index: game.image().view(0).descriptor_index
                    }).unwrap();
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
    context: ResMut<Context>,
    app_exit: EventReader<AppExit>
) {
    if !app_exit.is_empty() {
        context.wait_idle();
        //context.destroy_image(GameOfLifeImage).unwrap();
    }
}
