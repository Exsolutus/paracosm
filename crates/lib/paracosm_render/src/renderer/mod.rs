use crate::window::WindowSurfaces;
use crate::{
    mesh::*, 
    //image::*, 
    Pipeline, 
    PipelineManager
};


use ash::vk;

use bevy_asset::prelude::*;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_time::prelude::*;
use bevy_window::Windows;

use paracosm_gpu::{
    instance::Instance, 
    device::{Device, Queue}, 
    resource::ResourceManager,
};

use rust_shaders_shared::{self, glam};



/// The [`RenderContext`] resource provides access to the renderer's GPU resources
#[derive(Resource)]
pub struct RenderContext {
    pub device: Device,
    pub resource_manager: ResourceManager
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

    // Create GPU resource manager
    let resource_manager = ResourceManager::new(&device)
        .expect("A ResourceManager should be created for the Device");

    commands.insert_resource(
        RenderContext {
            device,
            resource_manager
        }
    )
}



// Renderer main loop
pub fn render_system(
    render_context: Res<RenderContext>,
    windows: Res<Windows>,
    mut window_surfaces: NonSendMut<WindowSurfaces>,
    pipeline_handles: Res<PipelineManager>,
    pipeline_assets: Res<Assets<Pipeline>>,
    mesh_handles: Res<MeshManager>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    // image_handles: Res<ImageManager>,
    // mut image_assets: ResMut<Assets<Image>>,
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

        // Do rendering tasks
        if let Some(Pipeline::Graphics(pipeline)) = match pipeline_handles.pipelines.get("mesh_pipeline") {
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

                let mesh_handle = mesh_handles.meshes.get("square");
                if let Some(mesh) = match mesh_handle {
                    Some(value) => mesh_assets.get_mut(value),
                    None => None
                } {
                    mesh.upload(&device).unwrap();

                    match mesh.bind(&device, command_buffer) {
                        Ok(_) => (),
                        Err(error) => return error!("Renderer::render_system: {}", error)
                    };

                    // Mesh A
                    let mut proj = glam::Mat4::perspective_infinite_rh(
                        45_f32.to_radians(), 
                        extent.width as f32 / extent.height as f32, 
                        0.1, 
                    );
                    proj.y_axis *= -1.0;
                    let view = glam::Mat4::look_at_rh(
                        glam::vec3(2.0, 2.0, 2.0), 
                        glam::Vec3::ZERO,
                        glam::Vec3::Y
                    );

                    let model = glam::Mat4::from_scale_rotation_translation(
                        glam::Vec3::ONE, 
                        glam::Quat::from_axis_angle(glam::Vec3::Y, time.elapsed_seconds() * 45_f32.to_radians()), 
                        glam::Vec3::ZERO
                    );

                    let render_matrix = proj * view * model;

                    let push_constant = [rust_shaders_shared::ShaderConstants {
                        render_matrix,
                        mesh_handle: 0
                    }];
                    let (_, push_constant_bytes, _) = push_constant.align_to::<u8>();

                    device.cmd_push_constants(command_buffer, pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, push_constant_bytes);
                    device.cmd_draw_indexed(command_buffer, mesh.index_count() as u32, 1, 0, 0, 0);

                    // Mesh B
                    let model = glam::Mat4::from_scale_rotation_translation(
                        glam::Vec3::ONE, 
                        glam::Quat::from_axis_angle(glam::Vec3::Y, time.elapsed_seconds() * -45_f32.to_radians()), 
                        glam::vec3(0.0, -0.2, 0.0)
                    );

                    let render_matrix = proj * view * model;
    
                    let push_constant = [rust_shaders_shared::ShaderConstants {
                        render_matrix,
                        mesh_handle: 0
                    }];
                    let (_, push_constant_bytes, _) = push_constant.align_to::<u8>();

                    device.cmd_push_constants(command_buffer, pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, push_constant_bytes);
                    device.cmd_draw_indexed(command_buffer, mesh.index_count() as u32, 1, 0, 0, 0);
                }

                // let image_handle = image_handles.images.get("test");
                // if let Some(image) = match image_handle {
                //     Some(value) => image_assets.get_mut(value),
                //     None => None
                // } {
                //     let skipped = image.upload(&device).unwrap();

                //     // if !skipped {
                //     //     // Prepare image for use in shaders
                //     //     match device.transition_image_layout(
                //     //         frame_data.command_buffer,
                //     //         image.gpu_image.as_ref().unwrap(),
                //     //         gpu_image::Format::R8G8B8A8_SRGB,
                //     //         gpu_image::ImageLayout::TRANSFER_DST_OPTIMAL,
                //     //         gpu_image::ImageLayout::SHADER_READ_ONLY_OPTIMAL
                //     //     ) {
                //     //         Ok(_) => (),
                //     //         Err(error) => return error!("Renderer::render_system: {}", error)
                //     //     };
                //     // }
                // }
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

