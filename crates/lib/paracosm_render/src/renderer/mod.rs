use crate::{
    image::*, 
    mesh::*,
    Pipeline,
    PipelineManager,
    render_asset::RenderAssets,
    render_resource::ResourceManager,
    window::WindowSurfaces
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
        sampler as gpu_sampler
    }
};

use rust_shaders_shared::glam;

use std::{
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
#[derive(Resource)]
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
    commands.insert_resource(
        RenderContext {
            device,
            resource_manager,
        }
    );

    // Create image samplers
    commands.add(|world: &mut World| {
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

        let mut sampler_assets = world.get_resource_mut::<Assets<Sampler>>()
            .expect("Sampler assets resource should exist");
        let asset_handle = sampler_assets.add(sampler);

        let mut sampler_manager = world.get_resource_mut::<SamplerManager>()
            .expect("SamplerManager should exist");
        sampler_manager.samplers.insert("Linear".to_string(), asset_handle);
    });

    // Insert SceneData
    commands.insert_resource(
        SceneData {
            object_buffers: vec![],
        }
    );
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
