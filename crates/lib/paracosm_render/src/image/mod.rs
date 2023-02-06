use crate::{
    render_asset::*,
    RenderContext
};

use anyhow::{bail, Result};
use bevy_app::Plugin;
use bevy_asset::{AddAsset, AssetLoader, Handle, LoadContext, LoadedAsset};
use bevy_ecs::{
    system::{
        lifetimeless::SRes,
        Resource,
    }
};

use bevy_reflect::{TypeUuid};
use bevy_utils::BoxedFuture;

use image::{
    DynamicImage,
    ImageFormat,
    io::Reader as ImageReader
};

use paracosm_gpu::resource::{
    buffer as gpu_buffer, 
    image as gpu_image, 
    sampler as gpu_sampler
};
use rust_shaders_shared::ResourceHandle;

use std::io::Cursor;
use std::collections::HashMap;
use std::mem::size_of;
use std::ops::Deref;



// TODO: remove when adding proper scene management
#[derive(Default, Resource)]
pub struct ImageManager {
    pub images: HashMap<String, Handle<Image>>
}

#[derive(Default, Resource)]
pub struct SamplerManager {
    pub samplers: HashMap<String, Handle<Sampler>>
}

/// Adds [`Image`] to Bevy as a supported asset type
pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_asset::<Image>()
            .add_asset::<Sampler>()
            .add_plugin(RenderAssetPlugin::<Image>::default())
            .add_plugin(RenderAssetPlugin::<Sampler>::default());

        app.insert_resource(ImageManager::default());
        app.insert_resource(SamplerManager::default());

        #[cfg(any(
            feature = "png",
        ))]
        {
            app.init_asset_loader::<ImageLoader>();
        }
    }
}

/// An [`AssetLoader`] for image assets.
#[derive(Default)]
pub struct ImageLoader;

impl AssetLoader for ImageLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, anyhow::Result<(), anyhow::Error>> {
        Box::pin(async move {
            let image = match ImageReader::with_format(Cursor::new(bytes), ImageFormat::Png).decode() {
                Ok(result) => result,
                Err(error) => bail!("Failed to load png: {}", error.to_string())
            };

            let asset = LoadedAsset::new(Image(image));

            load_context.set_default_asset(asset);
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["png"]
    }
}



#[derive(TypeUuid)]
#[uuid = "258d6fb5-6314-4816-9771-c24eb249abfe"]
#[repr(transparent)]
pub struct Image(DynamicImage);

impl Deref for Image {
    type Target = DynamicImage;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


pub struct GpuImage {
    pub image: gpu_image::Image,
    pub handle: ResourceHandle,
}


impl RenderAsset for Image {
    type PreparedAsset = GpuImage;
    type Param = SRes<RenderContext>;

    fn prepare_asset(
        source_asset: &Self,
        param: &mut bevy_ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, crate::render_asset::PrepareAssetError> {
        let device = &param.device;
        let resource_manager = &param.resource_manager;

        // Create staging buffer
        let size = (source_asset.width() * source_asset.height()) as usize * size_of::<u32>();
        let info = gpu_buffer::BufferInfo::new(size, gpu_buffer::BufferUsageFlags::TRANSFER_SRC, gpu_buffer::MemoryLocation::CpuToGpu);
        let staging_buffer = device.create_buffer("Image Staging Buffer", info, None);

        // Copy data to staging buffer
        staging_buffer.write_buffer(&source_asset.as_bytes().to_vec());

        // Create GPU image
        let create_info = gpu_image::ImageInfo {
            image_type: gpu_image::ImageType::TYPE_2D,
            image_format: gpu_image::Format::R8G8B8A8_SRGB,
            image_extent: gpu_image::Extent3D { width: source_asset.width(), height: source_asset.height(), depth: 1 },
            mip_levels: 1,
            array_layers: 1,
            samples: gpu_image::SampleCountFlags::TYPE_1,
            tiling: gpu_image::ImageTiling::OPTIMAL,
            usage: gpu_image::ImageUsageFlags::SAMPLED | gpu_image::ImageUsageFlags::TRANSFER_DST,
            aspect: gpu_image::ImageAspectFlags::COLOR,
            memory_location: gpu_image::MemoryLocation::GpuOnly
        };
        let image = device.create_image("Image", create_info, None);
        let handle = resource_manager.new_sampled_image_handle(&image);

        // Copy from staging buffer to GPU image
        device.copy_buffer_to_image(&staging_buffer, &image);

        Ok(GpuImage {
            image,
            handle,
        })
    }
}



#[derive(Clone, TypeUuid)]
#[uuid = "ae24f47e-e189-44a1-945d-da652e87944c"]
#[repr(transparent)]
pub struct Sampler(gpu_sampler::SamplerInfo);

impl Deref for Sampler {
    type Target = gpu_sampler::SamplerInfo;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Sampler {
    pub fn new(
        filter: (gpu_sampler::Filter, gpu_sampler::Filter),
        address_mode: (gpu_sampler::SamplerAddressMode, gpu_sampler::SamplerAddressMode, gpu_sampler::SamplerAddressMode),
        anisotropy: Option<f32>,
        border_color: gpu_sampler::BorderColor,
        unnormalized_coordinates: bool,
        compare_op: Option<gpu_sampler::CompareOp>,
        mipmap_mode: gpu_sampler::SamplerMipmapMode,
        mipmap_lod: (f32, f32, f32)
    ) -> Self {
        Self(gpu_sampler::SamplerInfo {
            filter,
            address_mode,
            anisotropy,
            border_color,
            unnormalized_coordinates,
            compare_op,
            mipmap_mode,
            mipmap_lod
        })
    }
}

pub struct GpuSampler {
    pub sampler: gpu_sampler::Sampler,
    pub handle: ResourceHandle
}

impl RenderAsset for Sampler {
    type PreparedAsset = GpuSampler;
    type Param = SRes<RenderContext>;

    fn prepare_asset(
        source_asset: &Self,
        param: &mut bevy_ecs::system::SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError> {
        let device = &param.device;
        let resource_manager = &param.resource_manager;

        let sampler_info = &source_asset.0;

        let sampler = device.create_sampler(sampler_info);
        let handle = resource_manager.new_sampler_handle(&sampler);

        Ok(GpuSampler {
            sampler,
            handle
        })
    }
}
