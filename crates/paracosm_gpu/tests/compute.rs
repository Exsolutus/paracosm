use paracosm_gpu::{
    prelude::*,
    pipeline::{ShaderSource, PipelineInfo}
};

const APPNAME: &str = "Paracosm GPU Compute Test";
const APPVER: (u32, u32, u32, u32) = (0, 0, 1, 0);


#[cfg(not(feature = "WSI"))]
#[test]
fn collatz() {
    // Create GPU context
    let mut context = Context::new(
        ContextInfo {
            application_name: APPNAME.into(),
            application_version: APPVER,
            ..Default::default()
        }, 
    ).unwrap();

    // Load shader modules
    let shader_module_a = context.load_shader_module(ShaderSource::Crate("crates/paracosm_gpu/tests/compute/shaders".into())).unwrap();

    // Define labels
    #[derive(PipelineLabel)] struct HelloCompute;

    // Set pipelines
    context.set_pipeline(HelloCompute, PipelineInfo::Compute {
        shader_module: shader_module_a,
        entry_point: "main_cs",
    }).unwrap();

    // Add nodes to frame graph
    context.add_nodes(Queue::Compute,
        |mut interface: ComputeInterface| {
            interface.bind_pipeline(HelloCompute).unwrap();
            interface.disbatch(192, 108, 1).unwrap();
        }
    ).unwrap();

    context.add_submit(
        Queue::Compute,
        SubmitInfo::default()
    ).unwrap();
}