mod swapchain;
mod frame_data;

use swapchain::Swapchain;
use frame_data::FrameData;

use crate::device::Device;

use ash::extensions::khr;
use ash::vk;

use bevy_log::prelude::*;
use bevy_window::{PresentMode, RawHandleWrapper};

use std::{
    cell::RefCell,
    ops::Deref,
    slice,
    string::String
};



/// Internal data for the Vulkan surface.
///
/// [`Surface`] is the public API for interacting with the Vulkan surface.
pub struct Surface {
    device: Device,
    _present_queue_index: u32,

    surface: khr::Surface,
    surface_handle: vk::SurfaceKHR,

    swapchain: RefCell<Option<Swapchain>>,
    pub swapchain_semaphore: vk::Semaphore,

    frame_number: usize,
    frame_data: Vec<FrameData>,
}

impl Surface {
    pub fn new(
        device: Device,
        raw_handle: &RawHandleWrapper
    ) -> Self {
        let instance = &device.instance;
        
        // Select presentation queue for device
        // TODO: evaluate all queues and select best
        let present_queue_index = device.queues.graphics_family;

        // Create surface from window
        let surface = khr::Surface::new(&instance.entry, &instance);
        //  Safety: ash_window::create_surface
        //  In order for the created vk::SurfaceKHR to be valid for the duration of its usage, 
        //  the Instance this was called on must be dropped later than the resulting vk::SurfaceKHR.
        //
        //  Guaranteed by Surface retaining a reference to this Instance
        let surface_handle = unsafe { 
            ash_window::create_surface(&instance.entry, &instance, raw_handle.display_handle, raw_handle.window_handle, None)
                .expect("Surface::new: Surface creation failed")
        };

        // Create semaphore to sync swapchain image acquisition
        let create_info = vk::SemaphoreCreateInfo::builder();
        let swapchain_semaphore = match unsafe { device.create_semaphore(&create_info, None) } {
            Ok(result) => result,
            Err(error) => panic!("Surface::new: {}", error.to_string())
        };

        let frame_data: Vec<FrameData> = vec![];

        Self {
            device,
            _present_queue_index: present_queue_index,
            surface,
            surface_handle,
            swapchain: RefCell::new(None),
            swapchain_semaphore,
            frame_number: 0,
            frame_data
        }
    }

    // TODO: refactor to more elegantly handle errors
    pub fn configure(&mut self, present_mode: PresentMode, extent: vk::Extent2D) {
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
        
        // Get swapchain parameters
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

        // Create swapchain
        let swapchain = match Swapchain::new(self.device.clone(), self.surface_handle, selected_format, present_mode, surface_extent, capabilities.current_transform, image_count) {
            Ok(result) => result,
            Err(error) => panic!("Surface::configure: {}", error.to_string())
        };
        self.swapchain.replace(Some(swapchain));

        // Create frame data for handling frames-in-flight
        self.frame_data.clear();
        for _ in 0..image_count {
            self.frame_data.push(FrameData::new(self.device.clone()).expect("Surface::new: FrameData creation failed"));
        }
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
                Ok(result.image_extent)
            },
            None => Err("Surface::extent: Surface has no swapchain!".to_string())
        }
    }

    pub fn format(&self) -> Result<vk::Format, String> {
        let swapchain = self.swapchain.borrow();
        match swapchain.deref() {
            Some(result) => {
                Ok(result.image_format)
            },
            None => Err("Surface::format: Surface has no swapchain!".to_string())
        }
    }

    pub fn image(&self, index: u32) -> Result<vk::Image, String> {
        let swapchain = self.swapchain.borrow();
        match swapchain.deref() {
            Some(result) => {
                Ok(result.images[index as usize])
            },
            None => Err("Surface::format: Surface has no swapchain!".to_string())
        }
    }

    pub fn frame_count(&self) -> usize {
        self.frame_data.len()
    }

    pub fn frame_data(&self) -> &FrameData {
        &self.frame_data[self.frame_number]
    }

    // Wrap Vulkan methods

    pub fn acquire_next_image(&self, timeout: u64) -> Result<(u32, bool), String> {
        let swapchain = self.swapchain.borrow();
        match swapchain.deref() {
            Some(result) => {
                match unsafe { result.swapchain.acquire_next_image(result.handle, timeout, self.swapchain_semaphore, vk::Fence::null()) } {
                    Ok(result) => Ok(result),
                    Err(error) => Err(format!("Surface::acquire_next_image: {}", error))
                }
            },
            None => Err("Surface::acquire_next_image: Surface has no swapchain!".to_string())
        }
    }

    pub fn queue_present(&mut self, queue: vk::Queue, image_indices: &[u32]) -> Result<bool, String> {
        let frame_data = &self.frame_data[self.frame_number];
        let frame_count = self.frame_data.len();
        self.frame_number = (self.frame_number + 1) % frame_count;

        let swapchain = self.swapchain.borrow();
        if let Some(swapchain) = swapchain.deref() {

            let present_info = &vk::PresentInfoKHR::builder()
                .swapchains(slice::from_ref(&swapchain.handle))
                .wait_semaphores(slice::from_ref(&frame_data.render_semaphore))
                .image_indices(image_indices);

            match unsafe { swapchain.queue_present(queue, present_info) } {
                Ok(result) => return Ok(result),
                Err(error) => return Err(format!("Surface::queue_present: {}", error.to_string()))
            };
        }
        
        Ok(false)
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        // First drop any active swapchain
        let _ = self.swapchain.replace(None);

        info!("Dropping Surface!");
        unsafe {
            self.device.device_wait_idle().unwrap();
            
            self.device.destroy_semaphore(self.swapchain_semaphore, None);

            self.surface.destroy_surface(self.surface_handle, None);
        }
    }
}


