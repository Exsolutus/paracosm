use crate::{node::resource::ResourceIndex, resource::{SAMPLED_IMAGE_BINDING, STORAGE_IMAGE_BINDING}};

use super::{ResourceLabel, ResourceManager, TransferMode};

use anyhow::{bail, Ok, Result};
use vk_mem::{Alloc, AllocationCreateFlags, AllocationCreateInfo, MemoryUsage};

use std::{any::{type_name, type_name_of_val, Any}, marker::PhantomData};

// Reexport
pub use ash::vk::{
    Format,
    SampleCountFlags
};


pub trait ImageLabel: ResourceLabel {  }

#[derive(Default)]
pub struct ImageInfo {
    pub format: ash::vk::Format,
    /// Image extent with format \[width, height, depth\]
    pub extent: [u32; 3],
    pub mip_levels: u32,
    pub array_layers: u32,
    pub samples: ash::vk::SampleCountFlags,
    pub shared: bool,
    pub transfer_mode: TransferMode
}

pub(crate) enum Image {
    Persistent {
        info: ImageInfo,

        image: ash::vk::Image,
        image_views: Box<[ash::vk::ImageView]>,
        allocation: vk_mem::Allocation,
        descriptor_index: u32,

        #[cfg(debug_assertions)] debug_name: &'static str
    },
    Transient {
        info: ImageInfo,

        #[cfg(debug_assertions)] debug_name: &'static str
    }
}

pub struct SamplerInfo {

}

pub(crate) struct Sampler {

}


impl crate::context::Context {
    pub fn create_image<L: ImageLabel + 'static>(
        &mut self,
        label: L,
        info: ImageInfo
    ) -> Result<()> {
        let device = &mut self.devices[self.configuring_device as usize];
        let mut resource_manager = device.graph_world.resource_mut::<ResourceManager>();

        let debug_name = type_name_of_val(&label);

        // Create new image
        let (image_type, image_extent) = match info.extent {
            [0, _, _] => bail!("Image width must be greater than 0."),
            [width, 0, 0] => (ash::vk::ImageType::TYPE_1D, ash::vk::Extent3D { width, height: 1, depth: 1 }),
            [width, height, 0] => (ash::vk::ImageType::TYPE_2D, ash::vk::Extent3D { width, height, depth: 1 }),
            [width, height, depth] => (ash::vk::ImageType::TYPE_3D, ash::vk::Extent3D { width, height, depth }),
        };

        let image_create_info = ash::vk::ImageCreateInfo::default()
            .flags(match image_type {
                ash::vk::ImageType::TYPE_3D => ash::vk::ImageCreateFlags::CUBE_COMPATIBLE,
                _ => ash::vk::ImageCreateFlags::empty()
            })
            .usage(
                ash::vk::ImageUsageFlags::STORAGE | 
                ash::vk::ImageUsageFlags::SAMPLED | 
                ash::vk::ImageUsageFlags::TRANSFER_SRC | 
                ash::vk::ImageUsageFlags::TRANSFER_DST
            )
            .image_type(image_type)
            .format(info.format)
            .extent(image_extent)
            .mip_levels(info.mip_levels)
            .array_layers(info.array_layers)
            .samples(info.samples)
            .tiling(ash::vk::ImageTiling::OPTIMAL);
        let allocation_create_info = match info.transfer_mode {
            TransferMode::Auto |
            TransferMode::AutoUpload |
            TransferMode::AutoDownload => AllocationCreateInfo {
                usage: MemoryUsage::AutoPreferDevice,
                required_flags: ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
                ..Default::default()
            },
            TransferMode::Stream => AllocationCreateInfo {
                usage: MemoryUsage::AutoPreferDevice,
                required_flags: ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
                flags: AllocationCreateFlags::MAPPED
                     | AllocationCreateFlags::HOST_ACCESS_RANDOM,
                ..Default::default()
            }
        };
        let (image, allocation) = unsafe { resource_manager.allocator.create_image(&image_create_info, &allocation_create_info)? };

        // Transition image to ImageLayout::GENERAL
        unsafe {
            device.logical_device.begin_command_buffer(
                device.graphics_graph.immediate_command_buffer, 
                &ash::vk::CommandBufferBeginInfo::default()
                    .flags(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT)
            )?;

            let barrier = ash::vk::ImageMemoryBarrier2::default()
                .src_stage_mask(ash::vk::PipelineStageFlags2::ALL_COMMANDS)
                .src_access_mask(ash::vk::AccessFlags2::MEMORY_WRITE)
                .dst_stage_mask(ash::vk::PipelineStageFlags2::ALL_COMMANDS)
                .dst_access_mask(ash::vk::AccessFlags2::MEMORY_WRITE | ash::vk::AccessFlags2::MEMORY_READ)
                .old_layout(ash::vk::ImageLayout::UNDEFINED)
                .new_layout(ash::vk::ImageLayout::GENERAL)
                .image(image)
                .subresource_range(
                    ash::vk::ImageSubresourceRange::default()
                        .aspect_mask(ash::vk::ImageAspectFlags::COLOR) // TODO: Support for depth/stencil formats
                        .base_array_layer(0)
                        .layer_count(ash::vk::REMAINING_ARRAY_LAYERS)
                        .base_mip_level(0)
                        .level_count(ash::vk::REMAINING_MIP_LEVELS)
                );
            device.logical_device.cmd_pipeline_barrier2(
                device.graphics_graph.immediate_command_buffer, 
                &ash::vk::DependencyInfo::default()
                    .image_memory_barriers(std::slice::from_ref(&barrier))
            );

            device.logical_device.end_command_buffer(device.graphics_graph.immediate_command_buffer)?;

            let command_buffer_info = ash::vk::CommandBufferSubmitInfo::default()
                .command_buffer(device.graphics_graph.immediate_command_buffer);
            let submit_info = ash::vk::SubmitInfo2::default()
                .command_buffer_infos(std::slice::from_ref(&command_buffer_info));
            device.logical_device.queue_submit2(device.graphics_graph.queue, std::slice::from_ref(&submit_info), ash::vk::Fence::null())?;
        }

        // Create image views
        let image_view_info = ash::vk::ImageViewCreateInfo::default()
            .view_type(match (info.extent, info.array_layers) {
                ([_width, 0, 0], 0) => ash::vk::ImageViewType::TYPE_1D,
                ([_width, 0, 0], _layers) => ash::vk::ImageViewType::TYPE_1D_ARRAY,
                ([_width, _height, 0], 0) => ash::vk::ImageViewType::TYPE_2D,
                ([_width, _height, 0], layers) => {
                    // TODO: Support for ImageViewType::Cube and ImageViewType::CubeArray
                    ash::vk::ImageViewType::TYPE_2D_ARRAY
                },
                ([_width, _height, _depth], 0) => ash::vk::ImageViewType::TYPE_3D,
                ([_width, _height, _depth], _layers) => bail!("3D images cannot have array layers."),
            })
            .format(info.format)
            .image(image)
            .subresource_range(
                ash::vk::ImageSubresourceRange::default()
                    .aspect_mask(ash::vk::ImageAspectFlags::COLOR) // TODO: Support for depth/stencil formats
                    .level_count(info.mip_levels)
                    .layer_count(info.array_layers)
            );
        let image_view = unsafe { device.logical_device.create_image_view(&image_view_info, None)? };
        
        let image_views = Box::new([image_view]);

        let descriptor_index = match resource_manager.images.free_descriptors.pop() {
            Some(index) => index,
            None => {
                let index = resource_manager.images.next_descriptor;
                resource_manager.images.next_descriptor += 1;
                index
            }
        };

        resource_manager.images.resources.insert(
            label.type_id(),
            Image::Persistent {
                info,
                image,
                image_views,
                allocation,
                descriptor_index,
                #[cfg(debug_assertions)] debug_name
            }
        );

        let image_info = [
            ash::vk::DescriptorImageInfo::default()
                .image_view(image_view)
                .image_layout(ash::vk::ImageLayout::GENERAL)
        ];
        unsafe { device.logical_device.update_descriptor_sets(
            &[
                ash::vk::WriteDescriptorSet::default()
                    .dst_set(resource_manager.descriptor_set)
                    .dst_binding(STORAGE_IMAGE_BINDING)
                    .dst_array_element(descriptor_index)
                    .descriptor_count(1)
                    .descriptor_type(ash::vk::DescriptorType::STORAGE_IMAGE)
                    .image_info(&image_info),
                ash::vk::WriteDescriptorSet::default()
                    .dst_set(resource_manager.descriptor_set)
                    .dst_binding(SAMPLED_IMAGE_BINDING)
                    .dst_array_element(descriptor_index)
                    .descriptor_count(1)
                    .descriptor_type(ash::vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image_info),
            ], 
            &[]
        ); }

        device.graph_world.insert_resource::<ResourceIndex<L>>(ResourceIndex::<L> { index: descriptor_index, _marker: PhantomData::<L>::default() });

        #[cfg(debug_assertions)]
        unsafe {
            let image_name = std::ffi::CString::new(format!("Storage Image: {}", debug_name))?;
            let image_name_info = ash::vk::DebugUtilsObjectNameInfoEXT::default()
                .object_handle(image)
                .object_name(&image_name);
            device.logical_device.debug_utils.set_debug_utils_object_name(&image_name_info)?;
        }

        Ok(())
    }

    pub fn destroy_image(&mut self, label: impl ImageLabel + 'static) -> Result<()> {
        let device = &mut self.devices[self.configuring_device as usize];
        let mut resource_manager = device.graph_world.resource_mut::<ResourceManager>();

        match resource_manager.images.resources.remove(&label.type_id()) {
            Some(Image::Persistent { info, image, image_views, mut allocation, descriptor_index, debug_name }) => unsafe {
                let mut image_info = vec![];

                for (offset, image_view) in image_views.iter().enumerate() {
                    image_info.push(ash::vk::DescriptorImageInfo::default());

                    device.logical_device.destroy_image_view(*image_view, None);

                    resource_manager.images.free_descriptors.push(descriptor_index + offset as u32);
                }

                device.logical_device.update_descriptor_sets(
                    &[
                        ash::vk::WriteDescriptorSet::default()
                            .dst_set(resource_manager.descriptor_set)
                            .dst_binding(STORAGE_IMAGE_BINDING)
                            .dst_array_element(descriptor_index)
                            .descriptor_count(1)
                            .descriptor_type(ash::vk::DescriptorType::STORAGE_IMAGE)
                            .image_info(&image_info),
                        ash::vk::WriteDescriptorSet::default()
                            .dst_set(resource_manager.descriptor_set)
                            .dst_binding(SAMPLED_IMAGE_BINDING)
                            .dst_array_element(descriptor_index)
                            .descriptor_count(1)
                            .descriptor_type(ash::vk::DescriptorType::SAMPLED_IMAGE)
                            .image_info(&image_info),
                    ], 
                    &[]
                );

                resource_manager.allocator.destroy_image(image, &mut allocation);
                
            },
            Some(Image::Transient { info, debug_name }) => {
                todo!()
            },
            None => bail!("No image found with label {}", type_name_of_val(&label))
        }

        Ok(())
    }
}