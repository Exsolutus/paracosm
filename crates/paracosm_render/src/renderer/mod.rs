use crate::window::{ExtractedWindows, WindowSurfaces};

use ash::vk;

use bevy_ecs::world::World;
use bevy_log::prelude::*;
use bevy_window::Window;

use paracosm_gpu::{Instance, Device, Surface};

use std::slice;

pub fn render_system(world: &mut World) {
    //let device = world.resource::<Device>();
    let queue = world.resource::<vk::Queue>();
    //let sync_structures = world.resource::<SyncStructures>();
    let window_surfaces = world.non_send_resource::<WindowSurfaces>();

    {
        //let _span = info_span!("present_frames").entered();

        // // Remove ViewTarget components to ensure swap chain TextureViews are dropped.
        // // If all TextureViews aren't dropped before present, acquiring the next swap chain texture will fail.
        // let view_entities = world
        //     .query_filtered::<Entity, With<ViewTarget>>()
        //     .iter(world)
        //     .collect::<Vec<_>>();
        // for view_entity in view_entities {
        //     world.entity_mut(view_entity).remove::<ViewTarget>();
        // }

        let windows = world.resource::<ExtractedWindows>();
        for window in windows.values() {
            if let Some(image_index) = window.swapchain_image_index {
                // Get surface and swapchain for window
                let surface: &Surface = match window_surfaces.surfaces.get(&window.id) {
                    Some(result) => result,
                    None => continue
                };

                // // Present rendered image to surface
                match surface.queue_present(*queue, slice::from_ref(&image_index)) {
                    Err(error) => return error!("{}", error),
                    _ => ()
                };
            }
        }
    }
}

type RenderContext = (Device, vk::Queue, SyncStructures);

pub fn initialize_renderer(
    instance: Instance, 
    window: &Window
) -> Result<RenderContext, String> {
    // Create Device
    let device = match Device::primary(instance, Some(window)) {
        Ok(result) => result,
        Err(error) => return Err(error.to_string()),
    };

    // Get first Graphics queue
    let queue = unsafe { device.get_device_queue(device.queues.graphics_family, 0) };

    // Create sync structures
    let create_info = vk::FenceCreateInfo::builder()
        .flags(vk::FenceCreateFlags::SIGNALED);
    let render_fence = match unsafe { device.create_fence(&create_info, None) } {
        Ok(result) => result,
        Err(error) => return Err(error.to_string())
    };
    let create_info = vk::SemaphoreCreateInfo::builder()
        .flags(vk::SemaphoreCreateFlags::empty());
    let render_semaphore = match unsafe { device.create_semaphore(&create_info, None) } {
        Ok(result) => result,
        Err(error) => return Err(error.to_string())
    };
    let present_semaphore = match unsafe { device.create_semaphore(&create_info, None) } {
        Ok(result) => result,
        Err(error) => return Err(error.to_string())
    };

    let sync_structures = SyncStructures {
        device: device.clone(),
        render_fence,
        render_semaphore,
        present_semaphore
    };

    Ok((device, queue, sync_structures))
}

// TODO: Maybe replace this with abstraction(s) in paracosm_gpu
pub struct SyncStructures {
    device: Device,
    pub render_fence: vk::Fence,
    pub render_semaphore: vk::Semaphore,
    pub present_semaphore: vk::Semaphore
}

impl Drop for SyncStructures {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();

            self.device.destroy_semaphore(self.present_semaphore, None);
            self.device.destroy_semaphore(self.render_semaphore, None);
            self.device.destroy_fence(self.render_fence, None);
        }
    }
}
