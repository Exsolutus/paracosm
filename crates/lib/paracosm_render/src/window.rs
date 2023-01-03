
use ash::vk::Extent2D;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_window::{WindowClosed, WindowId, WindowResized, Windows};

use paracosm_gpu::{device::Device, surface::Surface};

use std::collections::{HashMap, HashSet};

/// Token to ensure a system runs on the main thread.
#[derive(Default, Resource)]
pub struct NonSendMarker;


pub struct WindowRenderPlugin;

impl Plugin for WindowRenderPlugin {
    fn build(&self, app: &mut App) {
        app.init_non_send_resource::<WindowSurfaces>()
            .init_resource::<NonSendMarker>()
            .add_system(process_windows);
    }
}

#[derive(Default)]
pub struct WindowSurfaces {
    pub surfaces: HashMap<WindowId, Surface>,
    pub configured_windows: HashSet<WindowId>
}

// Window Systems

pub fn process_windows(
    // By accessing a NonSend resource, we tell the scheduler to put this system on the main thread,
    // which is necessary for some OS s
    _marker: NonSend<NonSendMarker>,
    device: Res<Device>,
    windows: Res<Windows>,
    mut window_surfaces: NonSendMut<WindowSurfaces>,
    mut resized: EventReader<WindowResized>,
    mut closed: EventReader<WindowClosed>,
) {
    // Check for resized windows
    resized.iter().for_each(|resized_window| {
        debug!("Window {} resized to {} x {}", resized_window.id, resized_window.width, resized_window.height);

        // Ensure surface will be reconfigured
        window_surfaces.configured_windows.remove(&resized_window.id);
    });

    // Process closed windows
    let closed_windows: HashSet<WindowId> = closed.iter().map(|closed_window| {
        // Drop surface for closed window
        window_surfaces.surfaces.remove(&closed_window.id);
        window_surfaces.configured_windows.remove(&closed_window.id);

        closed_window.id
    })
    .collect();

    // Process windows
    windows.iter().for_each(|window| {
        // Skip window if it was closed
        if closed_windows.contains(&window.id()) {
            return
        }

        let extent = Extent2D {
            width: window.physical_width().max(1), 
            height: window.physical_height().max(1)
        };

        // Create window surface if needed
        window_surfaces.surfaces
            .entry(window.id())
            .or_insert_with(|| {
                Surface::new(device.clone(), &window.raw_handle().unwrap())
            });

        // Configure window surface if needed
        if window_surfaces.configured_windows.insert(window.id()) {
            if let Some(surface) = window_surfaces.surfaces.get_mut(&window.id()) {
                surface.configure(window.present_mode(), extent);
            }
        }

        // TODO: consider moving swapchain image acquisition closer to surface present
        let surface = window_surfaces.surfaces.get_mut(&window.id()).unwrap();
        if let Err(error) = surface.acquire_next_image(1000000000) {
            error!("process_windows: {}", error.to_string());
        }
    });
}
