use paracosm_gpu::{
    pipeline::{PipelineInfo, ShaderSource}, prelude::*, resource::{image::ImageInfo, TransferMode}
};

use bevy::prelude::*;


const APPNAME: &str = "Paracosm GPU Basic Usage";
const APPVER: (u32, u32, u32, u32) = (0, 0, 1, 0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(PostStartup, startup)
        .add_systems(PostUpdate, render)
        .run();
}

fn startup(
    world: &mut World
) {
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

    // Check properties of active devices
    let devices = context.devices();

    // Optionally, manually set the primary device (index order as returned by [`devices()`])
    // context.set_primary_device(0).unwrap();

    // Work with primary device
    {
        // Resources
        #[derive(BufferLabel)] struct BufferA;
        #[derive(BufferLabel)] struct BufferB;

        context.create_buffer::<BufferA, u32>(TransferMode::Auto, 10).unwrap();
        context.create_transient_buffer(BufferB, 10).unwrap();
        //context.destroy_buffer(BufferA).unwrap();

        #[derive(ImageLabel)] struct ImageA;
        #[derive(ImageLabel)] struct ImageB;

        context.create_image(ImageInfo::default()).unwrap();
        

        // TODO: subresource view creation


        // Pipelines
        #[derive(PipelineLabel)] struct ComputeA;
        #[derive(PipelineLabel)] struct GraphicsA;
        #[derive(PipelineLabel)] struct RayTracingA;

        let shader_module_a = context.load_shader_module(ShaderSource::Crate("tests/compute".into())).unwrap();

        context.create_pipeline(ComputeA, PipelineInfo::Compute {
            shader_module: shader_module_a,
            entry_point: "main_cs",
        }).unwrap();
        context.create_pipeline(GraphicsA, PipelineInfo::Graphics {  }).unwrap();
        context.create_pipeline(RayTracingA, PipelineInfo::RayTracing {  }).unwrap();

        // Define nodes
        fn read(interface: ComputeInterface, read: Read<BufferA>) { /* ... */ }
        fn write(interface: ComputeInterface, write: Write<BufferB>) { /* ... */ }
        fn read_write(interface: TransferInterface, read: Read<BufferA>, write: Write<BufferB>) { /* ... */ }

        // Add nodes to frame graph
        context.add_nodes(
            Queue::Graphics,
            (
                read_write,
                read,
                write.after(read_write),
                // Inline node definition
                (|interface: GraphicsInterface, read: Read<BufferB>, write: Write<ImageB>| {
                    /* ... */
                }).after(write)
            )
        ).unwrap();

        context.add_nodes(
            Queue::Compute,
            |interface: ComputeInterface, write: Write<BufferA>| {
                /* ... */
            }
        ).unwrap();

        // Add queue submissions
        context.add_submit(
            Queue::Compute, 
            SubmitInfo {
                wait: [].into(),
                signal: [].into()
            }
        ).unwrap();
        context.add_submit(
            Queue::Graphics, 
            SubmitInfo {
                wait: [(Queue::Compute, 1)].into(),
                signal: [].into()
            }
        ).unwrap();
    }
    
    // Work with another device
    // {
    //     let devices = context.devices();
    //     let context = context.configuring_device(1).unwrap();

    //     // Same interface as above
    // }

    world.insert_non_send_resource(context);
}

fn render(mut context: NonSendMut<Context>) {
    // TODO: work out how resource upload/download works

    // Set pipeline push constants
    //context.set_push_constant::<GraphicsA>(PushConstantInfo::default()).unwrap();

    // TODO: work out window swapchain acquire and present

    // Build and run primary device frame graph
    context.execute().unwrap();

    // Present render targets to surfaces
    context.present().unwrap();
}