use crate::device::{Device, LogicalDevice};

use anyhow::{Context, Result};
use ash::vk::{
    Extent2D,
    Format
};

// Reexport
pub use ash::vk::{
    ImageUsageFlags as ImageUsage,
    PresentModeKHR as PresentMode,
    SurfaceTransformFlagsKHR as SurfaceTransform
};


pub trait HasSurfaceHandles: raw_window_handle::HasDisplayHandle + raw_window_handle::HasWindowHandle {}
impl<T> HasSurfaceHandles for T where T: raw_window_handle::HasDisplayHandle + raw_window_handle::HasWindowHandle {} 

pub type FormatSelector = fn(ash::vk::SurfaceFormatKHR) -> i32;

fn default_format_selector(format: ash::vk::SurfaceFormatKHR) -> i32 {
    match format.format {
        Format::R8G8B8A8_SRGB => 90,
        Format::R8G8B8A8_UNORM => 80,
        Format::B8G8R8A8_SRGB => 70,
        Format::B8G8R8A8_UNORM => 60,
        _ => 0
    }
}


pub struct SurfaceConfig {
    pub image_format_selector: FormatSelector,
    pub image_array_layers: u32,
    pub image_usage: ImageUsage,
    pub present_mode: PresentMode,
    pub pre_transform: SurfaceTransform,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            image_format_selector: default_format_selector,
            image_array_layers: 1,
            image_usage: ImageUsage::COLOR_ATTACHMENT,
            present_mode: PresentMode::FIFO,
            pre_transform: SurfaceTransform::IDENTITY
        }
    }
}


pub(crate) struct Surface {
    api: ash::khr::surface::Instance,
    pub surface: ash::vk::SurfaceKHR,
    surface_format: ash::vk::SurfaceFormatKHR,
    surface_extent: ash::vk::Extent2D,

    device: Option<*const LogicalDevice>,
    swapchain: Option<ash::vk::SwapchainKHR>,
}
unsafe impl Send for Surface {  }   // SAFETY: Surfaces are owned by the Context


impl std::ops::Deref for Surface {
    type Target = ash::khr::surface::Instance;

    fn deref(&self) -> &Self::Target {
        &self.api
    }
}

impl Surface {
    pub fn new(entry: &ash::Entry, instance: &ash::Instance, window: &dyn HasSurfaceHandles) -> Result<Self> {
        let api = ash::khr::surface::Instance::new(entry, instance);
        let surface = unsafe { 
            ash_window::create_surface(
                entry, 
                instance, 
                window.display_handle()?.as_raw(), 
                window.window_handle()?.as_raw(), 
                None
            )?
        };


        Ok(Self {
            api,
            surface,
            surface_format: ash::vk::SurfaceFormatKHR::default(),
            surface_extent: Extent2D::default(),
            device: None,
            swapchain: None
        })
    }

    pub fn configure(&mut self, device: &Device, config: SurfaceConfig) -> Result<()> {
        let surface_formats = unsafe { self.api.get_physical_device_surface_formats(**device, self.surface)? };
        let selected_format = surface_formats.iter()
            .max_by_key(|&&format| (config.image_format_selector)(format))
            .context("Physical device has no available surface formats.")?;


        let surface_capabilities = unsafe {
            self.api.get_physical_device_surface_capabilities(**device, self.surface)?
        };
        self.surface_extent = surface_capabilities.current_extent;

        let queue_family = [device.properties().queue.graphics_family];

        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(3)
            .image_format(selected_format.format)
            .image_color_space(selected_format.color_space)
            .image_extent(surface_capabilities.current_extent)
            .image_array_layers(config.image_array_layers)
            .image_usage(config.image_usage)
            .image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_family)
            .pre_transform(config.pre_transform)
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(config.present_mode)
            .clipped(true)
            .old_swapchain(self.swapchain.unwrap_or(ash::vk::SwapchainKHR::null()));
        let swapchain = unsafe {
            device.logical_device.swapchain.create_swapchain(&swapchain_create_info, None)?
        };

        self.device = Some(device.logical_device.as_ref());
        self.swapchain = Some(swapchain);

        Ok(())
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            if let (Some(device), Some(swapchain)) = (self.device, self.swapchain) {
                device.as_ref().unwrap().swapchain.destroy_swapchain(swapchain, None);
            }

            self.api.destroy_surface(self.surface, None);
        }
    }
}