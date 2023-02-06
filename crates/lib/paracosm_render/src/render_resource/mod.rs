pub mod pipeline;
pub mod shader;

//----------------------------------------------//

use anyhow::{Result, Context};
use ash::vk;

use std::{
    collections::{
        HashMap,
        VecDeque
    },
    mem::size_of,
    sync::Mutex
};

use paracosm_gpu::{
    device::Device,
    resource::{
        buffer::*,
        image::*,
        sampler::*
    }
};
use rust_shaders_shared::{
    ShaderConstants,
    ResourceHandle,
    STORAGE_BUFFER_BINDING,
    STORAGE_IMAGE_BINDING,
    SAMPLED_IMAGE_BINDING,
    SAMPLER_BINDING
};




#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
enum ResourceType {
    #[default] StorageBuffer,
    StorageImage,
    SampledImage,
    Sampler
}

#[derive(Default)]
struct ResourcePool {
    resource_type: ResourceType,
    pub(self) next_index: Mutex<u32>,
    pub(self) recycled_handles: Mutex<VecDeque<ResourceHandle>>
}

impl ResourcePool{
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
        let mut next_index = self.next_index.lock().unwrap();  // Lock index
        let current_index = next_index.clone(); // Clone current index value
        *next_index += 1;   // Iterate index

        current_index
    }
}

pub struct ResourceManager {
    pub device: Device,
    descriptor_pool: vk::DescriptorPool,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pub(crate) descriptor_set: vk::DescriptorSet,
    pub pipeline_layouts: Vec<vk::PipelineLayout>,
    resource_pools: HashMap<ResourceType, ResourcePool>
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
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::STORAGE_IMAGE,
                descriptor_count: limits.max_descriptor_set_storage_images
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: limits.max_descriptor_set_sampled_images
            },
            vk::DescriptorPoolSize {
                ty: vk::DescriptorType::SAMPLER,
                descriptor_count: limits.max_descriptor_set_samplers
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
                .binding(STORAGE_BUFFER_BINDING)
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .descriptor_count(limits.max_descriptor_set_storage_buffers)
                .stage_flags(vk::ShaderStageFlags::ALL)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .binding(STORAGE_IMAGE_BINDING)
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .descriptor_count(limits.max_descriptor_set_storage_images)
                .stage_flags(vk::ShaderStageFlags::ALL)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .binding(SAMPLED_IMAGE_BINDING)
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .stage_flags(vk::ShaderStageFlags::ALL)
                .descriptor_count(limits.max_descriptor_set_sampled_images)
                .build(),
            vk::DescriptorSetLayoutBinding::builder()
                .binding(SAMPLER_BINDING)
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .stage_flags(vk::ShaderStageFlags::ALL)
                .descriptor_count(limits.max_descriptor_set_samplers)
                .build(),
        ];

        let descriptor_binding_flags = vec![
            vk::DescriptorBindingFlags::PARTIALLY_BOUND | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND | vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING,
            vk::DescriptorBindingFlags::PARTIALLY_BOUND | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND | vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING,
            vk::DescriptorBindingFlags::PARTIALLY_BOUND | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND | vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING,
            vk::DescriptorBindingFlags::PARTIALLY_BOUND | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND | vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING,
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
                .size((size_of::<ShaderConstants>()) as u32) // TODO: generalize push constant size(s)
                .stage_flags(vk::ShaderStageFlags::ALL)
                .build(),
        ];
        
        let pipeline_layout = unsafe { device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder()
                .push_constant_ranges(&push_constants)
                .set_layouts(&descriptor_set_layouts), 
            None
        ).context("Device should create a pipeline layout")? };
        let pipeline_layouts = vec![pipeline_layout];


        // Create resource pools
        let mut resource_pools = HashMap::new();
        resource_pools.insert(ResourceType::StorageBuffer, ResourcePool {
            resource_type: ResourceType::StorageBuffer,
            ..Default::default()
        });
        resource_pools.insert(ResourceType::StorageImage, ResourcePool {
            resource_type: ResourceType::StorageImage,
            ..Default::default()
        });
        resource_pools.insert(ResourceType::SampledImage, ResourcePool {
            resource_type: ResourceType::SampledImage,
            ..Default::default()
        });
        resource_pools.insert(ResourceType::Sampler, ResourcePool {
            resource_type: ResourceType::Sampler,
            ..Default::default()
        });

        Ok(ResourceManager {
            device: device.clone(),
            descriptor_pool,
            descriptor_set_layout,
            descriptor_set,
            pipeline_layouts,
            resource_pools,
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
        let handle_type = ResourceType::StorageBuffer; // TODO: Get handle type from handle itself. Until then recycle won't work properly.

        let resource_pool = self.resource_pools.get(&handle_type)
            .expect("ResourceHandle should have a valid ResourceType");

        resource_pool.recycled_handles
            .lock()
            .unwrap()
            .push_back(handle);
    }

    pub(crate) fn new_buffer_handle(&self, buffer: &Buffer) -> ResourceHandle {
        let resource_pool = self.resource_pools.get(&ResourceType::StorageBuffer)
            .expect("StorageBuffer resource pool should exist");
        let handle = resource_pool.fetch_handle();

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
                .dst_binding(STORAGE_BUFFER_BINDING)
                .dst_array_element(handle.index())
                .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
                .buffer_info(&buffer_info)
                .build(),
        ];

        unsafe { self.device.update_descriptor_sets(&write, &[]); }

        handle
    }

    pub(crate) fn new_storage_image_handle(&self, image: &Image) -> ResourceHandle {
        let resource_pool = self.resource_pools.get(&ResourceType::StorageImage)
            .expect("StorageBuffer resource pool should exist");
        let handle = resource_pool.fetch_handle();

        let image_info = [
            vk::DescriptorImageInfo::builder()
                .image_layout(ImageLayout::GENERAL)
                .image_view(image.image_view)
                .sampler(vk::Sampler::null())
                .build(),
        ];

        let write = [
            vk::WriteDescriptorSet::builder()
                .dst_set(self.descriptor_set)
                .dst_binding(STORAGE_IMAGE_BINDING)
                .dst_array_element(handle.index())
                .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
                .image_info(&image_info)
                .build()
        ];

        unsafe { self.device.update_descriptor_sets(&write, &[]); }

        handle
    }

    pub(crate) fn new_sampled_image_handle(&self, image: &Image) -> ResourceHandle {
        let resource_pool = self.resource_pools.get(&ResourceType::SampledImage)
            .expect("StorageBuffer resource pool should exist");
        let handle = resource_pool.fetch_handle();

        let image_info = [
            vk::DescriptorImageInfo::builder()
                .image_layout(ImageLayout::READ_ONLY_OPTIMAL)
                .image_view(image.image_view)
                .sampler(vk::Sampler::null())
                .build(),
        ];

        let write = [
            vk::WriteDescriptorSet::builder()
                .dst_set(self.descriptor_set)
                .dst_binding(SAMPLED_IMAGE_BINDING)
                .dst_array_element(handle.index())
                .descriptor_type(vk::DescriptorType::SAMPLED_IMAGE)
                .image_info(&image_info)
                .build()
        ];

        unsafe { self.device.update_descriptor_sets(&write, &[]); }

        handle
    }

    pub(crate) fn new_sampler_handle(&self, sampler: &Sampler) -> ResourceHandle {
        let resource_pool = self.resource_pools.get(&ResourceType::Sampler)
            .expect("StorageBuffer resource pool should exist");
        let handle = resource_pool.fetch_handle();

        let sampler_info = [
            vk::DescriptorImageInfo::builder()
                .image_layout(ImageLayout::UNDEFINED)
                .image_view(vk::ImageView::null())
                .sampler(**sampler)
                .build(),
        ];

        let write = [
            vk::WriteDescriptorSet::builder()
                .dst_set(self.descriptor_set)
                .dst_binding(SAMPLER_BINDING)
                .dst_array_element(handle.index())
                .descriptor_type(vk::DescriptorType::SAMPLER)
                .image_info(&sampler_info)
                .build()
        ];

        unsafe { self.device.update_descriptor_sets(&write, &[]); }

        handle
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
