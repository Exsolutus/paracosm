pub mod buffer;
pub mod image;
pub mod pipeline;
pub mod sampler;
pub mod shader_module;

use anyhow::Context;
pub use gpu_allocator::MemoryLocation;



use crate::device::Device;

use anyhow::Result;
use ash::vk;

use std::{
    mem::size_of,
};


pub struct ResourceManager {
    device: Device,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    descriptor_set: vk::DescriptorSet,
    pub pipeline_layouts: Vec<vk::PipelineLayout>
}

impl ResourceManager {
    pub fn new(device: &Device) -> Result<ResourceManager> {
        let limits = device.limits();

        // Create bindless descriptor pool
        let pool_sizes = vec![
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_BUFFER,
                descriptor_count: limits.max_descriptor_set_storage_buffers
            },
        ];

        let descriptor_pool = unsafe { device.create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes) 
                .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND)
                .max_sets(limits.max_bound_descriptor_sets), 
            None
        ).context("Device should create a descriptor pool")? };
        
        // Create descriptor layouts
        let descriptor_bindings = vec![
            vk::DescriptorSetLayoutBinding::builder()
                .binding(0)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(limits.max_descriptor_set_storage_buffers)
                .stage_flags(vk::ShaderStageFlags::ALL)
                .build(),
        ];

        let descriptor_binding_flags = vec![
            vk::DescriptorBindingFlags::PARTIALLY_BOUND |
            vk::DescriptorBindingFlags::UPDATE_AFTER_BIND |
            vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
        ];

        let descriptor_set_layout = unsafe { device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&descriptor_bindings)
                .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
                .push_next(&mut vk::DescriptorSetLayoutBindingFlagsCreateInfo::builder()
                    .binding_flags(&descriptor_binding_flags)
                ), 
            None
        ).context("Device should create a descriptor set layout")? };

        let descriptor_set_layouts = vec![descriptor_set_layout];

        // Create descriptor set
        let descriptor_set = unsafe { device.allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&descriptor_set_layouts)
        ).context("Device should allocate descriptor sets from descriptor pool")?[0] };

        // Create pipeline layouts
        let push_constants = vec![
            vk::PushConstantRange::builder()
                .offset(0)
                .size((size_of::<u32>() * 68) as u32) // TODO: generalize push constant size(s)
                .stage_flags(vk::ShaderStageFlags::VERTEX)
                .build(),
        ];
        
        let pipeline_layout = unsafe { device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder()
                .push_constant_ranges(&push_constants)
                .set_layouts(&descriptor_set_layouts), 
            None
        ).context("Device should create a pipeline layout")? };
        let pipeline_layouts = vec![pipeline_layout];

        Ok(ResourceManager {
            device: device.clone(),
            descriptor_pool,
            descriptor_set_layout,
            descriptor_set,
            pipeline_layouts
        })
    }
}

impl Drop for ResourceManager {
    fn drop(&mut self) {
        unsafe {
            for i in 0..self.pipeline_layouts.len() {
                self.device.destroy_pipeline_layout(
                    self.pipeline_layouts.remove(i), 
                    None
                );
            }
            self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.device.destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}

// use crate::device::Device;

// use anyhow::Result;
// use ash::vk;

// use std::ops::Deref;

// // Reexport
// pub use vk::{
//     DescriptorSetLayoutCreateInfo as DescriptorLayoutCreateInfo,
//     DescriptorSetLayoutCreateFlags as DescriptorLayoutCreateFlags,
// };

// // TODO: create a trait for Vulkan handle wrappers?
// /// A [`DescriptorPool`] wrapping an opaque Vulkan handle
// pub struct DescriptorPool {
//     device: Device,
//     pool: vk::DescriptorPool
// }

// impl Deref for DescriptorPool {
//     type Target = vk::DescriptorPool;

//     fn deref(&self) -> &Self::Target {
//         &self.pool
//     }
// }

// impl Drop for DescriptorPool {
//     fn drop(&mut self) {
//         unsafe {
//             self.device.destroy_descriptor_pool(self.pool, None);
//         }
//     }
// }

// impl Device {
//     pub fn create_descriptor_pool(
//         &self, 
//         pool_sizes: Vec<vk::DescriptorPoolSize>,
//         flags: vk::DescriptorPoolCreateFlags,
//         max_sets: u32
//     ) -> Result<DescriptorPool> {
//         let create_info = vk::DescriptorPoolCreateInfo::builder()
//             .pool_sizes(&pool_sizes)
//             .flags(flags)
//             .max_sets(max_sets);

//         let pool = unsafe {
//             self.logical_device.create_descriptor_pool(&create_info, None)?
//         };

//         Ok(DescriptorPool {
//             device: self.clone(),
//             pool
//         })
//     }
// }



// /// A [`DescriptorLayout`] wrapping an opaque Vulkan handle
// pub struct DescriptorLayout {
//     device: Device,
//     layout: vk::DescriptorSetLayout
// }

// impl Deref for DescriptorLayout {
//     type Target = vk::DescriptorSetLayout;

//     fn deref(&self) -> &Self::Target {
//         &self.layout
//     }
// }

// impl Drop for DescriptorLayout {
//     fn drop(&mut self) {
//         unsafe {
//             self.device.destroy_descriptor_set_layout(self.layout, None);
//         }
//     }
// }

// impl Device {
//     pub fn create_descriptor_layout(
//         &self,
//         create_info: &DescriptorLayoutCreateInfo // TODO: maybe abstract this create info better
//     ) -> Result<DescriptorLayout> {
//         let layout = unsafe {
//             self.create_descriptor_set_layout(
//                 &create_info, 
//                 None
//             )?
//         };

//         Ok(DescriptorLayout {
//             device: self.clone(),
//             layout
//         })
//     }
// }