use crate::instance::Instance;
use super::utils::vk_to_string;

use anyhow::{Context, Result};
use ash::extensions::khr;
use ash::vk;
use bevy_log::prelude::*;
use gpu_allocator::{vulkan as vk_alloc, AllocatorDebugSettings};
use raw_window_handle::HasRawWindowHandle;
use std::{cell::RefCell, ops::Deref, os::raw::c_char, sync::{Arc, Mutex}};


// TODO: Rework queue info once it's clear how they're used
pub enum QueueFamily {
    GRAPHICS,
    COMPUTE,
    TRANSFER,
}

pub struct DeviceQueues {
    pub graphics_family: u32,
    pub compute_family: u32,
    pub transfer_family: u32,

    pub graphics_count: u32,
    pub compute_count: u32,
    pub transfer_count: u32,

    pub present_family: Option<u32>
}

pub struct DeviceOptions<'a> {
    window_handle: Option<&'a dyn HasRawWindowHandle>,
    extensions: &'a [*const c_char],
    features: &'a mut vk::PhysicalDeviceFeatures2,
    queues: [(QueueFamily, &'a [f32]); 3],
}

/// Internal data for the Vulkan device.
///
/// [`Device`] is the public API for interacting with the Vulkan device.
pub struct DeviceInternal {
    pub(crate) instance: Instance,
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) logical_device: ash::Device,

    pub(crate) queues: DeviceQueues,
    pub(crate) transfer_queue: vk::Queue,
    pub(crate) transfer_pool: vk::CommandPool,

    pub(crate) allocator: Option<Mutex<vk_alloc::Allocator>>
}

impl Deref for DeviceInternal {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.logical_device
    }
}

impl Drop for DeviceInternal {
    fn drop(&mut self) {
        info!("Dropping Device!");

        self.allocator = None;
        //drop(allocator);
        unsafe {
            self.logical_device.destroy_command_pool(self.transfer_pool, None);
            self.logical_device.destroy_device(None);
        }
    }
}

/// Public API for interacting with the Vulkan device.
#[derive(Clone)]
pub struct Device {
    internal: Arc<DeviceInternal>,
}

impl Device {
    pub fn new(
        instance: Instance,
        selector: fn(vk::PhysicalDeviceProperties2) -> bool,
        options: DeviceOptions
    ) -> Result<Self> {
        info!("Creating Vulkan device");

        // Get candidate physical devices filtered by selector
        // let physical_devices: Vec<vk::PhysicalDevice> = match unsafe {
        //     instance.enumerate_physical_devices()
        // } {
        //     Ok(result) => result,
        //     Err(error) => return Err(error.to_string()),
        // }
        let physical_devices: Vec<vk::PhysicalDevice> = unsafe {
            instance.enumerate_physical_devices()
                .context("Failed to enumerate physical devices")?
        }
        .iter()
        .filter_map(|&physical_device| {
            let device_properties = &mut vk::PhysicalDeviceProperties2::default();
            unsafe { instance.get_physical_device_properties2(physical_device, device_properties) };

            #[cfg(debug_assertions)]
            info!(
                "\t{}",
                vk_to_string(&device_properties.properties.device_name)
            );

            match selector(*device_properties) {
                true => Some(physical_device),
                false => None,
            }
        })
        .collect();

        // Attempt logical device creation with candidate physical devices
        let result = physical_devices.iter().find_map(|&physical_device| {
            // Check for requested queues
            let available_queue_families = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

            let mut queues = DeviceQueues {
                graphics_family: u32::MAX,
                compute_family: u32::MAX,
                transfer_family: u32::MAX,

                graphics_count: 0,
                compute_count: 0,
                transfer_count: 0,

                present_family: Some(0)
            };
            available_queue_families
                .iter()
                .enumerate()
                .for_each(|(index, properties)| {
                    let index = index as u32;
                    match (
                        properties.queue_flags.contains(vk::QueueFlags::GRAPHICS),
                        properties.queue_flags.contains(vk::QueueFlags::COMPUTE),
                        properties.queue_flags.contains(vk::QueueFlags::TRANSFER),
                    ) {
                        (true, true, true) => queues.graphics_family = index,
                        (false, true, true) => queues.compute_family = index,
                        (false, false, true) => queues.transfer_family = index,
                        _ => warn!("Encountered unsupported device queue family!"),
                    }
                });
            // Handle missing queue families
            match (
                queues.graphics_family,
                queues.compute_family,
                queues.transfer_family,
            ) {
                (u32::MAX, _, _) => return None,
                (_, u32::MAX, _) => return None,
                (_, _, u32::MAX) => return None,
                _ => (),
            };

            // Check for presentation support on window, if requested
            // TODO: should consider checking all queue families
            match options.window_handle {
                Some(window_handle) => {
                    //let window_handle = unsafe { window.raw_window_handle().get_handle() };
                    let surface = khr::Surface::new(&instance.entry, &instance);
                    let surface_handle = match unsafe { ash_window::create_surface(&instance.entry, &instance, window_handle, None) } {
                        Ok(result) => result,
                        Err(_) => return None
                    };
                    match unsafe { surface.get_physical_device_surface_support(physical_device, queues.graphics_family, surface_handle) } {
                        Ok(_) => (),
                        Err(_) => return None
                    }

                    unsafe { surface.destroy_surface(surface_handle, None) };

                    queues.present_family = Some(queues.graphics_family)
                },
                None => ()
            };

            // Attempt logical device creation
            let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = options.queues
                .iter()
                .filter_map(|(queue_family, priorities)| match priorities.len() > 0 {
                    true => {
                        let index = match queue_family {
                            QueueFamily::GRAPHICS => {
                                queues.graphics_count = priorities.len() as u32;
                                queues.graphics_family
                            },
                            QueueFamily::COMPUTE => {
                                queues.compute_count = priorities.len() as u32;
                                queues.compute_family
                            },
                            QueueFamily::TRANSFER => {
                                queues.transfer_count = priorities.len() as u32;
                                queues.transfer_family
                            },
                        };
                        Some(
                            vk::DeviceQueueCreateInfo::builder()
                                .queue_family_index(index as u32)
                                .queue_priorities(priorities)
                                .build(),
                        )
                    }
                    false => None,
                })
                .collect();

            let create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(queue_create_infos.as_slice())
                .enabled_extension_names(options.extensions)
                .push_next(options.features);
            //  Safety: vkCreateDevice
            //  In order for the created Device to be valid for the duration of its usage,
            //  the Instance this was called on must be dropped later than the resulting Device.
            //
            //  Guaranteed by Device retaining a reference to this Instance
            let logical_device =
                match unsafe { instance.create_device(physical_device, &create_info, None) } {
                    Ok(logical_device) => logical_device,
                    Err(_) => return None,
                };

            Some((physical_device, logical_device, queues))
        });
        // let (physical_device, logical_device, queues) = match result {
        //     Some(result) => result,
        //     None => return Err("No suitable device found for requested parameters!".to_string()),
        // };
        let (physical_device, logical_device, queues) = result.context("No suitable device found for requested parameters!")?;


        // Get first transfer queue
        let transfer_queue = (0 < queues.transfer_count).then(|| {
            unsafe { logical_device.get_device_queue(queues.transfer_family, 0) }
        }).context(format!("Queue index out of range; index {}, queue count {}", 0, queues.transfer_count))?;

        // Create transfer command pool
        let create_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queues.transfer_family)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let transfer_pool = unsafe { logical_device.create_command_pool(&create_info, None)? };


        // Create memory allocator
        let allocator = gpu_allocator::vulkan::Allocator::new(
            &gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.deref().deref().clone(),
                device: logical_device.clone(),
                physical_device,
                debug_settings: AllocatorDebugSettings {
                    log_memory_information: false,
                    log_leaks_on_shutdown: true,
                    store_stack_traces: false,
                    log_allocations: true,
                    log_frees: true,
                    log_stack_traces: false
                },
                buffer_device_address: true
            }
        ).unwrap();

        Ok(Self {
            internal: Arc::new(DeviceInternal {
                instance,
                physical_device,
                logical_device,
                queues,
                transfer_queue,
                transfer_pool,
                allocator: Some(Mutex::new(allocator))
            }),
        })
    }

    pub fn primary(instance: Instance, window_handle: Option<&dyn HasRawWindowHandle>) -> Result<Self> {
        let mut dynamic_rendering_feature = vk::PhysicalDeviceDynamicRenderingFeatures::builder()
            .dynamic_rendering(true);
        let mut buffer_device_address_feature = vk::PhysicalDeviceBufferDeviceAddressFeatures::builder()
            .buffer_device_address(true);

        let options = DeviceOptions {
            window_handle,
            extensions: &[
                // Enable swapchain extension
                ash::extensions::khr::Swapchain::name().as_ptr(), //ash::extensions::khr::AccelerationStructure::name().as_ptr()
            ],
            features: &mut vk::PhysicalDeviceFeatures2::builder()
                .push_next(&mut dynamic_rendering_feature)
                .push_next(&mut buffer_device_address_feature),
            queues: [
                (QueueFamily::GRAPHICS, &[1.0]),
                (QueueFamily::COMPUTE, &[1.0]),
                (QueueFamily::TRANSFER, &[1.0]),
            ],
        };

        Self::new(
            instance,
            |properties| {
                // Select a discrete GPU with Vulkan 1.3 support
                let base_properties = properties.properties;
                let _major_version = vk::api_version_major(base_properties.api_version);
                let _minor_version = vk::api_version_minor(base_properties.api_version);
                let _patch_version = vk::api_version_patch(base_properties.api_version);

                let properties_check = _major_version == 1
                    && _minor_version >= 3
                    && base_properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU;

                properties_check
            },
            options 
        )
    }

    pub fn graphics_queue(&self, queue_index: u32) -> Result<vk::Queue> {
        (queue_index < self.queues.graphics_count).then(|| {
            unsafe { self.get_device_queue(self.queues.graphics_family, queue_index) }
        })
        .context(format!("Queue index out of range; index {}, queue count {}", queue_index, self.queues.graphics_count))
    }

    pub fn transfer_queue(&self, queue_index: u32) -> Result<vk::Queue> {
        if queue_index == 0 {
            return Ok(self.transfer_queue);
        }

        (queue_index < self.queues.transfer_count).then(|| {
            unsafe { self.get_device_queue(self.queues.transfer_family, queue_index) }
        })
        .context(format!("Queue index out of range; index {}, queue count {}", queue_index, self.queues.transfer_count))
    }

    #[inline]
    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.internal)
    }
}

impl Deref for Device {
    type Target = DeviceInternal;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.internal
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        info!("Dropping ref to Device!");
    }
}
