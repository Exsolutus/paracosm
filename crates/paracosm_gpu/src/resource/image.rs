use crate::resource::{ResourceHandle, SAMPLED_IMAGE_BINDING, STORAGE_IMAGE_BINDING, SyncLabel};

use super::{TransferMode};

use anyhow::{Context, Ok, Result, bail};
use ash::vk::{ImageAspectFlags, ImageUsageFlags};
use bevy_ecs::{component::Component, entity::Entity};
use vk_mem::{Alloc, AllocationCreateFlags, AllocationCreateInfo, MemoryUsage};

// Reexport
pub use ash::vk::{
    Format,
};



pub struct ImageHandle {
    host_entity: Entity
}

impl ResourceHandle for ImageHandle {
    fn host_entity(&self) -> Entity { self.host_entity }
}

pub trait ImageLabel: SyncLabel { }

#[derive(Clone, Copy, Default)]
pub struct ImageInfo {
    pub format: ash::vk::Format,
    /// Image extent with format \[width, height, depth\]
    pub extent: [u32; 3],
    pub mip_levels: u32,
    pub array_layers: u32,
    pub samples: ash::vk::SampleCountFlags,
    pub shared: bool,
    pub transfer_mode: TransferMode,
    pub shader_mutable: bool,
    #[cfg(debug_assertions)] pub debug_name: &'static str
}

#[derive(Clone, Copy)]
pub struct ImageView {
    pub descriptor_index: u32,

    pub(crate) inner: ash::vk::ImageView,
}

#[derive(Component)]
pub struct Image {
    pub info: ImageInfo,

    pub(crate) image: ash::vk::Image,
    pub(crate) allocation: vk_mem::Allocation,
    pub(crate) image_views: Box<[ImageView]>
}

impl Image {
    pub fn view(&self, index: u32) -> ImageView {
        self.image_views[index as usize]
    }
}

pub(crate) mod image_helpers {
    use ash::vk::{Format, ImageAspectFlags};

    fn is_depth_format(format: Format) -> bool {
        match format {
            Format::D16_UNORM |
            Format::D16_UNORM_S8_UINT |
            Format::D24_UNORM_S8_UINT |
            Format::D32_SFLOAT |
            Format::D32_SFLOAT_S8_UINT |
            Format::X8_D24_UNORM_PACK32 => true,
            _ => false
        }
    }

    fn is_stencil_format(format: Format) -> bool {
        match format {
            Format::S8_UINT |
            Format::D16_UNORM_S8_UINT |
            Format::D24_UNORM_S8_UINT |
            Format::D32_SFLOAT_S8_UINT => true,
            _ => false
        }
    }

    pub fn aspect_from_format(format: Format) -> ImageAspectFlags {
        match (is_depth_format(format), is_stencil_format(format)) {
            (false, false) => ImageAspectFlags::COLOR,
            (true, false) => ImageAspectFlags::DEPTH,
            (false, true) => ImageAspectFlags::STENCIL,
            (true, true) => ImageAspectFlags::DEPTH | ImageAspectFlags::STENCIL
        }
    }
}

pub struct SamplerInfo {

}

pub(crate) struct Sampler {

}


impl crate::context::Context {
    pub fn create_image(
        &mut self,
        info: ImageInfo
    ) -> Result<ImageHandle> {
        let device = &mut self.active_device;

        // Create new image
        let (image_type, image_extent) = match info.extent {
            [0, _, _] => bail!("Image width must be greater than 0."),
            [width, 0, 0] => (ash::vk::ImageType::TYPE_1D, ash::vk::Extent3D { width, height: 1, depth: 1 }),
            [width, height, 0] => (ash::vk::ImageType::TYPE_2D, ash::vk::Extent3D { width, height, depth: 1 }),
            [width, height, depth] => (ash::vk::ImageType::TYPE_3D, ash::vk::Extent3D { width, height, depth }),
        };

        let mut image_usage = {
            ash::vk::ImageUsageFlags::STORAGE | 
            ash::vk::ImageUsageFlags::SAMPLED | 
            ash::vk::ImageUsageFlags::TRANSFER_SRC | 
            ash::vk::ImageUsageFlags::TRANSFER_DST
        };
        let aspect_mask = image_helpers::aspect_from_format(info.format);
        if aspect_mask.contains(ImageAspectFlags::COLOR) {
            image_usage |= ImageUsageFlags::COLOR_ATTACHMENT;
        }
        else if aspect_mask.contains(ImageAspectFlags::DEPTH) || aspect_mask.contains(ImageAspectFlags::STENCIL) {
            image_usage |= ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT;
        }

        let image_create_info = ash::vk::ImageCreateInfo::default()
            .flags(match image_type {
                ash::vk::ImageType::TYPE_3D => ash::vk::ImageCreateFlags::CUBE_COMPATIBLE,
                _ => ash::vk::ImageCreateFlags::empty()
            })
            .usage(image_usage)
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

        let (image, allocation) = unsafe {
            device.allocator.create_image(&image_create_info, &allocation_create_info)?
        };

        // Transition image to ImageLayout::GENERAL
        unsafe {
            device.begin_command_buffer(
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
                        .aspect_mask(aspect_mask)
                        .base_array_layer(0)
                        .layer_count(ash::vk::REMAINING_ARRAY_LAYERS)
                        .base_mip_level(0)
                        .level_count(ash::vk::REMAINING_MIP_LEVELS)
                );
            device.cmd_pipeline_barrier2(
                device.graphics_graph.immediate_command_buffer, 
                &ash::vk::DependencyInfo::default()
                    .image_memory_barriers(std::slice::from_ref(&barrier))
            );

            device.end_command_buffer(device.graphics_graph.immediate_command_buffer)?;

            let command_buffer_info = ash::vk::CommandBufferSubmitInfo::default()
                .command_buffer(device.graphics_graph.immediate_command_buffer);
            let submit_info = ash::vk::SubmitInfo2::default()
                .command_buffer_infos(std::slice::from_ref(&command_buffer_info));
            device.queue_submit2(device.graphics_graph.queue, std::slice::from_ref(&submit_info), ash::vk::Fence::null())?;

            device.queue_wait_idle(device.graphics_graph.queue)?;
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
                    .aspect_mask(aspect_mask)
                    .level_count(info.mip_levels)
                    .layer_count(info.array_layers)
            );
        let image_view = unsafe { device.create_image_view(&image_view_info, None)? };
        let descriptor_index = match device.image_view_descriptors.free_descriptors.pop() {
            Some(index) => index,
            None => {
                let index = device.image_view_descriptors.next_descriptor;
                device.image_view_descriptors.next_descriptor += 1;
                index
            }
        };
        let image_views = Box::new([ImageView { descriptor_index, inner: image_view }]);

        let host_entity = device.world.spawn(
            Image { info, image, allocation, image_views }
        ).id();

        let image_info = [
            ash::vk::DescriptorImageInfo::default()
                .image_view(image_view)
                .image_layout(ash::vk::ImageLayout::GENERAL)
        ];
        unsafe { device.update_descriptor_sets(
            &[
                ash::vk::WriteDescriptorSet::default()
                    .dst_set(device.descriptor_set)
                    .dst_binding(STORAGE_IMAGE_BINDING)
                    .dst_array_element(descriptor_index)
                    .descriptor_count(1)
                    .descriptor_type(ash::vk::DescriptorType::STORAGE_IMAGE)
                    .image_info(&image_info),
                ash::vk::WriteDescriptorSet::default()
                    .dst_set(device.descriptor_set)
                    .dst_binding(SAMPLED_IMAGE_BINDING)
                    .dst_array_element(descriptor_index)
                    .descriptor_count(1)
                    .descriptor_type(ash::vk::DescriptorType::SAMPLED_IMAGE)
                    .image_info(&image_info),
            ], 
            &[]
        ); }

        #[cfg(debug_assertions)]
        unsafe {
            let image_name = std::ffi::CString::new(format!("Storage Image: {}", info.debug_name))?;
            let image_name_info = ash::vk::DebugUtilsObjectNameInfoEXT::default()
                .object_handle(image)
                .object_name(&image_name);
            self.active_device.debug_utils.set_debug_utils_object_name(&image_name_info)?;
        }

        Ok(ImageHandle { host_entity })
    }

    pub fn get_image(&self, handle: &ImageHandle) -> Result<&Image> {
        self.active_device.world.get::<Image>(handle.host_entity)
            .context("Resource is not an Image.")
    }

    pub fn destroy_image(&mut self, handle: ImageHandle) -> Result<()> {
        let device = &mut self.active_device;

        let mut entity = device.world.entity_mut(handle.host_entity);
        let Some(mut image) = entity.take::<Image>() else {
            bail!("Resource is not an Image.")
        };
        device.world.despawn(handle.host_entity);

        unsafe { device.allocator.destroy_image(image.image, &mut image.allocation) };

        for image_view in image.image_views.iter() {
            device.image_view_descriptors.free_descriptors.push(image_view.descriptor_index);

            let image_info = [ash::vk::DescriptorImageInfo::default()];

            unsafe { device.update_descriptor_sets(
                &[
                    ash::vk::WriteDescriptorSet::default()
                        .dst_set(device.descriptor_set)
                        .dst_binding(STORAGE_IMAGE_BINDING)
                        .dst_array_element(image_view.descriptor_index)
                        .descriptor_count(1)
                        .descriptor_type(ash::vk::DescriptorType::STORAGE_IMAGE)
                        .image_info(&image_info),
                    ash::vk::WriteDescriptorSet::default()
                        .dst_set(device.descriptor_set)
                        .dst_binding(SAMPLED_IMAGE_BINDING)
                        .dst_array_element(image_view.descriptor_index)
                        .descriptor_count(1)
                        .descriptor_type(ash::vk::DescriptorType::SAMPLED_IMAGE)
                        .image_info(&image_info),
                ], 
                &[]
            ); }

            unsafe { device.destroy_image_view(image_view.inner, None) };
        }

        Ok(())
    }
}