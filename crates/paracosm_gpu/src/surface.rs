use super::Device;

use ash::extensions::khr;
use ash::vk;

use bevy_log::prelude::*;
use bevy_window::{PresentMode, RawWindowHandleWrapper};

use std::{
    cell::RefCell,
    ops::Deref, 
    rc::Rc,
    slice,
    string::String
};

// TODO: cleanup supporting structs, possibly split files

pub(self) struct Swapchain {
    device: Device,

    pub swapchain: khr::Swapchain,
    handle: vk::SwapchainKHR,

    _image_format: vk::Format,
    _image_extent: vk::Extent2D,
    _images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,

    present_semaphore: vk::Semaphore,
    render_semaphore: vk::Semaphore
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

/// Internal data for the Vulkan surface.
///
/// [`Surface`] is the public API for interacting with the Vulkan surface.
pub struct SurfaceInternal {
    pub device: Device,
    _present_queue_index: u32,

    surface: khr::Surface,
    surface_handle: vk::SurfaceKHR,

    swapchain: RefCell<Option<Swapchain>>,
}

impl Drop for SurfaceInternal {
    fn drop(&mut self) {
        // First drop any active swapchain
        let _ = self.swapchain.replace(None);

        info!("Dropping Surface!");
        unsafe {
            self.surface.destroy_surface(self.surface_handle, None);
        }
    }
}

/// Public API for interacting with the Vulkan instance.
pub struct Surface {
    // Not Arc because surface should only be used on main thread
    internal: Rc<SurfaceInternal>,
}

impl Surface {
    pub fn new(
        device: Device,
        window_handle: &RawWindowHandleWrapper
    ) -> Result<Self, String> {
        let instance = &device.instance;
        let window_handle = unsafe { window_handle.get_handle() };

        // Create surface from window
        let surface = khr::Surface::new(&instance.entry, &instance);
        //  Safety: ash_window::create_surface
        //  In order for the created vk::SurfaceKHR to be valid for the duration of its usage, 
        //  the Instance this was called on must be dropped later than the resulting vk::SurfaceKHR.
        //
        //  Guaranteed by Surface retaining a reference to this Instance
        let surface_handle = match unsafe { ash_window::create_surface(&instance.entry, &instance, &window_handle, None) } {
            Ok(result) => result,
            Err(error) => return Err(format!("Surface::new: {}", error.to_string()))
        };

        // Select presentation queue for device
        // TODO: evaluate all queues and select best
        let present_queue_index = device.queues.graphics_family;

        Ok(Self {
            internal: Rc::new(SurfaceInternal {
                device,
                _present_queue_index: present_queue_index,
                surface,
                surface_handle,
                swapchain: RefCell::new(None),
            })
        })
    }

    // TODO: refactor to more elegantly handle errors
    pub fn configure(&self, present_mode: PresentMode, extent: vk::Extent2D, present_semaphore: vk::Semaphore, render_semaphore: vk::Semaphore) {
        // Drop any existing swapchain
        self.swapchain.replace(None);

        // Check swapchain support
        let capabilities = match unsafe { self.surface.get_physical_device_surface_capabilities(self.device.physical_device, self.surface_handle) } {
            Ok(result) => result,
            Err(error) => panic!("Surface::configure: {}", error.to_string())
        };
        let formats = match unsafe { self.surface.get_physical_device_surface_formats(self.device.physical_device, self.surface_handle) } {
            Ok(result) => result,
            Err(error) => panic!("Surface::configure: {}", error.to_string())
        };
        let present_modes = match unsafe { self.surface.get_physical_device_surface_present_modes(self.device.physical_device, self.surface_handle) } {
            Ok(result) => result,
            Err(error) => panic!("Surface::configure: {}", error.to_string())
        };

        if formats.is_empty() || present_modes.is_empty() {
            panic!("Surface::configure: {}", "Presentation to this window not supported by this device".to_string())
        }
        
        // Create swapchain
        let selected_format = *formats.iter().find(|format| {
            match (format.format, format.color_space) {
                (vk::Format::R8G8B8A8_SRGB, vk::ColorSpaceKHR::SRGB_NONLINEAR) => true,
                _ => false
            }
        })
        .or_else(|| {
            Some(&formats[0])
        })
        .unwrap();
        let present_mode = match present_mode {
            PresentMode::Fifo => vk::PresentModeKHR::FIFO,
            PresentMode::Mailbox => vk::PresentModeKHR::MAILBOX,
            PresentMode::Immediate => vk::PresentModeKHR::IMMEDIATE,
            PresentMode::AutoVsync => vk::PresentModeKHR::FIFO,
            PresentMode::AutoNoVsync => vk::PresentModeKHR::MAILBOX,
        };
        let surface_extent = match capabilities.current_extent.width {
            u32::MAX => extent,
            _ => capabilities.current_extent
        };

        let image_count = match capabilities.max_image_count > 0 && capabilities.max_image_count < capabilities.min_image_count + 1 {
            true => capabilities.max_image_count,
            false => capabilities.min_image_count + 1
        };

        let create_info = &vk::SwapchainCreateInfoKHR::builder()
            .surface(self.surface_handle)
            .min_image_count(image_count)
            .image_format(selected_format.format)
            .image_color_space(selected_format.color_space)
            .image_extent(surface_extent)
            .image_array_layers(1)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true);

        let swapchain = khr::Swapchain::new(&self.device.instance, &self.device);
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
                self.device.create_image_view(&create_info, None)
                    .expect("Surface::configure: Failed to create image view!")
            }
        })
        .collect();

        let _ = self.internal.swapchain.borrow_mut().insert(Swapchain {
            device: self.device.clone(),
            swapchain,
            handle: swapchain_handle,
            _image_format: selected_format.format,
            _image_extent: surface_extent,
            _images: images,
            image_views,
            present_semaphore,
            render_semaphore
        });
    }

    pub fn attachment_info(&self, image_index: u32, clear_value: vk::ClearValue) -> Result<vk::RenderingAttachmentInfo, String> {
        let swapchain = self.swapchain.borrow();
        match swapchain.deref() {
            Some(swapchain) => {
                let attachment_info = vk::RenderingAttachmentInfo::builder()
                    .image_view(swapchain.image_views[image_index as usize])
                    .image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
                    .load_op(vk::AttachmentLoadOp::CLEAR)
                    .store_op(vk::AttachmentStoreOp::STORE)
                    .clear_value(clear_value)
                    .build();

                Ok(attachment_info)
            },
            None => Err("Surface::attachment_info: Surface has no swapchain!".to_string())
        }
    }

    pub fn extent(&self) -> Result<vk::Extent2D, String> {
        let swapchain = self.swapchain.borrow();
        match swapchain.deref() {
            Some(result) => {
                Ok(result._image_extent)
            },
            None => Err("Surface::extent: Surface has no swapchain!".to_string())
        }
    }

    pub fn format(&self) -> Result<vk::Format, String> {
        let swapchain = self.swapchain.borrow();
        match swapchain.deref() {
            Some(result) => {
                Ok(result._image_format)
            },
            None => Err("Surface::format: Surface has no swapchain!".to_string())
        }
    }

    pub fn image(&self, index: u32) -> Result<vk::Image, String> {
        let swapchain = self.swapchain.borrow();
        match swapchain.deref() {
            Some(result) => {
                Ok(result._images[index as usize])
            },
            None => Err("Surface::format: Surface has no swapchain!".to_string())
        }
    }

    // Wrap Vulkan methods

    pub fn acquire_next_image(&self, timeout: u64) -> Result<(u32, bool), String> {
        let swapchain = self.swapchain.borrow();
        match swapchain.deref() {
            Some(result) => {
                match unsafe { result.swapchain.acquire_next_image(result.handle, timeout, result.present_semaphore, vk::Fence::null()) } {
                    Ok(result) => Ok(result),
                    Err(error) => Err(format!("Surface::acquire_next_image: {}", error))
                }
            },
            None => Err("Surface::acquire_next_image: Surface has no swapchain!".to_string())
        }
    }

    pub fn queue_present(&self, queue: vk::Queue, image_indices: &[u32]) -> Result<bool, String> {
        let swapchain = self.swapchain.borrow();
        if let Some(swapchain) = swapchain.deref() {
            let present_info = &vk::PresentInfoKHR::builder()
                .swapchains(slice::from_ref(&swapchain.handle))
                .wait_semaphores(slice::from_ref(&swapchain.render_semaphore))
                .image_indices(image_indices);

            match unsafe { swapchain.queue_present(queue, present_info) } {
                Ok(result) => return Ok(result),
                Err(error) => return Err(format!("Surface::queue_present: {}", error.to_string()))
            };
        }
        
        Ok(false)
    }

    #[inline]
    pub fn strong_count(&self) -> usize {
        Rc::strong_count(&self.internal)
    }
}

impl Deref for Surface {
    type Target = SurfaceInternal;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        info!("Dropping ref to Surface!");
    }
}
