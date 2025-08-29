use std::{marker::PhantomData, u64};

use crate::{device::{Device, LogicalDevice}, node::resource::ResourceIndex, resource::{ResourceLabel, ResourceManager}};

use anyhow::{bail, Context, Ok, Result};
use ash::vk::{
    Format
};

use std::any::Any;

// Reexport
pub use ash::vk::{
    ImageUsageFlags as ImageUsage,
    PresentModeKHR as PresentMode,
    SurfaceTransformFlagsKHR as SurfaceTransform
};


const MAX_FRAMES_IN_FLIGHT: u32 = 2;


pub trait SurfaceLabel: ResourceLabel { }

pub struct PrimarySurface;
impl SurfaceLabel for PrimarySurface { }
impl ResourceLabel for PrimarySurface { }

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
    //pub image_usage: ImageUsage,
    pub present_mode: PresentMode,
    pub pre_transform: SurfaceTransform,

    #[cfg(debug_assertions)] pub debug_name: &'static str
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            image_format_selector: default_format_selector,
            image_array_layers: 1,
            //image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
            present_mode: PresentMode::FIFO,
            pre_transform: SurfaceTransform::IDENTITY,
            #[cfg(debug_assertions)] debug_name: "Window Surface"
        }
    }
}

pub(crate) struct Surface {
    device: *const LogicalDevice,
    api: ash::khr::surface::Instance,

    pub surface: ash::vk::SurfaceKHR,
    pub swapchain: ash::vk::SwapchainKHR,
    pub images: Vec<ash::vk::Image>,
    pub acquire_semaphores: [ash::vk::Semaphore; MAX_FRAMES_IN_FLIGHT as usize],
    pub submit_semaphores: Box<[ash::vk::Semaphore]>,

    pub frame_index: u32,
    pub image_index: u32,
    pub extent: ash::vk::Extent2D
}

impl Surface {
    pub fn configure(&mut self, device: &Device, config: SurfaceConfig) -> Result<()> {
        // Create new swapchain for current device and config
        let surface_formats = unsafe { self.api.get_physical_device_surface_formats(**device, self.surface)? };
        let selected_format = surface_formats.iter()
            .max_by_key(|&&format| (config.image_format_selector)(format))
            .context("Physical device has no available surface formats.")?;

        let surface_capabilities = unsafe {
            self.api.get_physical_device_surface_capabilities(**device, self.surface)?
        };

        let queue_family = [device.properties().queue.graphics_family];

        let swapchain_create_info = ash::vk::SwapchainCreateInfoKHR::default()
            .surface(self.surface)
            .min_image_count(3)
            .image_format(selected_format.format)
            .image_color_space(selected_format.color_space)
            .image_extent(surface_capabilities.current_extent)
            .image_array_layers(config.image_array_layers)
            .image_usage(ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST)
            .image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE)
            .queue_family_indices(&queue_family)
            .pre_transform(config.pre_transform)
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(config.present_mode)
            .clipped(true)
            .old_swapchain(self.swapchain);
        let swapchain = unsafe {
            device.logical_device.swapchain.create_swapchain(&swapchain_create_info, None)?
        };

        // Clean up old swapchain if any
        unsafe {
            if let Some(device) = self.device.as_ref() { // use device old swapchain was created with
                device.swapchain.destroy_swapchain(self.swapchain, None);
            }
        }

        // Gather swapchain images
        let images = unsafe { device.logical_device.swapchain.get_swapchain_images(swapchain)? };
        
        // Create submit semaphores
        let mut submit_semaphores = vec![ash::vk::Semaphore::null(); images.len()];
        for i in 0..images.len() {
            submit_semaphores[i] = unsafe { device.logical_device.create_semaphore(&ash::vk::SemaphoreCreateInfo::default(), None)? }
        }

        self.device = device.logical_device.as_ref();
        self.swapchain = swapchain;
        self.images = images;
        self.submit_semaphores = submit_semaphores.into();
        self.extent = surface_capabilities.current_extent;

        #[cfg(debug_assertions)]
        unsafe {
            let swapchain_name = std::ffi::CString::new(format!("Swapchain: {}", config.debug_name))?;
            let swapchain_name_info = ash::vk::DebugUtilsObjectNameInfoEXT::default()
                .object_handle(swapchain)
                .object_name(&swapchain_name);
            device.logical_device.debug_utils.set_debug_utils_object_name(&swapchain_name_info)?;
        }

        Ok(())
    }

    pub fn acquire(&mut self) -> Result<(ash::vk::ImageMemoryBarrier2, ash::vk::Semaphore, ash::vk::Semaphore)> {
        let device = unsafe { self.device.as_ref().unwrap() };
        
        let acquire_semaphore = self.acquire_semaphores[self.frame_index as usize];
        let (image_index, suboptimal) = unsafe { device.swapchain.acquire_next_image(self.swapchain, u64::MAX, acquire_semaphore, ash::vk::Fence::null())? };

        // TODO: handle suboptimal swapchain

        let barrier = ash::vk::ImageMemoryBarrier2::default()
            .src_stage_mask(ash::vk::PipelineStageFlags2::ALL_COMMANDS)
            .src_access_mask(ash::vk::AccessFlags2::MEMORY_WRITE)
            .dst_stage_mask(ash::vk::PipelineStageFlags2::ALL_COMMANDS)
            .dst_access_mask(ash::vk::AccessFlags2::MEMORY_WRITE | ash::vk::AccessFlags2::MEMORY_READ)
            .old_layout(ash::vk::ImageLayout::UNDEFINED)
            .new_layout(ash::vk::ImageLayout::GENERAL)
            .image(self.images[image_index as usize])
            .subresource_range(
                ash::vk::ImageSubresourceRange::default()
                    .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                    .base_array_layer(0)
                    .layer_count(ash::vk::REMAINING_ARRAY_LAYERS)
                    .base_mip_level(0)
                    .level_count(ash::vk::REMAINING_MIP_LEVELS)
            );

        self.image_index = image_index;

        Ok((barrier, acquire_semaphore, self.submit_semaphores[image_index as usize]))
    }

    pub fn finish(&mut self) -> Result<ash::vk::ImageMemoryBarrier2> {
        let barrier = ash::vk::ImageMemoryBarrier2::default()
                    .src_stage_mask(ash::vk::PipelineStageFlags2::ALL_COMMANDS)
                    .src_access_mask(ash::vk::AccessFlags2::MEMORY_WRITE)
                    .dst_stage_mask(ash::vk::PipelineStageFlags2::ALL_COMMANDS)
                    .dst_access_mask(ash::vk::AccessFlags2::MEMORY_WRITE | ash::vk::AccessFlags2::MEMORY_READ)
                    .old_layout(ash::vk::ImageLayout::GENERAL)
                    .new_layout(ash::vk::ImageLayout::PRESENT_SRC_KHR)
                    .image(self.images[self.image_index as usize])
                    .subresource_range(
                        ash::vk::ImageSubresourceRange::default()
                            .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                            .base_array_layer(0)
                            .layer_count(ash::vk::REMAINING_ARRAY_LAYERS)
                            .base_mip_level(0)
                            .level_count(ash::vk::REMAINING_MIP_LEVELS)
                    );

        Ok(barrier)
    }

    pub fn present(&mut self, queue: ash::vk::Queue) -> Result<()> {
        let device = unsafe { self.device.as_ref().unwrap() };

        let present_info = ash::vk::PresentInfoKHR::default()
            .swapchains(std::slice::from_ref(&self.swapchain))
            .image_indices(std::slice::from_ref(&self.image_index))
            .wait_semaphores(std::slice::from_ref(&self.submit_semaphores[self.image_index as usize]));

        let suboptimal = unsafe { device.swapchain.queue_present(queue, &present_info)? };
        
        // TODO: handle suboptimal swapchain

        self.frame_index = (self.frame_index + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(())
    }
}

impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            if let Some(device) = self.device.as_ref() {
                for semaphore in &self.acquire_semaphores {
                    device.destroy_semaphore(*semaphore, None);
                }
                for semaphore in &self.submit_semaphores {
                    device.destroy_semaphore(*semaphore, None);
                }
                
                device.swapchain.destroy_swapchain(self.swapchain, None);
            }

            self.api.destroy_surface(self.surface, None);
        }
    }
}


impl crate::context::Context {
    pub fn create_surface<L: SurfaceLabel + 'static>(
        &mut self,
        label: L, 
        window: impl HasSurfaceHandles, 
        config: SurfaceConfig
    ) -> Result<()> {
        let device = &mut self.devices[self.primary_device as usize];

        // Create Surface for window
        let api = ash::khr::surface::Instance::new(&self.entry, &self.instance);
        let surface = unsafe { 
            ash_window::create_surface(
                &self.entry, 
                &self.instance, 
                window.display_handle()?.as_raw(), 
                window.window_handle()?.as_raw(), 
                None
            )?
        };

        let acquire_semaphores = unsafe { [
            device.logical_device.create_semaphore(&ash::vk::SemaphoreCreateInfo::default(), None)?,
            device.logical_device.create_semaphore(&ash::vk::SemaphoreCreateInfo::default(), None)?
        ] };

        let mut surface = Surface {
            device: device.logical_device.as_ref(),
            api, 
            surface,
            swapchain: ash::vk::SwapchainKHR::null(),
            images: vec![],
            acquire_semaphores,
            submit_semaphores: Default::default(),
            frame_index: 0,
            image_index: 0,
            extent: Default::default()
        };

        surface.configure(&device, config)?;

        
        let mut resource_manager = device.graph_world.resource_mut::<ResourceManager>();
        resource_manager.surfaces.resources.insert(label.type_id(), surface);

        device.graph_world.insert_resource(ResourceIndex::<L> { index: 0, _marker: PhantomData::default() });

        Ok(())
    }
}