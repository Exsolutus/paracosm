use crate::device::Device;
use crate::resource::image::*;

use anyhow::Result;
use ash::extensions::khr;
use ash::vk;

use bevy_log::prelude::*;

use std::{
    ops::Deref
};


pub(super) struct Swapchain {
    device: Device,

    pub swapchain: khr::Swapchain,
    pub handle: vk::SwapchainKHR,

    pub image_format: vk::Format,
    pub image_extent: vk::Extent2D,
    pub images: Vec<Image>,
    pub depth_images: Vec<Image>
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
    ) -> Result<Self> {
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
        let image_handles = match unsafe { swapchain.get_swapchain_images(swapchain_handle) } {
            Ok(result) => result,
            Err(error) => panic!("Surface::configure: {}", error.to_string())
        };
        let images: Vec<Image> = image_handles.iter().map(|&image| {
            let image_info = ImageInfo {
                image_type: ImageType::TYPE_2D,
                image_format: selected_format.format,
                image_extent: Extent3D { width: surface_extent.width, height: surface_extent.height, depth: 1 },
                mip_levels: 1,
                array_layers: 1,
                samples: SampleCountFlags::TYPE_1, // unused
                tiling: ImageTiling::OPTIMAL,  // unused
                usage: ImageUsageFlags::COLOR_ATTACHMENT, // unused
                aspect: ImageAspectFlags::COLOR,
                memory_location: MemoryLocation::Unknown  // unused
            };

            match Image::from_vk(&device, image, image_info) {
                Ok(result) => result,
                Err(error) => panic!("swapchain::new: {}", error.to_string())
            }
        })
        .collect();

        // Create depth images
        let mut depth_images: Vec<Image> = vec![];
        for i in 0..images.len() {
            let create_info = ImageInfo {
                image_type: ImageType::TYPE_2D,
                image_format: Format::D24_UNORM_S8_UINT,
                image_extent: Extent3D { width: surface_extent.width, height: surface_extent.height, depth: 1 },
                mip_levels: 1,
                array_layers: 1,
                samples: SampleCountFlags::TYPE_1,
                tiling: ImageTiling::OPTIMAL,
                usage: ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
                aspect: ImageAspectFlags::DEPTH | ImageAspectFlags::STENCIL,
                memory_location: MemoryLocation::GpuOnly
            };
            depth_images.push(device.create_image(format!("Depth Buffer {}", i).as_str(), create_info, None));
        }


        Ok(Self {
            device,
            swapchain,
            handle: swapchain_handle,
            image_format: selected_format.format,
            image_extent: surface_extent,
            images,
            // image_views,
            depth_images
        })
    }

    pub fn image_count(&self) -> usize {
        self.images.len()
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

            self.images.clear();
            self.destroy_swapchain(self.handle, None);
        }
    }
}
