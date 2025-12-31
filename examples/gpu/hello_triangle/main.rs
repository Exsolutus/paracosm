use paracosm_gpu::{
    prelude::*, 
    queue::commands::graphics::{AttachmentLoadOp, AttachmentStoreOp, ClearValue, RenderingAttachmentInfo, RenderingInfo}, 
    resource::{buffer::BufferInfo, image::ImageInfo}
};

use bevy::{
    prelude::*, winit::{DisplayHandleWrapper}
};

use hello_triangle_shared::{
    PushConstant,
    Vertex
};


const APPNAME: &str = "Paracosm GPU Hello Triangle";

const SIZE: UVec2 = UVec2::new(1280, 720);

#[derive(PipelineLabel)] struct HelloTriangle;
#[derive(ImageLabel)] struct ColorImage;
#[derive(ImageLabel)] struct DepthImage;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(WindowPlugin {
                        primary_window: Some(Window {
                            resolution: (SIZE).as_vec2().into(),
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
    let shader_source = ShaderSource::Crate("examples/gpu/hello_triangle/shaders".into());
    context.create_pipeline(HelloTriangle, PipelineInfo::Graphics { 
        task_shader: None,
        mesh_shader: (shader_source.clone(), "main_ms"),
        fragment_shader: (shader_source, "main_fs"),
        viewport: ViewportInfo::default(),
        rasterization: RasterizationInfo { 
            depth_clamp_enable: false, 
            rasterizer_discard_enable: false, 
            polygon_mode: PolygonMode::FILL, 
            cull_mode: CullMode::BACK, 
            front_face: FrontFace::CLOCKWISE, 
            depth_bias_enable: false, 
            depth_bias_constant_factor: 0.0, 
            depth_bias_clamp: 0.0, 
            depth_bias_slope_factor: 0.0, 
            line_width: 1.0 
        },
        multisample: MultisampleInfo { 
            rasterization_samples: SampleCount::TYPE_1, 
            sample_shading_enable: false, 
            min_sample_shading: 1.0, 
            sample_mask: Default::default(), 
            alpha_to_coverage_enable: false, 
            alpha_to_one_enable: false 
        },
        depth_stencil: DepthTestInfo { 
            depth_test_enable: true, 
            depth_write_enable: true, 
            depth_compare_op: CompareOp::GREATER 
        },
        attachment: AttachmentInfo { 
            color_attachments: [(
                Format::R8G8B8A8_UNORM, 
                Some(ColorBlendInfo::default())
            )].into(),
            ..Default::default()
        }
    }).unwrap();

    // Create triangle buffer
    let mut triangle_buffer = context.create_buffer(BufferInfo { 
        transfer_mode: TransferMode::Stream, 
        size: size_of::<Vertex>() * 3,
        shader_mutable: false,
        #[cfg(debug_assertions)] debug_name: "TriangleBuffer"
    }).unwrap();
    let memory = context.get_buffer_memory_mut::<[Vertex; 3]>(&mut triangle_buffer).unwrap();
    memory[0] = Vertex { position: [ 0.0, -0.5, 0.0 ], color: [ 1.0, 0.0, 0.0 ] };
    memory[1] = Vertex { position: [ 0.5, 0.5, 0.0 ], color: [ 0.0, 1.0, 0.0 ] };
    memory[2] = Vertex { position: [ -0.5, 0.5, 0.0 ], color: [ 0.0, 0.0, 1.0 ] };

    // Create render targets
    let color_image = context.create_image(ImageInfo {
        format: Format::R8G8B8A8_UNORM,
        extent: [SIZE.x, SIZE.y, 0],
        mip_levels: 1,
        array_layers: 1,
        samples: SampleCount::TYPE_1,
        shared: false,
        transfer_mode: TransferMode::Auto,
        shader_mutable: true,
        #[cfg(debug_assertions)] debug_name: "Color"
    }).unwrap();
    context.set_image_label(ColorImage, &color_image).unwrap();

    let depth_image = context.create_image(ImageInfo {
        format: Format::D16_UNORM,
        extent: [SIZE.x, SIZE.y, 0],
        mip_levels: 1,
        array_layers: 1,
        samples: SampleCount::TYPE_1,
        shared: false,
        transfer_mode: TransferMode::Auto,
        shader_mutable: true,
        #[cfg(debug_assertions)] debug_name: "Depth"
    }).unwrap();
    context.set_image_label(DepthImage, &depth_image).unwrap();

    let triangle_buffer_index = context.get_buffer(&triangle_buffer).unwrap().descriptor_index;
    
    context.add_nodes(Queue::Graphics, (
        move |mut interface: GraphicsInterface, color: Write<ColorImage>, depth: Write<DepthImage>| {
            interface.begin_rendering(RenderingInfo {
                render_area: (0, 0, SIZE.x, SIZE.y),
                color_attachments: [
                    RenderingAttachmentInfo {
                        image_view: color.image().view(0),
                        load_op: AttachmentLoadOp::CLEAR,
                        store_op: AttachmentStoreOp::STORE,
                        clear_value: ClearValue::default(),
                        resolve: None
                    }
                ].into(),
                depth_attachment: Some(RenderingAttachmentInfo {
                    image_view: depth.image().view(0),
                    load_op: AttachmentLoadOp::CLEAR,
                    store_op: AttachmentStoreOp::STORE,
                    clear_value: ClearValue::default(),
                    resolve: None
                }),
                stencil_attachment: None
            }).unwrap();
            interface.bind_pipeline(HelloTriangle).unwrap();
            interface.set_push_constant(PushConstant { 
                vertex_data: triangle_buffer_index
            }).unwrap();
            interface.draw_mesh([1, 1, 1]).unwrap();
            interface.end_rendering();
        },
        |mut interface: GraphicsInterface, color: Read<ColorImage>, surface: Write<PrimarySurface>| {
            interface.blit_image_to_surface(color, surface).unwrap();
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
        //context.destroy_buffer(TriangleBuffer).unwrap();
    }
}
