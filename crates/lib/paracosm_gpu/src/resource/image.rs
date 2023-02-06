use crate::device::Device;
use crate::resource::buffer::Buffer;

use anyhow::{Result, bail};
use ash::vk;

use bevy_log::prelude::*;

use gpu_allocator::vulkan::*;

use std::slice;
use std::ptr::copy_nonoverlapping as memcpy;

// re-export
pub use vk::{
    ImageViewType as ImageType,
    Format,
    Extent3D,
    SampleCountFlags,
    ImageTiling,
    ImageUsageFlags,
    ImageAspectFlags,
    ImageLayout,
    ImageSubresourceRange
};
pub use gpu_allocator::MemoryLocation;



pub struct ImageInfo {
    pub image_type: ImageType,
    pub image_format: Format,
    pub image_extent: Extent3D,
    pub mip_levels: u32,
    pub array_layers: u32,
    pub samples: SampleCountFlags,
    pub tiling: ImageTiling,
    pub usage: ImageUsageFlags,
    pub aspect: ImageAspectFlags,
    pub memory_location: MemoryLocation,
    //pub alignment: Option<u64>
}

//#[derive(Debug)]
pub struct Image {
    device: Device,
    cleanup: bool,
    pub info: ImageInfo,
    pub image: vk::Image,
    pub image_view: vk::ImageView,
    pub(crate) allocation: Option<Allocation>,
}

impl Image {
    pub fn from_vk(device: &Device, image: vk::Image, info: ImageInfo) -> Result<Self> {
        let create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(info.image_type)
            .format(info.image_format)
            .subresource_range(
                vk::ImageSubresourceRange::builder()
                    .aspect_mask(info.aspect)
                    .base_mip_level(0)
                    .level_count(info.mip_levels)
                    .base_array_layer(0)
                    .layer_count(info.array_layers)
                    .build()
            );
        let image_view = unsafe {
            device.create_image_view(&create_info, None)?
        };

        Ok(Self {
            device: device.clone(),
            cleanup: false,
            info,
            image,
            image_view,
            allocation: None
        })
    }

    pub fn write_image<T>(
        &self,
        data: &Vec<T>
    ) -> Result<()> {
        let allocation = match &self.allocation {
            Some(value) => value,
            None => bail!("Image has no active allocation")
        };
        let memory = match allocation.mapped_ptr() {
            Some(value) => value.as_ptr(),
            None => bail!("Image allocation is not host visible")
        };
        unsafe { memcpy(data.as_ptr(), memory.cast(), data.len()) };

        Ok(())
    }

    pub fn extent(&self) -> Extent3D {
        self.info.image_extent
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            // TODO: look into waiting on queue idle instead
            self.device.device_wait_idle().unwrap();

            match self.allocation.take() {
                Some(value) => {
                    match self.device.allocator
                        .as_ref()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .free(value)
                    {
                        Ok(_) => (),
                        Err(error) => debug!("{}", error.to_string())
                    };
                },
                None => ()
            };

            self.device.destroy_image_view(self.image_view, None);
            if self.cleanup {
                self.device.destroy_image(self.image, None);
            }
        }
    }
}


impl Device {
    pub fn create_image(
        &self, 
        name: &str, 
        info: ImageInfo,
        data: Option<&[u8]>
    ) -> Image {
        // Create image
        let image_type = match info.image_type {
            vk::ImageViewType::TYPE_2D => vk::ImageType::TYPE_2D,
            _ => panic!("Unsupported image type")
        };
        let create_info = vk::ImageCreateInfo::builder()
            .image_type(image_type)
            .format(info.image_format)
            .extent(info.image_extent)
            .mip_levels(info.mip_levels)
            .array_layers(info.array_layers)
            .samples(info.samples)
            .tiling(info.tiling)
            .usage(info.usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        let image = unsafe { 
            self.logical_device.create_image(&create_info, None)
                .expect("Device should create an image.")
        };
        let requirements = unsafe { self.logical_device.get_image_memory_requirements(image) };

        let allocation = self.allocator
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .allocate(&AllocationCreateDesc {
                name,
                requirements,
                location: info.memory_location,
                linear: true
            })
            .expect("Image memory should be allocated.");

        unsafe {
            self.bind_image_memory(image, allocation.memory(), allocation.offset())
                .expect("Image memory should be bound on device.")
        };

        // Create image view
        let create_info = vk::ImageViewCreateInfo::builder()
            .view_type(info.image_type)
            .image(image)
            .format(info.image_format)
            .subresource_range(vk::ImageSubresourceRange::builder()
                .base_mip_level(0)
                .level_count(info.mip_levels)
                .base_array_layer(0)
                .layer_count(info.array_layers)
                .aspect_mask(info.aspect)
                .build()
            );

        let image_view = unsafe {
            self.logical_device.create_image_view(&create_info, None)
                .expect("Device should create an image view.")
        };

        Image {
            device: self.clone(),
            cleanup: true,
            info,
            image,
            image_view,
            allocation: Some(allocation)
        }
    }

    pub fn transition_image_layout(
        &self,
        command_buffer: vk::CommandBuffer,
        image: &Image,
        old_layout: ImageLayout,
        new_layout: ImageLayout
    ) {
        let (
            src_access_mask,
            dst_access_mask,
            src_stage_mask,
            dst_stage_mask,
        ) = match (old_layout, new_layout) {
            // Color attachment transitions
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
            ),
            (vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR) => (
                vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
                vk::AccessFlags::empty(),
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
            ),
            // Depth attachment transitions
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_READ | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
            ),
            // Data transfer transitions
            (vk::ImageLayout::UNDEFINED, vk::ImageLayout::TRANSFER_DST_OPTIMAL) => (
                vk::AccessFlags::empty(),
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TOP_OF_PIPE,
                vk::PipelineStageFlags::TRANSFER,
            ),
            (vk::ImageLayout::TRANSFER_DST_OPTIMAL, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL) => (
                vk::AccessFlags::TRANSFER_WRITE,
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            ),
            _ => panic!("Unsupported image layout transition!"),
        };

        let image_barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(old_layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(image.image)
            .subresource_range(ImageSubresourceRange::builder()
                .aspect_mask(image.info.aspect)
                .base_mip_level(0)
                .level_count(image.info.mip_levels)
                .base_array_layer(0)
                .layer_count(image.info.array_layers)
                .build()
            )
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask);
        unsafe {
            self.cmd_pipeline_barrier(
                command_buffer, 
                src_stage_mask,
                dst_stage_mask,
                vk::DependencyFlags::empty(), 
                &[], 
                &[], 
                slice::from_ref(&image_barrier)
            );
        }
    }
    
    // TODO: robustness/safety for general usage
    pub fn copy_image(
        &self,
        command_buffer: vk::CommandBuffer,
        source: &Image,
        destination: &Image,
    ) -> Result<()> {
        unsafe {
            let regions = vk::ImageCopy::builder()
                .extent(source.extent());
            self.cmd_copy_image(
                command_buffer,
                source.image,
                vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
                destination.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                slice::from_ref(&regions)
            );
        }

        Ok(())
    }

    pub fn copy_buffer_to_image(
        &self,
        buffer: &Buffer,
        image: &Image,
    ) {
        let command_buffer = self.begin_transfer_commands()
            .expect("Transfer command buffer should begin recording.");

        self.transition_image_layout(
            command_buffer,
            &image,
            ImageLayout::UNDEFINED,
            ImageLayout::TRANSFER_DST_OPTIMAL
        );

        unsafe {
            let regions = vk::BufferImageCopy::builder()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(
                    vk::ImageSubresourceLayers::builder()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(1)
                        .build()
                )
                .image_offset(vk::Offset3D { x: 0, y: 0, z: 0 })
                .image_extent(image.extent());

            self.cmd_copy_buffer_to_image(
                command_buffer,
                buffer.buffer,
                image.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                slice::from_ref(&regions),
            );
        }

        self.end_transfer_commands(command_buffer)
            .expect("Transfer command buffer should end recording and submit to device.");
    }
}
