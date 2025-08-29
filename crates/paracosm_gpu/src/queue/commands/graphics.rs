use crate::{
    resource::{
        image::{Image, ImageLabel}
    },
    node::resource::{Read, Write},
    queue::commands::CommandRecorder
};
use crate::resource::surface::SurfaceLabel;

use anyhow::{Context, Result};

use std::any::TypeId;



#[allow(private_bounds)]
pub trait GraphicsCommands: CommandRecorder {
    fn blit_image_to_surface<I: ImageLabel + 'static, S: SurfaceLabel + 'static>(&mut self, _image: Read<I>, _surface: Write<S>) -> Result<()> {
        let resource_manager = self.resources();

        let (src_info, src_image) = match resource_manager.images.get(TypeId::of::<I>()).context("Source image not found.")? {
            Image::Persistent { info, image, image_views, allocation, descriptor_index, debug_name } => {
                (info, image)
            },
            Image::Transient { info, debug_name } => {
                todo!()
            }
        };
        let surface = resource_manager.surfaces.get(TypeId::of::<S>()).context("Surface not found.")?;
        let dst_image = surface.images[surface.image_index as usize];

        let regions = [
            ash::vk::ImageBlit::default()
                .src_subresource(
                    ash::vk::ImageSubresourceLayers::default()
                        .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(ash::vk::REMAINING_ARRAY_LAYERS)
                )
                .src_offsets([
                    ash::vk::Offset3D { x: 0, y: 0, z: 0 },
                    ash::vk::Offset3D { x: src_info.extent[0] as i32, y: src_info.extent[1] as i32, z: 1 }
                ])
                .dst_subresource(
                    ash::vk::ImageSubresourceLayers::default()
                        .aspect_mask(ash::vk::ImageAspectFlags::COLOR)
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(ash::vk::REMAINING_ARRAY_LAYERS)
                )
                .dst_offsets([
                    ash::vk::Offset3D { x: 0, y: 0, z: 0 },
                    ash::vk::Offset3D { x: surface.extent.width as i32, y: surface.extent.height as i32, z: 1 }
                ])
        ];
        
        unsafe { 
            self.device().cmd_blit_image(
                self.command_buffer(), 
                *src_image, 
                ash::vk::ImageLayout::GENERAL, 
                dst_image, 
                ash::vk::ImageLayout::GENERAL, 
                &regions, 
                ash::vk::Filter::NEAREST
            ); 
        }

        Ok(())
    }
}