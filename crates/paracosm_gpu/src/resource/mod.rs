pub mod buffer;
pub mod image;

use crate::device::LogicalDevice;

use buffer::Buffer;
use image::{
    ImageInfo,
    ImageView
};

use anyhow::{Context as _, Result};
use bevy_ecs::prelude::Resource;

use std::{any::TypeId, collections::HashMap, mem::ManuallyDrop};


pub const BUFFER_BINDING: u32 = 0;
pub const STORAGE_IMAGE_BINDING: u32 = 1;
pub const SAMPLED_IMAGE_BINDING: u32 = 2;
pub const SAMPLER_BINDING: u32 = 3;
pub const ACCELERATION_STRUCTURE_BINDING: u32 = 4;

pub trait ResourceLabel: Send + Sync { }

pub trait BufferLabel: ResourceLabel {  }
pub trait ImageLabel: ResourceLabel {  }
pub trait AccelStructLabel: ResourceLabel {  }

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    Auto,
    AutoUpload,
    AutoDownload,
    Stream,
    //Manual
}


struct ResourcePool<T> {
    resources: HashMap<TypeId, T>,
    next_descriptor: u32,
    free_descriptors: Vec<u32>
}

pub(crate) struct ResourceManager {
    device: *const LogicalDevice,
    allocator: vk_mem::Allocator,

    descriptor_pool: ash::vk::DescriptorPool,
    pub descriptor_set_layout: ash::vk::DescriptorSetLayout,
    pub descriptor_set: ash::vk::DescriptorSet,

    buffers: ResourcePool<Buffer>,
}

impl ResourceManager {
    pub fn new(
        device: &LogicalDevice,
        allocator: vk_mem::Allocator,
        storage_buffers: u32,
        storage_images: u32,
        sampled_images: u32,
        samplers: u32,
        acceleration_structures: u32
    ) -> Result<Self> {
        // Create descriptor pool
        let pool_sizes = [
            ash::vk::DescriptorPoolSize::default()
                .ty(ash::vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(storage_buffers),
            ash::vk::DescriptorPoolSize::default()
                .ty(ash::vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(storage_images),
            ash::vk::DescriptorPoolSize::default()
                .ty(ash::vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(sampled_images),
            ash::vk::DescriptorPoolSize::default()
                .ty(ash::vk::DescriptorType::SAMPLER)
                .descriptor_count(samplers),
            ash::vk::DescriptorPoolSize::default()
                .ty(ash::vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .descriptor_count(acceleration_structures),
        ];

        let descriptor_pool_create_info = ash::vk::DescriptorPoolCreateInfo::default()
            .flags(ash::vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND | ash::vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(1)
            .pool_sizes(&pool_sizes);

        let descriptor_pool = unsafe {
            device.create_descriptor_pool(&descriptor_pool_create_info, None)
                .context("DescriptorPool should be created.")?
        };

        // Create descriptor set layout
        let descriptor_set_layout_bindings = [
            ash::vk::DescriptorSetLayoutBinding::default()
                .binding(BUFFER_BINDING)
                .descriptor_type(ash::vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(storage_buffers)
                .stage_flags(ash::vk::ShaderStageFlags::ALL),
            ash::vk::DescriptorSetLayoutBinding::default()
                .binding(STORAGE_IMAGE_BINDING)
                .descriptor_type(ash::vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(storage_images)
                .stage_flags(ash::vk::ShaderStageFlags::ALL),
            ash::vk::DescriptorSetLayoutBinding::default()
                .binding(SAMPLED_IMAGE_BINDING)
                .descriptor_type(ash::vk::DescriptorType::SAMPLED_IMAGE)
                .descriptor_count(sampled_images)
                .stage_flags(ash::vk::ShaderStageFlags::ALL),
            ash::vk::DescriptorSetLayoutBinding::default()
                .binding(SAMPLER_BINDING)
                .descriptor_type(ash::vk::DescriptorType::SAMPLER)
                .descriptor_count(samplers)
                .stage_flags(ash::vk::ShaderStageFlags::ALL),
            ash::vk::DescriptorSetLayoutBinding::default()
                .binding(ACCELERATION_STRUCTURE_BINDING)
                .descriptor_type(ash::vk::DescriptorType::ACCELERATION_STRUCTURE_KHR)
                .descriptor_count(acceleration_structures)
                .stage_flags(ash::vk::ShaderStageFlags::ALL),
        ];

        let descriptor_binding_flags = [
            ash::vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING | ash::vk::DescriptorBindingFlags::PARTIALLY_BOUND | ash::vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            ash::vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING | ash::vk::DescriptorBindingFlags::PARTIALLY_BOUND | ash::vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            ash::vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING | ash::vk::DescriptorBindingFlags::PARTIALLY_BOUND | ash::vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            ash::vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING | ash::vk::DescriptorBindingFlags::PARTIALLY_BOUND | ash::vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
            ash::vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING | ash::vk::DescriptorBindingFlags::PARTIALLY_BOUND | ash::vk::DescriptorBindingFlags::UPDATE_AFTER_BIND,
        ];

        let mut descriptor_set_layout_binding_flags_create_info = ash::vk::DescriptorSetLayoutBindingFlagsCreateInfo::default()
            .binding_flags(&descriptor_binding_flags);

        let descriptor_set_layout_create_info = ash::vk::DescriptorSetLayoutCreateInfo::default()
            .push_next(&mut descriptor_set_layout_binding_flags_create_info)
            .flags(ash::vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .bindings(&descriptor_set_layout_bindings);

        let descriptor_set_layout = unsafe {
            device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
                .context("DescriptorSetLayout should be created.")?
        };

        // Allocate descriptor set
        let descriptor_set_allocate_info = ash::vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(std::slice::from_ref(&descriptor_set_layout));

        let descriptor_set = unsafe {
            device.allocate_descriptor_sets(&descriptor_set_allocate_info)
                .context("DescriptorSet should be allocated.")?[0]
        };

        // Create resource pools
        let buffers = ResourcePool::<Buffer> {
            resources: Default::default(),
            next_descriptor: 0,
            free_descriptors: vec![]
        };

        Ok(Self {
            device,
            allocator,
            descriptor_pool,
            descriptor_set_layout,
            descriptor_set,
            buffers
        })
    }
}

impl Drop for ResourceManager {
    fn drop(&mut self) {
        unsafe {
            let device = self.device.as_ref().unwrap();

            // TODO: Verify destruction safety requirements

            device.destroy_descriptor_pool(self.descriptor_pool, None);
            device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            // Cleanup remaining resources
            for (_, buffer) in self.buffers.resources.iter_mut() {
                match buffer {
                    Buffer::Persistent { buffer, allocation, descriptor_index, transfer_mode, size, debug_name } => {
                        println!("Persistent buffer {}", debug_name);
                        self.allocator.destroy_buffer(*buffer, allocation);
                    },
                    _ => ()
                } 
            }
        }
    }
}



impl crate::context::Context {
    pub fn set_persistent_image(&mut self, label: impl ImageLabel, image: &ImageView) -> Result<()> {
        todo!()
    }

    pub fn set_transient_image(&mut self, label: impl ImageLabel, image: ImageInfo) -> Result<()> {
        todo!()
    }
}