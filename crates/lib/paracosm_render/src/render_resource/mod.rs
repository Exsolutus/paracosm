pub mod pipeline;
pub mod shader;

//----------------------------------------------//

use anyhow::{Result, Context};
use ash::vk;

use std::{
    collections::VecDeque,
    mem::size_of,
    sync::Mutex,
};

use paracosm_gpu::{
    device::Device,
    resource::buffer::Buffer
};
use rust_shaders_shared::ResourceHandle;


pub struct ResourceManager {
    pub device: Device,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pub(crate) descriptor_set: vk::DescriptorSet,
    pub pipeline_layouts: Vec<vk::PipelineLayout>,
    // TODO: track resource handles per resource type
    next_index: Mutex<u32>,
    recycled_handles: Mutex<VecDeque<ResourceHandle>>
}

impl ResourceManager {
    pub(crate) fn new(device: &Device) -> Result<ResourceManager> {
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
                .binding(0)  // TODO: bindings as constants
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(limits.max_descriptor_set_storage_buffers)
                .stage_flags(vk::ShaderStageFlags::ALL)
                .build(),
        ];

        let descriptor_binding_flags = vec![
            vk::DescriptorBindingFlags::PARTIALLY_BOUND |
            vk::DescriptorBindingFlags::UPDATE_AFTER_BIND //|
            //vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
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
                .size((size_of::<u32>() * 20) as u32) // TODO: generalize push constant size(s)
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
            pipeline_layouts,

            next_index: Mutex::new(0),
            recycled_handles: Mutex::new(VecDeque::new())
        })
    }

    pub fn bind(&self, command_buffer: vk::CommandBuffer) {
        unsafe {
            // Bind global descriptor set
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layouts[0],
                0,
                &[self.descriptor_set],
                &[]
            );
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layouts[0],
                0,
                &[self.descriptor_set],
                &[]
            );
            self.device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::RAY_TRACING_KHR,
                self.pipeline_layouts[0],
                0,
                &[self.descriptor_set],
                &[]
            );
        }
    }

    pub(crate) fn recycle_handle(&self, handle: ResourceHandle) {
        self.recycled_handles
            .lock()
            .unwrap()
            .push_back(handle);
    }

    // TODO: track resource handles per resource type
    pub(crate) fn new_buffer_handle(&self, buffer: &Buffer) -> ResourceHandle {
        let handle = self.fetch_handle();

        let buffer_info = [
            vk::DescriptorBufferInfo::builder()
                .buffer(buffer.buffer)
                .offset(0)
                .range(vk::WHOLE_SIZE)
                .build(),
        ];

        let write = [
            vk::WriteDescriptorSet::builder()
                .dst_set(self.descriptor_set)
                .dst_binding(0) // TODO: bindings as constants
                .dst_array_element(handle.index())
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&buffer_info)
                .build(),
        ];

        unsafe { self.device.update_descriptor_sets(&write, &[]); }

        handle
    }

    fn fetch_handle(&self) -> ResourceHandle {
        self.recycled_handles
            .lock()
            .unwrap()
            .pop_front()
            .map_or_else(
                || ResourceHandle::new(self.increment_index()), 
                |recycled_handle| recycled_handle
            )
    }

    fn increment_index(&self) -> u32 {
        let index = self.next_index.lock().unwrap().clone(); // Lock, then clone current index and drop lock
        *self.next_index.lock().unwrap() += 1;  // Lock, then iterate current index
        index
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
