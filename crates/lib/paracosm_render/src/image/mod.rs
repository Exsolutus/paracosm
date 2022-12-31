
use ash::vk;
use anyhow::{bail, Result};
use bevy_app::{App, AppExit, Plugin, CoreStage};
use bevy_asset::{AddAsset, AssetEvent, AssetLoader, Assets, AssetServer, Handle, LoadContext, LoadedAsset};
use bevy_ecs::{prelude::*, schedule::ShouldRun, system::Resource};
use bevy_log::prelude::*;
use bevy_reflect::{TypeUuid};
use bevy_utils::{BoxedFuture, HashSet};

use image::{
    DynamicImage,
    ImageFormat,
    io::Reader as ImageReader
};

use paracosm_gpu::device::Device;
use paracosm_gpu::resource::{buffer as gpu_buffer, image as gpu_image, sampler as gpu_sampler};

use std::io::Cursor;
use std::collections::HashMap;
use std::mem::size_of;
use std::ops::Deref;


#[derive(TypeUuid)]
#[uuid = "258d6fb5-6314-4816-9771-c24eb249abfe"]
pub struct Image {
    image: DynamicImage,
    pub gpu_image: Option<gpu_image::Image>,
    pub sampler: Option<gpu_sampler::Sampler>
}

impl Deref for Image {
    type Target = DynamicImage;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

impl Image {
    pub fn upload(&mut self, device: &Device) -> Result<bool> {
        if self.gpu_image.is_some() {
            return Ok(false)    // Skipped upload, since already uploaded
        }

        // Create staging buffer
        let size = (self.image.width() * self.image.height()) as usize * size_of::<u32>();
        let info = gpu_buffer::BufferInfo::new(size, gpu_buffer::BufferUsageFlags::TRANSFER_SRC, gpu_buffer::MemoryLocation::CpuToGpu);
        let staging_buffer = device.create_buffer("Image Staging Buffer", info, None)?;

        // Copy data to staging buffer
        staging_buffer.write_buffer(&self.image.as_bytes().to_vec())?;

        // Create GPU image
        let create_info = gpu_image::ImageInfo {
            image_type: gpu_image::ImageType::TYPE_2D,
            image_format: gpu_image::Format::R8G8B8A8_SRGB,
            image_extent: gpu_image::Extent3D { width: self.width(), height: self.height(), depth: 1 },
            mip_levels: 1,
            array_layers: 1,
            samples: gpu_image::SampleCountFlags::TYPE_1,
            tiling: gpu_image::ImageTiling::OPTIMAL,
            usage: gpu_image::ImageUsageFlags::SAMPLED | gpu_image::ImageUsageFlags::TRANSFER_DST,
            aspect: gpu_image::ImageAspectFlags::COLOR,
            memory_location: gpu_image::MemoryLocation::GpuOnly
        };
        let image = device.create_image("Image", create_info, None)?;

        // Copy from staging buffer to GPU image
        let command_buffer = device.begin_transfer_commands()?;

        device.transition_image_layout(
            command_buffer,
            &image,
            //gpu_image::Format::R8G8B8A8_SRGB,
            gpu_image::ImageLayout::UNDEFINED,
            gpu_image::ImageLayout::TRANSFER_DST_OPTIMAL
        )?;

        device.copy_buffer_to_image(command_buffer, &staging_buffer, &image)?;

        device.end_transfer_commands(command_buffer)?;

        // Create image sampler
        let create_info = gpu_sampler::SamplerInfo {
            filter: (gpu_sampler::Filter::LINEAR, gpu_sampler::Filter::LINEAR),
            address_mode: (gpu_sampler::SamplerAddressMode::REPEAT, gpu_sampler::SamplerAddressMode::REPEAT, gpu_sampler::SamplerAddressMode::REPEAT),
            anisotropy: Some(16.0),
            border_color: gpu_sampler::BorderColor::INT_OPAQUE_BLACK,
            unnormalized_coordinates: false,
            compare_op: None,
            mipmap_mode: gpu_sampler::SamplerMipmapMode::LINEAR,
            mipmap_lod: (0.0, 0.0, 0.0)
        };
        let sampler = device.create_sampler(create_info)?;


        self.gpu_image = Some(image);
        self.sampler = Some(sampler);
        Ok(true)
    }
}



#[derive(Resource)]
pub struct ImageManager {
    pub images: HashMap<String, Handle<Image>>
}

pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        #[cfg(any(
            feature = "png",
        ))]
        {
            app.add_asset::<Image>()
                .init_asset_loader::<ImageLoader>();

            app.insert_resource(ImageManager {
                images: HashMap::new()
            });
        }
    }
}

/// An [`AssetLoader`] for images.
#[derive(Default)]
pub struct ImageLoader;

impl AssetLoader for ImageLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let name = load_context.path().file_name();

            let image = match ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png).decode() {
                Ok(result) => result,
                Err(error) => bail!("Failed to load png: {}", error.to_string())
            };

            let asset = LoadedAsset::new(Image {
                image,
                gpu_image: None,
                sampler: None
            });

            load_context.set_default_asset(asset);
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["png"]
    }
}