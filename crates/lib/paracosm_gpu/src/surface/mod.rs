mod swapchain;
mod frame_data;

use swapchain::Swapchain;
use frame_data::FrameData;

use crate::device::Device;

use anyhow::{Result, bail};
use ash::extensions::khr;
use ash::vk;

use bevy_log::prelude::*;
use bevy_window::{PresentMode, RawHandleWrapper};

use std::{
    cell::RefCell,
    ops::Deref,
    slice,
    string::String, borrow::Borrow
};


/// Public API for interacting with the Vulkan surface.
pub struct Surface {
    device: Device,
    _present_queue_index: u32,

    surface: khr::Surface,
    surface_handle: vk::SurfaceKHR,

    swapchain: Option<RefCell<Swapchain>>,
    pub swapchain_semaphore: vk::Semaphore,

    frame_index: usize,
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
            swapchain: None,
            swapchain_semaphore,
            frame_index: 0,
            frame_data
        }
    }

    // TODO: refactor to more elegantly handle errors
    pub fn configure(&mut self, present_mode: PresentMode, extent: vk::Extent2D) {
        // Drop any existing swapchain
        self.swapchain = None;

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

        // Create frame data for frame-in-flight pipelining
        self.frame_data.clear();
        for _ in 0..swapchain.image_count() {
            self.frame_data.push(FrameData::new(self.device.clone()).expect("Surface::new: FrameData creation failed"));
        }

        self.swapchain = Some(RefCell::new(swapchain));
    }



    pub fn begin_rendering(&mut self) -> Result<vk::CommandBuffer> {
        let Some(swapchain) = &self.swapchain else {
            bail!("Surface has no swapchain!");
        };
        let swapchain = swapchain.borrow();

        let extent = swapchain.image_extent;
        let render_target = &swapchain.images[self.frame_index];
        let depth_target = &swapchain.depth_images[self.frame_index];
        
        // Get current frame data
        let frame_data = &self.frame_data[self.frame_index];

        unsafe {
            // Wait for frame-in-flight completion
            self.device.wait_for_fences(&[frame_data.in_flight_fence], true, 1000000000)?;
            self.device.reset_fences(&[frame_data.in_flight_fence])?;

            // Reset command buffer
            self.device.reset_command_buffer(frame_data.command_buffer, vk::CommandBufferResetFlags::empty())?;

            // Begin command recording
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            self.device.begin_command_buffer(frame_data.command_buffer, &begin_info)?;

            // Transition attachments layouts to optimal
            self.device.transition_image_layout(
                frame_data.command_buffer, 
                render_target, 
                vk::ImageLayout::UNDEFINED, 
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL
            )?;
            // TODO: depth target

            // Begin rendering
            let color_attachment_info = vk::RenderingAttachmentInfo::builder()
                .image_view(swapchain.images[self.frame_index].image_view)
                .image_layout(vk::ImageLayout::ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue { float32: [0.0, 0.0, 0.0, 1.0] }
                });
            let depth_attachment_info = vk::RenderingAttachmentInfo::builder()
                .image_view(swapchain.depth_images[self.frame_index].image_view)
                .image_layout(vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    depth_stencil: vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 }
                });
            let rendering_info = vk::RenderingInfo::builder()
                .render_area(vk::Rect2D::builder()
                    // Leave offset default
                    .extent(extent)
                    .build()
                )
                .layer_count(1)
                .color_attachments(slice::from_ref(&color_attachment_info));
                //.depth_attachment(&depth_attachment_info);
            self.device.cmd_begin_rendering(frame_data.command_buffer, &rendering_info);
        }

        Ok(frame_data.command_buffer)
    }

    pub fn end_rendering(&self, queue: vk::Queue) -> Result<()> {
        let Some(swapchain) = &self.swapchain else {
            bail!("Surface has no swapchain!");
        };
        let swapchain = swapchain.borrow();

        let extent = swapchain.image_extent;
        let render_target = &swapchain.images[self.frame_index];
        let depth_target = &swapchain.depth_images[self.frame_index];
        
        // Get current frame data
        let frame_data = &self.frame_data[self.frame_index];

        unsafe {
            // End rendering
            self.device.cmd_end_rendering(frame_data.command_buffer);
            
            // Transition attachments layouts to optimal
            self.device.transition_image_layout(
                frame_data.command_buffer, 
                render_target, 
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, 
                vk::ImageLayout::PRESENT_SRC_KHR
            )?;
            // TODO: depth target

            // End command recording
            self.device.end_command_buffer(frame_data.command_buffer)?;

            // Submit command buffer
            let submit_infos = &[
                vk::SubmitInfo::builder()
                    .wait_dst_stage_mask(slice::from_ref(&vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT))
                    .wait_semaphores(slice::from_ref(&self.swapchain_semaphore))
                    .signal_semaphores(slice::from_ref(&frame_data.render_semaphore))
                    .command_buffers(slice::from_ref(&frame_data.command_buffer))
                    .build()
            ];
            self.device.queue_submit(queue, submit_infos, frame_data.in_flight_fence)?
        }

        Ok(())
    }

    pub fn extent(&self) -> Result<vk::Extent2D> {
        let Some(swapchain) = &self.swapchain else {
            bail!("Surface has no swapchain!");
        };
        let swapchain = swapchain.borrow();

        Ok(swapchain.image_extent)
    }

    pub fn frame_count(&self) -> usize {
        self.frame_data.len()
    }

    pub fn frame_data(&self) -> &FrameData {
        &self.frame_data[self.frame_index]
    }

    // Wrap Vulkan methods

    pub fn acquire_next_image(&mut self, timeout: u64) -> Result<bool> {
        let Some(swapchain) = &self.swapchain else {
            bail!("Surface has no swapchain!");
        };
        let swapchain = swapchain.borrow();
        
        unsafe {
            let (index, suboptimal) = swapchain.acquire_next_image(swapchain.handle, timeout, self.swapchain_semaphore, vk::Fence::null())?;
            self.frame_index = index as usize;

            Ok(suboptimal)
        }
    }

    pub fn queue_present(&mut self, queue: vk::Queue) -> Result<bool> {
        let frame_data = &self.frame_data[self.frame_index];
        
        let Some(swapchain) = &self.swapchain else {
            bail!("Surface has no swapchain!");
        };
        let swapchain = swapchain.borrow();

        let indices = &[self.frame_index as u32];
        let present_info = &vk::PresentInfoKHR::builder()
            .swapchains(slice::from_ref(&swapchain.handle))
            .wait_semaphores(slice::from_ref(&frame_data.render_semaphore))
            .image_indices(indices);

        unsafe {
            swapchain.queue_present(queue, present_info)?;
        }

        Ok(false)
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        // First drop any active swapchain
        self.swapchain = None;

        info!("Dropping Surface!");
        unsafe {
            self.device.device_wait_idle().unwrap();
            
            self.device.destroy_semaphore(self.swapchain_semaphore, None);

            self.surface.destroy_surface(self.surface_handle, None);
        }
    }
}


