pub mod buffer;
pub mod image;
pub mod surface;

use crate::{context::Context, resource::{buffer::{Buffer, BufferHandle, BufferLabel}, image::{Image, ImageHandle, ImageLabel}}};

use anyhow::{Result, bail};
use bevy_ecs::{component::Component, entity::Entity};

use std::marker::PhantomData;



pub const BUFFER_BINDING: u32 = 0;
pub const STORAGE_IMAGE_BINDING: u32 = 1;
pub const SAMPLED_IMAGE_BINDING: u32 = 2;
pub const SAMPLER_BINDING: u32 = 3;
pub const ACCELERATION_STRUCTURE_BINDING: u32 = 4;

pub trait ResourceHandle {
    fn host_entity(&self) -> Entity;
}

pub trait SyncLabel: Send + Sync { }

#[derive(Component, Default)]
pub(crate) struct SyncLabelComponent<L: SyncLabel> {
    _marker: PhantomData<L>
}

#[derive(Component)]
struct SyncMarker;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    #[default] Auto,
    AutoUpload,
    AutoDownload,
    Stream,
    //Manual
}

impl Context {
    pub fn set_buffer_label<L: BufferLabel + 'static>(
        &mut self,
        _label: L,
        handle: &BufferHandle
    ) -> Result<()> {
        let device = &mut self.active_device;

        // Validate resource to be synced
        if device.world.get::<SyncMarker>(handle.host_entity()).is_some() {
            bail!("Buffer resource already synchronized.");
        }

        let Some(buffer) = device.world.get::<Buffer>(handle.host_entity()) else {
            bail!("Buffer not found.")
        };
        if !buffer.info.shader_mutable {
            bail!("Buffer resource must be created with shader mutability enabled.")
        }

        // Remove SyncLabelComponent from old synced resource if any
        if let Ok((old_entity, _)) = device.world.query::<(Entity, &SyncLabelComponent<L>)>().single(&device.world) {
            device.world.entity_mut(old_entity).remove::<(SyncLabelComponent<L>, SyncMarker)>();
        }
                
        // Add SyncLabelComponent to new resource    
        device.world.entity_mut(handle.host_entity()).insert((
            SyncLabelComponent::<L> {
                _marker: Default::default()
            },
            SyncMarker
        ));

        Ok(())
    }

    pub fn set_image_label<L: ImageLabel + 'static>(
        &mut self,
        _label: L,
        handle: &ImageHandle
    ) -> Result<()> {
        let device = &mut self.active_device;

        // Validate resource to be synced
        if device.world.get::<SyncMarker>(handle.host_entity()).is_some() {
            bail!("Image resource already synchronized.");
        }

        let Some(image) = device.world.get::<Image>(handle.host_entity()) else {
            bail!("Image not found.")
        };
        if !image.info.shader_mutable {
            bail!("Image resource must be created with shader mutability enabled.")
        }

        // Remove SyncLabelComponent from old synced resource if any
        if let Ok((old_entity, _)) = device.world.query::<(Entity, &SyncLabelComponent<L>)>().single(&device.world) {
            device.world.entity_mut(old_entity).remove::<(SyncLabelComponent<L>, SyncMarker)>();
        }
                
        // Add SyncLabelComponent to new resource    
        device.world.entity_mut(handle.host_entity()).insert((
            SyncLabelComponent::<L> {
                _marker: Default::default()
            },
            SyncMarker
        ));

        Ok(())
    }
}
