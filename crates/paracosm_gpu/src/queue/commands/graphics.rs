use crate::{
    node::resource::{Read, Write}, 
    queue::commands::CommandRecorder, 
    resource::image::{ImageLabel, ImageView, image_helpers}
};
use crate::resource::surface::SurfaceLabel;

use anyhow::Result;


pub use ash::vk::{
    AttachmentLoadOp,
    AttachmentStoreOp,
    ClearValue,
    ClearColorValue,
    ClearDepthStencilValue,
    ResolveModeFlags as ResolveMode
};


pub struct RenderingInfo {
    pub render_area: (i32, i32, u32, u32),
    //pub layer_count: u32,
    pub color_attachments: Box<[RenderingAttachmentInfo]>,
    pub depth_attachment: Option<RenderingAttachmentInfo>,
    pub stencil_attachment: Option<RenderingAttachmentInfo>
}

pub struct RenderingAttachmentInfo {
    pub image_view: ImageView,
    pub load_op: AttachmentLoadOp,
    pub store_op: AttachmentStoreOp,
    pub clear_value: ClearValue,
    pub resolve: Option<AttachmentResolveInfo>
}

pub struct AttachmentResolveInfo {
    pub mode: ResolveMode,
    pub image_id: u32
}


#[allow(private_bounds)]
pub trait GraphicsCommands: CommandRecorder {
    fn begin_rendering(&mut self, info: RenderingInfo) -> Result<()> {
        let mut color_attachments = vec![];
        
        for color_attachment_info in info.color_attachments.iter() {
            color_attachments.push(
                ash::vk::RenderingAttachmentInfo::default()
                    .image_view(color_attachment_info.image_view.inner)
                    .image_layout(ash::vk::ImageLayout::GENERAL)
                    .load_op(color_attachment_info.load_op)
                    .store_op(color_attachment_info.store_op)
                    .clear_value(color_attachment_info.clear_value)
            );
        }

        let mut rendering_info = ash::vk::RenderingInfo::default()
            .render_area(ash::vk::Rect2D {
                offset: ash::vk::Offset2D { x: info.render_area.0, y: info.render_area.1 },
                extent: ash::vk::Extent2D { width: info.render_area.2, height: info.render_area.3}
            })
            .layer_count(1)
            .color_attachments(&*color_attachments);

        // if let Some(depth_attachment) = info.depth_attachment {
        //     rendering_info = rendering_info.depth_attachment(&ash::vk::RenderingAttachmentInfo::default());
        // }

        let device = self.device();
        let command_buffer = self.command_buffer();

        let scissor = ash::vk::Rect2D::default()
            .offset(
                ash::vk::Offset2D::default()
                    .x(info.render_area.0)
                    .y(info.render_area.1)
            )
            .extent(
                ash::vk::Extent2D::default()
                    .width(info.render_area.2)
                    .height(info.render_area.3)
            );
        unsafe { device.cmd_set_scissor(command_buffer, 0, std::slice::from_ref(&scissor)); };

        let viewport = ash::vk::Viewport::default()
            .x(info.render_area.0 as f32)
            .y(info.render_area.1 as f32)
            .width(info.render_area.2 as f32)
            .height(info.render_area.3 as f32)
            .min_depth(0.0)
            .max_depth(1.0);
        unsafe { device.cmd_set_viewport(command_buffer, 0, std::slice::from_ref(&viewport)); };

        unsafe { self.device().cmd_begin_rendering(self.command_buffer(), &rendering_info) };

        Ok(())
    }

    fn end_rendering(&mut self) {
        unsafe {
            self.device().cmd_end_rendering(self.command_buffer());
        }
    }

    fn draw_mesh(&mut self, group_count: [u32; 3]) -> Result<()> {
        unsafe {
            self.device().mesh.cmd_draw_mesh_tasks(
                self.command_buffer(), 
                group_count[0], 
                group_count[1], 
                group_count[2]
            );
        }

        Ok(())
    }

    fn blit_image_to_surface<I: ImageLabel + 'static, S: SurfaceLabel + 'static>(&mut self, image: Read<I>, surface: Write<S>) -> Result<()> {
        let image = image.image();
        let surface = surface.surface();

        let src_image = image.image;
        let dst_image = surface.images[surface.image_index as usize];

        let regions = [
            ash::vk::ImageBlit::default()
                .src_subresource(
                    ash::vk::ImageSubresourceLayers::default()
                        .aspect_mask(image_helpers::aspect_from_format(image.info.format))
                        .mip_level(0)
                        .base_array_layer(0)
                        .layer_count(ash::vk::REMAINING_ARRAY_LAYERS)
                )
                .src_offsets([
                    ash::vk::Offset3D { x: 0, y: 0, z: 0 },
                    ash::vk::Offset3D { x: image.info.extent[0] as i32, y: image.info.extent[1] as i32, z: 1 }
                ])
                .dst_subresource(
                    ash::vk::ImageSubresourceLayers::default()
                        .aspect_mask(image_helpers::aspect_from_format(image.info.format))
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
                src_image, 
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