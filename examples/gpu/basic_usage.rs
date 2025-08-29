use paracosm_gpu::{
    pipeline::{PipelineInfo, ShaderSource}, prelude::*, resource::{image::ImageInfo, TransferMode}
};

use bevy::{prelude::*, winit::DisplayHandleWrapper};


const APPNAME: &str = "Paracosm GPU Basic Usage";
const APPVER: (u32, u32, u32, u32) = (0, 0, 1, 0);


#[derive(BufferLabel)] struct BufferA;
#[derive(BufferLabel)] struct BufferB;

#[derive(ImageLabel)] struct ImageA;
#[derive(ImageLabel)] struct ImageB;

#[derive(PipelineLabel)] struct ComputeA;
#[derive(PipelineLabel)] struct GraphicsA;
#[derive(PipelineLabel)] struct RayTracingA;


fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(PostStartup, startup)
        .add_systems(PostUpdate, render)
        .run();
}

fn startup(
    world: &mut World,
) {
    // Create GPU context
    let display = world.resource::<DisplayHandleWrapper>();
    let mut context = Context::new(
        ContextInfo {
            application_name: APPNAME.into(),
            application_version: APPVER,
            ..Default::default()
        },
        Some(&display.0)
    ).unwrap();

    // Check properties of active devices
    let devices = context.devices();

    // Optionally, manually set the primary device (index order as returned by [`devices()`])
    // context.set_primary_device(0).unwrap();


    // Work with primary device
    {        
        // Manage active window surfaces
        let mut primary_window = world.query::<(Entity, &Window, &bevy::window::RawHandleWrapper, &bevy::window::PrimaryWindow)>();
        let primary_window_handle = unsafe { primary_window.single(world).unwrap().2.get_handle() }; 

        context.create_surface(PrimarySurface, primary_window_handle, SurfaceConfig::default()).unwrap();

        // Resources
        context.create_buffer(BufferA, TransferMode::Auto, 10).unwrap();
        context.create_transient_buffer(BufferB, 10).unwrap();
        //context.destroy_buffer(BufferA).unwrap();

        context.create_image(ImageA, ImageInfo::default()).unwrap();
        //context.destroy_image(ImageA).unwrap();

        // Pipelines
        let shader_module_a = context.load_shader_module(ShaderSource::Crate("path to shader crate here".into())).unwrap();

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
                (|mut interface: GraphicsInterface, read: Read<ImageB>, write: Write<PrimarySurface>| {
                    interface.blit_image_to_surface(read, write).unwrap();
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
        let compute_timeline_value = context.add_submit(
            Queue::Compute, 
            None
        ).unwrap();
        let graphics_timeline_value = context.add_submit(
            Queue::Graphics, 
            Some((Queue::Compute, compute_timeline_value))
        ).unwrap();

        context.clear_queue(Queue::Graphics).unwrap();
    }
    
    // Work with another device
    // {
    //     let devices = context.devices();
    //     let context = context.configuring_device(1).unwrap();

    //     // Same interface as above
    // }

    world.insert_resource(context);
}

fn render(mut context: NonSendMut<Context>) {
    // TODO: work out how resource upload/download works


    // TODO: work out window swapchain acquire and present

    // Build and run primary device frame graph
    context.execute().unwrap();
}