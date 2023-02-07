use crate::{
    image::*, 
    mesh::*,
    Pipeline,
    PipelineManager,
    render_asset::RenderAssets,
    render_resource::ResourceManager,
    window::WindowSurfaces,
    Shader, 
    ShaderManager,
};

use ash::vk;

use bevy_asset::prelude::*;
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_time::prelude::*;
use bevy_window::Windows;

use paracosm_gpu::{
    instance::Instance, 
    device::Device,
    resource::{
        buffer::*,
        image as gpu_image,
        pipeline::*,
        sampler as gpu_sampler,
    }
};

use rust_shaders_shared::glam;

use std::{
    borrow::Cow,
    env,
    path::Path,
    mem::size_of,
    slice
};



/// The [`RenderContext`] resource provides access to the renderer's GPU resources
#[derive(Resource)]
pub struct RenderContext {
    pub device: Device,
    pub resource_manager: ResourceManager,
}

// TODO: Properly implement scene object management
#[derive(Default, Resource)]
pub struct SceneData {
    object_buffers: Vec<(Buffer, ResourceHandle)>,
}



pub fn initialize_renderer(
    windows: Res<Windows>,
    instance: Res<Instance>,
    mut commands: Commands
) {
    // Create Device
    let Some(window) = windows.get_primary() else {
        return error!("No windows found for application!");
    };
    let window_handle = window.raw_handle();

    let device = Device::primary(instance.clone(), window_handle)
        .expect("Vulkan should find a Device with required support");

    // Create resource manager
    let resource_manager = ResourceManager::new(&device)
        .expect("A ResourceManager should be created for the Device");

    // Insert RenderContext
    let render_context = RenderContext {
        device,
        resource_manager,
    };
    
    initialize_internal_assets(&render_context, &mut commands);

    // Insert renderer resources
    commands.insert_resource(render_context);
    commands.insert_resource(SceneData::default());
}

/// Renderer main loop
pub fn render_system(
    render_context: Res<RenderContext>,
    windows: Res<Windows>,
    mut window_surfaces: NonSendMut<WindowSurfaces>,
    pipeline_handles: Res<PipelineManager>,
    pipeline_assets: Res<Assets<Pipeline>>,
    mesh_handles: Res<MeshManager>,
    meshes: Res<RenderAssets<Mesh>>,
    image_handles: Res<ImageManager>,
    images: Res<RenderAssets<Image>>,
    sampler_handles: Res<SamplerManager>,
    samplers: Res<RenderAssets<Sampler>>,
    mut scene_data: ResMut<SceneData>,  // TODO: properly implement scene object management
    time: NonSend<Time>
) {
    let device = &render_context.device;
    let resource_manager = &render_context.resource_manager;
    let pipeline_layout = resource_manager.pipeline_layouts[0];

    //let _span = info_span!("present_frames").entered();

    // TODO: convert window iteration to Views and simultaneous presentation
    // Render for each active window surface
    for window in windows.iter() {
        // Check window is configured
        if !window_surfaces.configured_windows.contains(&window.id()) {
            continue;
        }

        // Get surface for window
        let Some(surface) = window_surfaces.surfaces.get_mut(&window.id()) else {
            continue;
        };
        let Ok(extent) = surface.extent() else {
            continue;
        };

        // Begin rendering
        let command_buffer = match surface.begin_rendering() {
            Ok(result) => result,
            Err(error) => {
                error!("Renderer::render_system: {}", error);
                continue;
            }
        };

        resource_manager.bind(command_buffer);



        // TODO: properly implement scene object management
        // Init per-frame object buffers if necessary
        let object_buffers = &mut scene_data.object_buffers;
        if object_buffers.is_empty() {
            for frame in 0..surface.frame_count() {
                let info = BufferInfo::new(
                    size_of::<rust_shaders_shared::ObjectData>() * 10000,
                    BufferUsageFlags::INDIRECT_BUFFER | BufferUsageFlags::STORAGE_BUFFER,
                    MemoryLocation::CpuToGpu
                );
                let object_buffer = device.create_buffer(format!("Object Buffer (Frame {})", frame).as_str(), info, None);
                let handle = resource_manager.new_buffer_handle(&object_buffer);
                object_buffers.push((object_buffer, handle));
            }
        }
        let object_buffer = &object_buffers[0];

        let mut object_data = Vec::with_capacity(10000);
        for i in 0..100 {
            for j in 0..100 {
                object_data.push(rust_shaders_shared::ObjectData{
                    model_matrix: glam::Mat4::from_scale_rotation_translation(
                        glam::Vec3::ONE, 
                        glam::Quat::from_axis_angle(glam::Vec3::Y, time.elapsed_seconds() * (45_f32 + j as f32).to_radians()), 
                        glam::vec3((i * 2) as f32, 0f32, (j * 2) as f32)
                    ),
                })
            }
        }
        object_buffer.0.write_buffer(&object_data);
        let object_buffers = &scene_data.object_buffers;

        let mesh_asset = match mesh_handles.meshes.get("square") {
            Some(value) => meshes.get(value),
            None => None
        };

        let test_image = match image_handles.images.get("statue") {
            Some(value) => images.get(value),
            None => None
        };

        let linear_sampler = match  sampler_handles.samplers.get("Linear") {
            Some(value) => samplers.get(value),
            None => None
        };

        // Do rendering tasks
        if let Some(Pipeline::Graphics(pipeline)) = match pipeline_handles.pipelines.get("textured_lit_mesh") {
            Some(value) => pipeline_assets.get(value),
            None => None
        } {
            unsafe {
                let viewports = [
                    vk::Viewport::builder()
                        .width(extent.width as f32)
                        .height(extent.height as f32)
                        .min_depth(1.0)
                        .max_depth(0.0)
                        .build()
                ];
                let scissors = [extent.into()];
                device.cmd_set_viewport(command_buffer, 0, &viewports);
                device.cmd_set_scissor(command_buffer, 0, &scissors);
                device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, pipeline.pipeline);

                // Camera
                let mut proj = glam::Mat4::perspective_infinite_rh(
                    45_f32.to_radians(), 
                    extent.width as f32 / extent.height as f32, 
                    0.1, 
                );
                proj.y_axis *= -1.0;
                let view = glam::Mat4::look_at_rh(
                    glam::vec3(-5.0, 2.0, -5.0), 
                    glam::Vec3::ZERO,
                    glam::Vec3::Y
                );
                let camera_matrix = proj * view;

                let push_constant = [rust_shaders_shared::ShaderConstants {
                    camera_matrix,
                    object_buffer_handle: object_buffers[0].1,
                }];
                let (_, push_constant_bytes, _) = push_constant.align_to::<u8>();

                device.cmd_push_constants(command_buffer, pipeline_layout, vk::ShaderStageFlags::ALL, 0, push_constant_bytes);

                if let Some(mesh) = mesh_asset {
                    let vertex_buffer = mesh.vertex_buffer.buffer;
                    let index_buffer = mesh.index_buffer.buffer;
                    
                    device.cmd_bind_vertex_buffers(command_buffer, 0, slice::from_ref(&vertex_buffer), &[0]);
                    device.cmd_bind_index_buffer(command_buffer, index_buffer, 0, vk::IndexType::UINT32);

                    device.cmd_draw_indexed(command_buffer, mesh.index_count as u32, 10000, 0, 0, 0);
                }
            }
        }

        // End rendering
        if let Err(error) = surface.end_rendering() {
            error!("Renderer::render_system: {}", error);
            continue;
        };

        // Present rendered image to surface
        if let Err(error) = surface.queue_present() {
            error!("Renderer::render_system: {}", error);
            continue;
        };
    }
}


fn initialize_internal_assets(render_context: &RenderContext, commands: &mut Commands) {
    let device = &render_context.device;
    let resource_manager = &render_context.resource_manager;
    let pipeline_layout = resource_manager.pipeline_layouts[0];
    
    // Load shaders
    let path = Path::new("assets/shaders/rust_shaders.spv");
    let module = device.create_shader_module(&path).unwrap();
    let mesh_vert = Shader {
        module: module.clone(),
        entry_point: Cow::from("vert::mesh::main\0")
    };
    let unlit_frag = Shader {
        module: module.clone(),
        entry_point: Cow::from("frag::unlit::main\0")
    };
    let textured_lit_frag = Shader {
        module,
        entry_point: Cow::from("frag::textured_lit::main\0")
    };

    // Create mesh pipeline
    let unlit_pipeline = Pipeline::graphics(
        device.clone(), 
        VertexStageInfo {
            shader: mesh_vert.module.clone(),
            entry_point: mesh_vert.entry_point.clone(),
            vertex_input_desc: VertexInputDescription {
                binding_description: Vertex::binding_description(),
                attribute_descriptions: Vertex::attribute_descriptions().to_vec()
            }
        },
        FragmentStageInfo {
            shader: unlit_frag.module.clone(),
            entry_point: unlit_frag.entry_point.clone(),
            color_blend_states: vec![
                PipelineColorBlendAttachmentState::builder()
                    .blend_enable(false)
                    .src_color_blend_factor(BlendFactor::SRC_COLOR)
                    .dst_color_blend_factor(BlendFactor::ONE_MINUS_DST_COLOR)
                    .color_blend_op(BlendOp::ADD)
                    .src_alpha_blend_factor(BlendFactor::ZERO)
                    .dst_alpha_blend_factor(BlendFactor::ZERO)
                    .alpha_blend_op(BlendOp::ADD)
                    .color_write_mask(ColorComponentFlags::RGBA)
                    .build()
            ],
            target_states: vec![
                Format::B8G8R8A8_UNORM
            ]
        },
        pipeline_layout
    ).expect("Graphics pipeline should be created");

    let textured_lit_pipeline = Pipeline::graphics(
        device.clone(), 
        VertexStageInfo {
            shader: mesh_vert.module.clone(),
            entry_point: mesh_vert.entry_point.clone(),
            vertex_input_desc: VertexInputDescription {
                binding_description: Vertex::binding_description(),
                attribute_descriptions: Vertex::attribute_descriptions().to_vec()
            }
        },
        FragmentStageInfo {
            shader: textured_lit_frag.module.clone(),
            entry_point: textured_lit_frag.entry_point.clone(),
            color_blend_states: vec![
                PipelineColorBlendAttachmentState::builder()
                    .blend_enable(false)
                    .src_color_blend_factor(BlendFactor::SRC_COLOR)
                    .dst_color_blend_factor(BlendFactor::ONE_MINUS_DST_COLOR)
                    .color_blend_op(BlendOp::ADD)
                    .src_alpha_blend_factor(BlendFactor::ZERO)
                    .dst_alpha_blend_factor(BlendFactor::ZERO)
                    .alpha_blend_op(BlendOp::ADD)
                    .color_write_mask(ColorComponentFlags::RGBA)
                    .build()
            ],
            target_states: vec![
                Format::B8G8R8A8_UNORM
            ]
        },
        pipeline_layout
    ).expect("Graphics pipeline should be created");

    // Create linear image sampler
    let sampler = Sampler::new(
        (gpu_sampler::Filter::LINEAR, gpu_sampler::Filter::LINEAR),
        (gpu_sampler::SamplerAddressMode::REPEAT, gpu_sampler::SamplerAddressMode::REPEAT, gpu_sampler::SamplerAddressMode::REPEAT),
        Some(16.0),
        gpu_sampler::BorderColor::INT_OPAQUE_BLACK,
        false,
        None,
        gpu_sampler::SamplerMipmapMode::LINEAR,
        (0.0, 0.0, 0.0)
    );

    // Add internal assets to world
    commands.add(|world: &mut World| {
        // Add shader assets
        let mut shader_assets = world.resource_mut::<Assets<Shader>>();
        let mesh_vert_handle = shader_assets.add(mesh_vert);
        let unlit_frag_handle = shader_assets.add(unlit_frag);
        let textured_lit_frag_handle = shader_assets.add(textured_lit_frag);

        let mut shader_manager = world.resource_mut::<ShaderManager>();
        shader_manager.shaders.insert("mesh_vert".to_string(), mesh_vert_handle);
        shader_manager.shaders.insert("unlit_frag".to_string(), unlit_frag_handle);
        shader_manager.shaders.insert("textured_lit_frag".to_string(), textured_lit_frag_handle);

        // Add pipeline assets
        let mut pipeline_assets = world.resource_mut::<Assets<Pipeline>>();
        let unlit_pipeline_handle = pipeline_assets.add(unlit_pipeline);
        let textured_lit_pipeline_handle = pipeline_assets.add(textured_lit_pipeline);

        let mut pipeline_manager = world.resource_mut::<PipelineManager>();
        pipeline_manager.pipelines.insert("unlit_mesh".to_string(), unlit_pipeline_handle);
        pipeline_manager.pipelines.insert("textured_lit_mesh".to_string(), textured_lit_pipeline_handle);

        // Add sampler assets
        let mut sampler_assets = world.resource_mut::<Assets<Sampler>>();
        let asset_handle = sampler_assets.add(sampler);

        let mut sampler_manager = world.resource_mut::<SamplerManager>();
        sampler_manager.samplers.insert("Linear".to_string(), asset_handle);
    });
}