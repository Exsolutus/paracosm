
use super::Instance;
use super::utils::vk_to_string;

use ash::vk;

use bevy_log::prelude::*;

use std::{
    ops::Deref,
    os::raw::c_char,
    sync::Arc
};

// TODO: Rework queue info once it's clear how they're used
pub enum QueueFamily {
    GRAPHICS,
    COMPUTE,
    TRANSFER
}

pub struct DeviceQueues {
    graphics_family: usize,
    compute_family: usize,
    transfer_family: usize
}

/// Internal data for the Vulkan device.
///
/// [`Device`] is the public API for interacting with the Vulkan device.
pub struct DeviceInternal {
    pub instance: Instance,
    pub physical_device: vk::PhysicalDevice,
    logical_device: ash::Device,
    pub queues: DeviceQueues
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

        unsafe {
            self.logical_device.destroy_device(None);
        }
    }
}


/// Public API for interacting with the Vulkan instance.
#[derive(Clone)]
pub struct Device {
    internal: Arc<DeviceInternal>
}

impl Device {
    pub fn new(
        instance: Instance,
        selector: fn(vk::PhysicalDeviceProperties2) -> bool,
        requested_extensions: &[*const c_char],
        requested_features: &vk::PhysicalDeviceFeatures2,
        requested_queues: [(QueueFamily, &[f32]); 3]
    ) -> Result<Self, String> {
        info!("Creating Vulkan device");

        // Get candidate physical devices filtered by selector
        let physical_devices: Vec<vk::PhysicalDevice> = match unsafe { instance.enumerate_physical_devices() } {
            Ok(result) => result,
            Err(error) => return Err(error.to_string())
        }
        .iter().filter_map(|&physical_device| {
            let device_properties = &mut vk::PhysicalDeviceProperties2::default();
            unsafe { instance.get_physical_device_properties2(physical_device, device_properties) };

            #[cfg(debug_assertions)]
            info!("{}", vk_to_string(&device_properties.properties.device_name));

            match selector(*device_properties) {
                true => Some(physical_device),
                false => None
            }
        })
        .collect();

        // Attempt logical device creation with candidate physical devices
        let result = physical_devices.iter().find_map(|&physical_device| {
            // Check for requested queues
            let available_queue_families = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };

            let mut queues = DeviceQueues {
                graphics_family: usize::MAX,
                compute_family: usize::MAX,
                transfer_family: usize::MAX
            };
            available_queue_families.iter().enumerate().for_each(|(index, properties)| {
                match  (properties.queue_flags.contains(vk::QueueFlags::GRAPHICS), 
                        properties.queue_flags.contains(vk::QueueFlags::COMPUTE),
                        properties.queue_flags.contains(vk::QueueFlags::TRANSFER)) {
                    (true, true, true) => queues.graphics_family = index,
                    (false, true, true) => queues.compute_family = index,
                    (false, false, true) => queues.transfer_family = index,
                    _ => warn!("Encountered unsupported device queue family!")
                }
            });
            // Handle missing queue families
            match (queues.graphics_family, queues.compute_family, queues.transfer_family) {
                (usize::MAX, _, _) => return None,
                (_, usize::MAX, _) => return None,
                (_, _, usize::MAX) => return None,
                _ => ()
            };

            // Attempt logical device creation
            let queue_create_infos: Vec<vk::DeviceQueueCreateInfo> = requested_queues.iter().filter_map(|(queue_family, priorities)| {
                match priorities.len() > 0 {
                    true => {
                        let index = match queue_family {
                            QueueFamily::GRAPHICS => queues.graphics_family,
                            QueueFamily::COMPUTE => queues.compute_family,
                            QueueFamily::TRANSFER => queues.transfer_family
                        };
                        Some(vk::DeviceQueueCreateInfo::builder()
                            .queue_family_index(index as u32)
                            .queue_priorities(priorities)
                            .build())
                    },
                    false => None
                }
            })
            .collect();

            let create_info = vk::DeviceCreateInfo::builder()
                .queue_create_infos(queue_create_infos.as_slice())
                .enabled_extension_names(requested_extensions)
                .enabled_features(&requested_features.features);
            //  Safety: vkCreateDevice
            //  In order for the created [`Device`] to be valid for the duration of its usage, 
            //  the [`Instance`] this was called on must be dropped later than the resulting [`Device`].
            //  
            //  Guaranteed by retaining Arc<Instance> inside Device object
            let logical_device = match unsafe { instance.create_device(physical_device, &create_info, None) } {
                Ok(logical_device) => logical_device,
                Err(_) => return None
            };

            Some((physical_device, logical_device, queues))
        });
        let (physical_device, logical_device, queues) = match result {
            Some(result) => result,
            None => return Err("No suitable device found for requested parameters!".to_string())
        };
        
        Ok(Self {
            internal: Arc::new(DeviceInternal {
                instance,
                physical_device,
                logical_device,
                queues
            })
        })
    }

    pub fn primary(instance: Instance) -> Result<Self, String> {
        Self::new(
            instance, 
            |properties| {
                // Select a discrete GPU with Vulkan 1.3 support
                let base_properties = properties.properties;
                let _major_version = vk::api_version_major(base_properties.api_version);
                let _minor_version = vk::api_version_minor(base_properties.api_version);
                let _patch_version = vk::api_version_patch(base_properties.api_version);
                
                let properties_check = 
                    _major_version == 1 && _minor_version >= 3 &&
                    base_properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU;

                properties_check
            },
            &[
                // Enable swapchain extension
                ash::extensions::khr::Swapchain::name().as_ptr()
                //ash::extensions::khr::AccelerationStructure::name().as_ptr()
            ],
            &vk::PhysicalDeviceFeatures2::builder(),
            [
                (QueueFamily::GRAPHICS, &[1.0]),
                (QueueFamily::COMPUTE, &[1.0]),
                (QueueFamily::TRANSFER, &[1.0]),
            ]
        )
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
