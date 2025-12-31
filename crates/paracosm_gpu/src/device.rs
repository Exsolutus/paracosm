use crate::{
    context::Context, 
    pipeline::PipelineManager, 
    queue::{Queue, QueueGraph}, 
    resource::{ACCELERATION_STRUCTURE_BINDING, BUFFER_BINDING, SAMPLED_IMAGE_BINDING, SAMPLER_BINDING, STORAGE_IMAGE_BINDING, buffer::Buffer, image::Image, surface::Surface}, 
    validation::DebugUtilsDevice
};

use anyhow::{Context as _, Result};
use bevy_ecs::world::World;

use std::mem::ManuallyDrop;

// Reexport
pub use ash::vk::PhysicalDeviceType as DeviceType;



#[derive(Clone, Copy, Default)]
pub struct QueueProperties {
    pub compute_family: u32,
    pub graphics_family: u32,
    pub transfer_family: u32,

    pub compute_count: u32,
    pub transfer_count: u32
}

#[derive(Clone, Copy)]
pub struct DeviceProperties {
    properties: ash::vk::PhysicalDeviceProperties,
    pub queue: QueueProperties,
    pub acceleration_structure: ash::vk::PhysicalDeviceAccelerationStructurePropertiesKHR<'static>,
}

impl std::ops::Deref for DeviceProperties {
    type Target = ash::vk::PhysicalDeviceProperties;

    fn deref(&self) -> &Self::Target {
        &self.properties
    }
}


#[derive(Clone, Copy)]
pub(crate) struct PhysicalDevice {
    inner: ash::vk::PhysicalDevice,
    pub properties: DeviceProperties,
}

impl std::ops::Deref for PhysicalDevice {
    type Target = ash::vk::PhysicalDevice;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl PhysicalDevice {
    pub fn new(instance: &ash::Instance, physical_device: ash::vk::PhysicalDevice) -> Result<Self> {
        let mut acceleration_structure_properties = ash::vk::PhysicalDeviceAccelerationStructurePropertiesKHR::default();
        let mut device_properties2 = ash::vk::PhysicalDeviceProperties2::default()
            .push_next(&mut acceleration_structure_properties);
        unsafe { instance.get_physical_device_properties2(physical_device, &mut device_properties2) };

        let mut queue_properties = QueueProperties::default();
        let queue_family_properties = unsafe { instance.get_physical_device_queue_family_properties(physical_device) };
        for (family, properties) in queue_family_properties.iter().enumerate()  {
            match (
                properties.queue_flags.contains(ash::vk::QueueFlags::GRAPHICS),
                properties.queue_flags.contains(ash::vk::QueueFlags::COMPUTE),
                properties.queue_flags.contains(ash::vk::QueueFlags::TRANSFER),
            ) {
                // Graphics queue family
                (true, true, true) => queue_properties.graphics_family = family as u32,
                // Compute queue family
                (false, true, true) => if queue_properties.compute_count == 0 {
                    queue_properties.compute_family = family as u32;
                    queue_properties.compute_count = properties.queue_count;
                },
                // Transfer queue family
                (false, false, true) => if queue_properties.transfer_count == 0 {
                    queue_properties.transfer_family = family as u32;
                    queue_properties.transfer_count = properties.queue_count;
                },
                _ => ()
            }
        };

        Ok(Self {
            inner: physical_device,
            properties: DeviceProperties {
                properties: device_properties2.properties,
                queue: queue_properties,
                acceleration_structure: acceleration_structure_properties
            }
        })
    }
}


pub(crate) struct LogicalDevice {
    device: ash::Device,
    pub mesh: ash::ext::mesh_shader::Device,
    pub timeline: ash::khr::timeline_semaphore::Device,
    pub swapchain: ash::khr::swapchain::Device,
    #[cfg(debug_assertions)] pub debug_utils: DebugUtilsDevice,
}

impl std::ops::Deref for LogicalDevice {
    type Target = ash::Device;

    fn deref(&self) -> &Self::Target {
        &self.device    
    }
}

#[derive(Default)]
pub(crate) struct DescriptorCache {
    pub next_descriptor: u32,
    pub free_descriptors: Vec<u32>
}

pub(crate) struct Device {
    instance: ash::Instance,
    pub physical_device: PhysicalDevice,
    pub logical_device: Box<LogicalDevice>,
    pub world: World,

    // Frame graph 
    pub graphics_graph: ManuallyDrop<QueueGraph>,
    pub compute_graph: ManuallyDrop<QueueGraph>,

    // Resources
    pub allocator: ManuallyDrop<vk_mem::Allocator>,
    descriptor_pool: ash::vk::DescriptorPool,
    descriptor_set_layout: ash::vk::DescriptorSetLayout,
    pub descriptor_set: ash::vk::DescriptorSet,
    pub buffer_descriptors: DescriptorCache,
    pub image_view_descriptors: DescriptorCache,
}

impl std::ops::Deref for Device {
    type Target = LogicalDevice;

    fn deref(&self) -> &Self::Target {
        &self.logical_device
    }
}

impl Device {
    pub fn new(
        instance: ash::Instance,
        physical_device: PhysicalDevice,
        display_support: bool
    ) -> Result<Self> {
        let queue_properties = physical_device.properties.queue;
        
        // Gather required device features and extensions
        let physical_device_features = ash::vk::PhysicalDeviceFeatures::default()
            .image_cube_array(true)
            .multi_draw_indirect(true)
            .fill_mode_non_solid(true)
            .wide_lines(true)
            .sampler_anisotropy(true)
            .fragment_stores_and_atomics(true)
            .shader_storage_image_multisample(true)
            .shader_storage_image_read_without_format(true)
            .shader_storage_image_write_without_format(true);

        let mut physical_device_descriptor_indexing_features = ash::vk::PhysicalDeviceDescriptorIndexingFeatures::default()
            .shader_sampled_image_array_non_uniform_indexing(true)
            .shader_storage_image_array_non_uniform_indexing(true)
            .shader_storage_buffer_array_non_uniform_indexing(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_update_unused_while_pending(true)
            .descriptor_binding_partially_bound(true)
            .runtime_descriptor_array(true);
        
        let mut physical_device_host_query_reset_features = ash::vk::PhysicalDeviceHostQueryResetFeatures::default()
            .host_query_reset(true);

        let mut physical_device_shader_atomic_int64_features = ash::vk::PhysicalDeviceShaderAtomicInt64Features::default()
            .shader_buffer_int64_atomics(true)
            .shader_shared_int64_atomics(true);

        let mut physical_device_shader_image_atomic_int64_features = ash::vk::PhysicalDeviceShaderImageAtomicInt64FeaturesEXT::default()
            .shader_image_int64_atomics(true);

        let mut physical_device_dynamic_rendering_features = ash::vk::PhysicalDeviceDynamicRenderingFeatures::default()
            .dynamic_rendering(true);

        let mut physical_device_timeline_semaphore_features = ash::vk::PhysicalDeviceTimelineSemaphoreFeatures::default()
            .timeline_semaphore(true);

        let mut physical_device_synchronization_2_features = ash::vk::PhysicalDeviceSynchronization2Features::default()
            .synchronization2(true);

        let mut physical_device_robustness_2_features = ash::vk::PhysicalDeviceRobustness2FeaturesEXT::default()
            .null_descriptor(true);

        let mut physical_device_scalar_layout_features = ash::vk::PhysicalDeviceScalarBlockLayoutFeatures::default()
            .scalar_block_layout(true);

        let mut physical_device_vulkan_memory_model_features = ash::vk::PhysicalDeviceVulkanMemoryModelFeatures::default()
            .vulkan_memory_model(true);

        let mut physical_device_acceleration_structure_features = ash::vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default()
            .acceleration_structure(true)
            .descriptor_binding_acceleration_structure_update_after_bind(true);

        let mut physical_device_ray_tracing_pipeline_features = ash::vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default()
            .ray_tracing_pipeline(true)
            .ray_tracing_pipeline_trace_rays_indirect(true)
            .ray_traversal_primitive_culling(true);

        let mut physical_device_maintenance5_features = ash::vk::PhysicalDeviceMaintenance5FeaturesKHR::default()
            .maintenance5(true);

        let mut physical_device_mesh_shader_features = ash::vk::PhysicalDeviceMeshShaderFeaturesEXT::default()
            .task_shader(true)
            .mesh_shader(true);

        let mut physical_device_features_2 = ash::vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut physical_device_descriptor_indexing_features)
            .push_next(&mut physical_device_host_query_reset_features)
            .push_next(&mut physical_device_shader_atomic_int64_features)
            .push_next(&mut physical_device_shader_image_atomic_int64_features)
            .push_next(&mut physical_device_dynamic_rendering_features)
            .push_next(&mut physical_device_timeline_semaphore_features)
            .push_next(&mut physical_device_synchronization_2_features)
            .push_next(&mut physical_device_robustness_2_features)
            .push_next(&mut physical_device_scalar_layout_features)
            .push_next(&mut physical_device_vulkan_memory_model_features)
            .push_next(&mut physical_device_acceleration_structure_features)
            .push_next(&mut physical_device_ray_tracing_pipeline_features)
            .push_next(&mut physical_device_maintenance5_features)
            .push_next(&mut physical_device_mesh_shader_features)
            .features(physical_device_features);

        let mut extension_names = vec![
            ash::ext::descriptor_indexing::NAME.as_ptr(),
            ash::ext::shader_image_atomic_int64::NAME.as_ptr(),
            ash::ext::robustness2::NAME.as_ptr(),
            ash::khr::push_descriptor::NAME.as_ptr(),
            ash::khr::deferred_host_operations::NAME.as_ptr(),
            ash::khr::acceleration_structure::NAME.as_ptr(),
            ash::khr::ray_tracing_pipeline::NAME.as_ptr(),
            ash::ext::mesh_shader::NAME.as_ptr(),
            ash::khr::maintenance5::NAME.as_ptr()
        ];
        if display_support {
            extension_names.push(ash::khr::swapchain::NAME.as_ptr());
        }

        // Gather queue info
        let compute_priorities = vec![0.0; queue_properties.compute_count as usize];
        let transfer_priorities = vec![0.0; queue_properties.transfer_count as usize];
        let queue_create_infos = [
            ash::vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_properties.graphics_family)
                .queue_priorities(&[0.0]),
            ash::vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_properties.compute_family)
                .queue_priorities(&compute_priorities),
            ash::vk::DeviceQueueCreateInfo::default()
                .queue_family_index(queue_properties.transfer_family)
                .queue_priorities(&transfer_priorities),
        ];

        // Create logical device
        let device_create_info = ash::vk::DeviceCreateInfo::default()
            .push_next(&mut physical_device_features_2)
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&extension_names);
        let device = unsafe { instance.create_device(*physical_device, &device_create_info, None)? };

        let mesh = ash::ext::mesh_shader::Device::new(&instance, &device);
        let timeline = ash::khr::timeline_semaphore::Device::new(&instance, &device);
        let swapchain = ash::khr::swapchain::Device::new(&instance, &device);
        #[cfg(debug_assertions)] let debug_utils = DebugUtilsDevice::new(&instance, &device);

        let logical_device = Box::new(LogicalDevice {
            device,
            mesh,
            timeline,
            swapchain,
            #[cfg(debug_assertions)] debug_utils
        });

        // Create allocator
        let allocator_create_info = vk_mem::AllocatorCreateInfo::new(
            &instance, 
            &logical_device, 
            *physical_device
        );
        let allocator = unsafe { vk_mem::Allocator::new(allocator_create_info)? };

        let storage_buffers = physical_device.properties.limits.max_descriptor_set_storage_buffers.min(1000);
        let storage_images = physical_device.properties.limits.max_descriptor_set_storage_images.min(1000);
        let sampled_images = physical_device.properties.limits.max_descriptor_set_sampled_images.min(1000);
        let samplers = physical_device.properties.limits.max_descriptor_set_samplers.min(1000);
        let acceleration_structures = physical_device.properties.acceleration_structure.max_descriptor_set_acceleration_structures.min(1000);

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
            logical_device.create_descriptor_pool(&descriptor_pool_create_info, None)
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

        let descriptor_binding_flags = [ash::vk::DescriptorBindingFlags::UPDATE_UNUSED_WHILE_PENDING | ash::vk::DescriptorBindingFlags::PARTIALLY_BOUND | ash::vk::DescriptorBindingFlags::UPDATE_AFTER_BIND; 5];

        let mut descriptor_set_layout_binding_flags_create_info = ash::vk::DescriptorSetLayoutBindingFlagsCreateInfo::default()
            .binding_flags(&descriptor_binding_flags);

        let descriptor_set_layout_create_info = ash::vk::DescriptorSetLayoutCreateInfo::default()
            .push_next(&mut descriptor_set_layout_binding_flags_create_info)
            .flags(ash::vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL)
            .bindings(&descriptor_set_layout_bindings);

        let descriptor_set_layout = unsafe {
            logical_device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None)
                .context("DescriptorSetLayout should be created.")?
        };

        // Allocate descriptor set
        let descriptor_set_allocate_info = ash::vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(descriptor_pool)
            .set_layouts(std::slice::from_ref(&descriptor_set_layout));

        let descriptor_set = unsafe {
            logical_device.allocate_descriptor_sets(&descriptor_set_allocate_info)
                .context("DescriptorSet should be allocated.")?[0]
        };
        
        // Create PipelineManager
        let pipeline_manager = PipelineManager::new(
            &logical_device,
            physical_device.properties.limits.max_push_constants_size,
            &descriptor_set_layout
        )?;

        // Create queue graphs
        let compute_graph = QueueGraph::new(
            &logical_device, 
            Queue::Compute, 
            physical_device.properties.queue.compute_family, 
            pipeline_manager.pipeline_layout,
            descriptor_set
        )?;
        let graphics_graph = QueueGraph::new(
            &logical_device, 
            Queue::Graphics, 
            physical_device.properties.queue.graphics_family, 
            pipeline_manager.pipeline_layout,
            descriptor_set
        )?;

        let mut world = World::new();
        world.insert_resource(pipeline_manager);

        Ok(Self {
            instance,
            physical_device,
            logical_device,
            world,
            graphics_graph: ManuallyDrop::new(graphics_graph),
            compute_graph: ManuallyDrop::new(compute_graph),
            allocator: ManuallyDrop::new(allocator),
            descriptor_pool,
            descriptor_set_layout,
            descriptor_set,
            buffer_descriptors: Default::default(),
            image_view_descriptors: Default::default()
        })
    }

    pub fn properties(&self) -> &DeviceProperties {
        &self.physical_device.properties
    }

    pub fn execute(&mut self) -> Result<()> {
        // TODO: queue graph validations

        self.compute_graph.run(&mut self.world)?;
        self.graphics_graph.run(&mut self.world)?;

        Ok(())
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.logical_device.device_wait_idle().unwrap_unchecked();

            // Clean up buffers
            let mut buffers = self.world.query::<&mut Buffer>();
            let mut buffer_count = 0;
            for mut buffer in buffers.iter_mut(&mut self.world) {
                self.allocator.destroy_buffer(buffer.inner, &mut buffer.allocation);
                buffer_count += 1;
            }
            println!("Cleaned up {} buffers.", buffer_count);

            // Clean up images
            let mut images = self.world.query::<&mut Image>();
            let mut image_count = 0;
            for mut image in images.iter_mut(&mut self.world) {
                for image_view in &image.image_views {
                    self.logical_device.destroy_image_view(image_view.inner, None);
                }

                self.allocator.destroy_image(image.image, &mut image.allocation);
                image_count += 1;
            }
            println!("Cleaned up {} images.", image_count);

            // Clean up surfaces
            let mut surfaces = self.world.query::<&mut Surface>();
            for surface in surfaces.iter(&self.world) {
                for semaphore in &surface.acquire_semaphores {
                    self.logical_device.destroy_semaphore(*semaphore, None);
                }
                for semaphore in &surface.submit_semaphores {
                    self.destroy_semaphore(*semaphore, None);
                }
                
                self.swapchain.destroy_swapchain(surface.swapchain, None);
                surface.api.destroy_surface(surface.surface, None);
            }

            self.world.clear_all();
            
            ManuallyDrop::drop(&mut self.graphics_graph);
            ManuallyDrop::drop(&mut self.compute_graph);

            ManuallyDrop::drop(&mut self.allocator);
            self.logical_device.destroy_descriptor_pool(self.descriptor_pool, None);
            self.logical_device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);

            //  Safety: vkDestroyDevice
            //  Host Synchronization
            //   -  Host access to device must be externally synchronized
            //   -  Host access to all VkQueue objects created from device must be externally synchronized
            //
            //  Synchronized host access to instance guaranteed by borrow checker with '&mut self'
            self.logical_device.destroy_device(None);
        }
    }
}

impl Context {
    pub fn wait_idle(&self) {
        unsafe { self.active_device.logical_device.device_wait_idle().unwrap() }
    }
}
