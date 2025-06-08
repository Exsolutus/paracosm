use paracosm_gpu::{
    pipeline::{PipelineInfo, ShaderSource}, prelude::*, resource::{buffer::BufferInfo, TransferMode}
};

use bevy::prelude::*;

const APPNAME: &str = "Paracosm GPU Hello Compute";
const APPVER: (u32, u32, u32, u32) = (0, 0, 1, 0);

const NUMBERS: [u32; 4] = [1, 2, 3, 4];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(PostStartup, startup)
        .run();
}

fn startup(world: &mut World) {
    let mut primary_window = world.query::<(Entity, &Window, &bevy::window::RawHandleWrapper, &bevy::window::PrimaryWindow)>();
    let window_handle = unsafe { primary_window.single(world).2.get_handle() }; 

    // Create GPU context
    let mut context = Context::new(
        ContextInfo {
            application_name: APPNAME.into(),
            application_version: APPVER,
            ..Default::default()
        }, 
        &window_handle,
        SurfaceConfig::default()
    ).unwrap();

    // Load shader modules
    let shader_module_a = context.load_shader_module(ShaderSource::Crate("examples/gpu/hello_compute/shaders".into())).unwrap();

    // Create resources
    let storage_buffer = context.create_buffer(BufferInfo {
        size: size_of::<u32>() * NUMBERS.len(),
        transfer_mode: TransferMode::Stream,
        debug_name: "NumbersBuffer"
    }).unwrap();

    // Define labels
    #[derive(PipelineLabel)] struct HelloCompute;
    #[derive(BufferLabel)] struct NumbersBuffer;

    // Set pipelines
    context.set_pipeline(HelloCompute, PipelineInfo::Compute {
        shader_module: shader_module_a,
        entry_point: "main_cs",
    }).unwrap();

    // Set resources
    context.set_persistent_buffer(NumbersBuffer, &storage_buffer).unwrap();



    // Add nodes to frame graph
    context.add_nodes(Queue::Compute,
        |mut interface: ComputeInterface, numbers: Write<NumbersBuffer>| {
            interface.bind_pipeline(HelloCompute).unwrap();
            interface.disbatch(NUMBERS.len() as u32, 1, 1).unwrap();
        }
    ).unwrap();

    context.add_submit(
        Queue::Compute,
        SubmitInfo::default()
    ).unwrap();


    // Build and run frame graph
    context.execute().unwrap();
}
