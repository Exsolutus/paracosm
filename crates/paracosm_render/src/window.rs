use crate::{Extract, RenderApp, RenderStage};
use crate::raster::Renderer;

use ash::vk;
use ash::vk::Extent2D;

use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use bevy_log::prelude::*;
use bevy_window::{PresentMode, RawWindowHandleWrapper, WindowClosed, WindowId, Windows};

use paracosm_gpu::{Surface};

use std::collections::{HashMap, HashSet};
use std::ops::{Deref, DerefMut};

/// Token to ensure a system runs on the main thread.
#[derive(Default)]
pub struct NonSendMarker;

#[derive(SystemLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub enum WindowSystem {
    Prepare,
}

pub struct WindowRenderPlugin;

impl Plugin for WindowRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .init_resource::<ExtractedWindows>()
                .init_non_send_resource::<WindowSurfaces>()
                .init_resource::<NonSendMarker>()
                .add_system_to_stage(RenderStage::Extract, extract_windows)
                .add_system_to_stage(RenderStage::Prepare, prepare_windows.label(WindowSystem::Prepare));
        }
    }
}


pub struct ExtractedWindow {
    pub id: WindowId,
    pub handle: RawWindowHandleWrapper,
    pub extent: vk::Extent2D,
    pub present_mode: PresentMode,
    pub swapchain_image_index: Option<u32>,
    pub resized: bool,
    pub configured: bool
}

#[derive(Default)]
pub struct ExtractedWindows {
    pub windows: HashMap<WindowId, ExtractedWindow>
}

impl Deref for ExtractedWindows {
    type Target = HashMap<WindowId, ExtractedWindow>;

    fn deref(&self) -> &Self::Target {
        &self.windows
    }
}

impl DerefMut for ExtractedWindows {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.windows
    }
}

#[derive(Default)]
pub struct WindowSurfaces {
    pub surfaces: HashMap<WindowId, Surface>,
    configured_windows: HashSet<WindowId>
}

pub fn extract_windows(
    mut extracted_windows: ResMut<ExtractedWindows>,
    mut closed: Extract<EventReader<WindowClosed>>,
    windows: Extract<Res<Windows>>
) {
    windows.iter().for_each(|window| {
        let extent = Extent2D {
            width: window.physical_width().max(1),
            height: window.physical_height().max(1)
        };

        let mut extracted_window = extracted_windows
            .entry(window.id())
            .or_insert(ExtractedWindow {
                id: window.id(),
                handle: window.raw_window_handle(),
                extent,
                present_mode: window.present_mode(),
                swapchain_image_index: None,
                resized: false,
                configured: false
            });
        
        // Drop active swapchain frame
        extracted_window.swapchain_image_index = None;

        // Check for window resize
        extracted_window.resized = extent != extracted_window.extent;
        if extracted_window.resized {
            debug!(
                "Window size changed from {}x{} to {}x{}",
                extracted_window.extent.width,
                extracted_window.extent.height,
                extent.width,
                extent.height
            );

            extracted_window.extent = extent;
        }
    });

    closed.iter().for_each(|closed_window| {
        extracted_windows.remove(&closed_window.id);
    });
}

pub fn prepare_windows(
    _marker: NonSend<NonSendMarker>,
    mut windows: ResMut<ExtractedWindows>,
    mut window_surfaces: NonSendMut<WindowSurfaces>,
    renderer: Res<Renderer>
) {
    let window_surfaces = window_surfaces.deref_mut();
    windows.values_mut().for_each(|window| {
        let surface = window_surfaces.surfaces
            .entry(window.id)
            .or_insert_with(|| {
                match Surface::new(renderer.device.clone(), &window.handle) {
                    Ok(result) => result,
                    Err(error) => panic!("{}", error.to_string())
                }
            });

        if window_surfaces.configured_windows.insert(window.id) || window.resized {
            surface.configure(window.present_mode, window.extent, renderer.present_semaphore);
            window.configured = true;
        }

        let image_index = match surface.acquire_next_image(1000000000) {
            Ok(result) => {
                info!("Signaled present semaphore");
                result.0
            },
            // Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
            //     self.configure(window, result.present_semaphore);
            //     unsafe { result.swapchain.acquire_next_image(result.handle, timeout, result.present_semaphore, vk::Fence::null()) }
            // },
            Err(error) => return error!("{}", error.to_string())
        };

        window.swapchain_image_index = Some(image_index);
    })
}