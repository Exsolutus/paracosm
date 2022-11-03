use crate::Device;

use gpu_allocator;

use std::{ops::Deref};


pub struct Allocator {
    device: Device,
    allocator: gpu_allocator::vulkan::Allocator
}

impl Allocator {
    pub fn new(device: Device) -> Self {
        let allocator = gpu_allocator::vulkan::Allocator::new(
            &gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: device.instance.deref().deref().clone(),
                device: device.deref().deref().clone(),
                physical_device: device.physical_device,
                debug_settings: Default::default(),
                buffer_device_address: true
            }
        ).unwrap();

        Self {
            device,
            allocator
        }
    }
}