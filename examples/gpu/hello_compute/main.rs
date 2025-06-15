use paracosm_gpu::{
    pipeline::{PipelineInfo, ShaderSource}, prelude::*, resource::TransferMode
};

use bevy::prelude::*;

const APPNAME: &str = "Paracosm GPU Hello Compute";
const APPVER: (u32, u32, u32, u32) = (0, 0, 1, 0);

const NUMBERS: [u32; 4] = [1, 2, 3, 4];
const OVERFLOW: u32 = 0xffffffff;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(PostStartup, run)
        .run();
}

fn run(world: &mut World) {
    let steps = execute_gpu(world, &NUMBERS);

    // Output results
    let disp_steps: Vec<String> = steps.iter()
        .map(|&n| match n {
            OVERFLOW => "OVERFLOW".to_string(),
            _ => n.to_string(),
        })
        .collect();
    println!("Steps: [{}]", disp_steps.join(", "));
}

fn execute_gpu(
    world: &mut World,
    numbers: &[u32]
) -> Vec<u32> {
    let mut primary_window = world.query::<(Entity, &Window, &bevy::window::RawHandleWrapper, &bevy::window::PrimaryWindow)>();
    let window_handle = unsafe { primary_window.single(world).unwrap().2.get_handle() }; 

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

    // Create numbers buffer
    #[derive(BufferLabel)] struct NumbersBuffer;

    context.create_buffer::<NumbersBuffer, u32>(
        TransferMode::Stream,
        numbers.len()
    ).unwrap();
    for (index, element) in context.get_buffer_memory_mut::<NumbersBuffer, u32>().unwrap().iter_mut().enumerate() {
        *element = numbers[index];
    }

    // Create shader pipelines
    let shader_module_a = context.load_shader_module(ShaderSource::Crate("examples/gpu/hello_compute/shaders".into())).unwrap();

    #[derive(PipelineLabel)] struct HelloCompute;
    context.create_pipeline(HelloCompute, PipelineInfo::Compute {
        shader_module: shader_module_a,
        entry_point: "main_cs",
    }).unwrap();


    // Add nodes to frame graph
    context.add_nodes(Queue::Compute,
        |mut interface: ComputeInterface, numbers_buffer: Write<NumbersBuffer>| {
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

    // Wait for primary device to finish execution
    context.wait_idle();

    let result = context.get_buffer_memory::<NumbersBuffer, u32>().unwrap().to_vec();

    context.destroy_buffer(NumbersBuffer).unwrap();

    result
}
