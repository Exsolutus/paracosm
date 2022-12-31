pub mod device;
pub mod instance;
//pub mod raster;
pub mod resource;
pub mod surface;
pub mod utils;

use ash::vk;

use bevy_app::Plugin;
use bevy_log::prelude::*;

use std::{ffi::CStr, ffi::CString};

// Expose glm
pub use nalgebra_glm as glm;


/// Vulkan abstractions exposed through Bevy plugin to provide flexible GPU access
#[derive(Default)]
pub struct GpuPlugin;

impl Plugin for GpuPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // Acquire application window
        let windows = app.world.resource_mut::<bevy_window::Windows>();
        let window = windows
            .get_primary()
            .expect("Failed to get application window!");
        let window_handle = window.raw_handle()
            .expect("Failed to get window handle!")
            .get_display_handle();
        let window_extensions = ash_window::enumerate_required_extensions(window_handle)
            .expect("Failed to get window extensions!")
            .to_vec();

        #[cfg(debug_assertions)]
        {
            info!("Application Window Extensions:");
            window_extensions
                .iter()
                .for_each(|name| info!("\t{:?}", unsafe { CStr::from_ptr(*name) }));
        }

        // Ash entry
        let entry = ash::Entry::linked();

        // Create Instance
        let api_version = match entry.try_enumerate_instance_version().unwrap() {
            // Vulkan 1.1+
            Some(version) => {
                let major = vk::api_version_major(version);
                let minor = vk::api_version_minor(version);
                let patch = vk::api_version_patch(version);
                info!("API Version: {}.{}.{}", major, minor, patch);
                if major < 1 || minor < 3 {
                    panic!("Vulkan API version 1.3 or greater is required!")
                }
                version
            }
            // Vulkan 1.0
            None => panic!("Vulkan API version 1.3 or greater is required!"),
        };
        let app_info = vk::ApplicationInfo::builder()
            .application_name(CString::new("Paracosm").unwrap().as_c_str())
            .application_version(vk::make_api_version(0, 0, 1, 0))
            .engine_name(CString::new("Paracosm").unwrap().as_c_str())
            .engine_version(vk::make_api_version(0, 0, 1, 0))
            .api_version(api_version)
            .build();
        let mut instance_extensions = window_extensions;
        let instance = match instance::Instance::new(entry, app_info, &mut instance_extensions) {
            Ok(result) => result,
            Err(error) => panic!("Instance creation failed: {}", error),
        };

        // Add Instance to main app as resource
        app.world.insert_resource(instance);
    }
}
