use crate::device::Device;

use ash::extensions::khr;
use ash::vk;

use bevy_log::prelude::*;

use std::{
    ops::Deref,
    string::String
};


pub(super) struct Swapchain {
    device: Device,

    pub swapchain: khr::Swapchain,
    pub handle: vk::SwapchainKHR,

    pub image_format: vk::Format,
    pub image_extent: vk::Extent2D,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>
}

impl Swapchain {
    pub fn new(
        device: Device,
        surface_handle: vk::SurfaceKHR,
        selected_format: vk::SurfaceFormatKHR,
        present_mode: vk::PresentModeKHR,
        surface_extent: vk::Extent2D,
        surface_transform: vk::SurfaceTransformFlagsKHR,
        image_count: u32
    ) -> Result<Self, String> {
        let create_info = &vk::SwapchainCreateInfoKHR::builder()
            .surface(surface_handle)
            .min_image_count(image_count)
            .image_format(selected_format.format)
            .image_color_space(selected_format.color_space)
            .image_extent(surface_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        let swapchain = khr::Swapchain::new(&device.instance, &device);
        let swapchain_handle = match unsafe { swapchain.create_swapchain(create_info, None) } {
            Ok(result) => result,
            Err(error) => panic!("Surface::configure: {}", error.to_string())
        };

        // Get images and create image views
        let images = match unsafe { swapchain.get_swapchain_images(swapchain_handle) } {
            Ok(result) => result,
            Err(error) => panic!("Surface::configure: {}", error.to_string())
        };
        let image_views: Vec<vk::ImageView> = images.iter().map(|image| {
            let create_info = vk::ImageViewCreateInfo::builder()
                .image(*image)
                .view_type(vk::ImageViewType::TYPE_2D)
                .format(selected_format.format)
                // default components
                .subresource_range(
                    vk::ImageSubresourceRange::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .base_mip_level(0)
                        .level_count(1)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build()
                );
            unsafe {
                device.create_image_view(&create_info, None)
                    .expect("Surface::configure: Failed to create image view!")
            }
        })
        .collect();

        Ok(Self {
            device,
            swapchain,
            handle: swapchain_handle,
            image_format: selected_format.format,
            image_extent: surface_extent,
            images,
            image_views
        })
    }
}

impl Deref for Swapchain {
    type Target = khr::Swapchain;

    fn deref(&self) -> &Self::Target {
        &self.swapchain
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        info!("Dropping Swapchain!");

        unsafe {
            let _ = self.device.device_wait_idle();

            self.image_views.iter().for_each(|image_view| {
                self.device.destroy_image_view(*image_view, None)
            });
            self.destroy_swapchain(self.handle, None);
        }
    }
}
