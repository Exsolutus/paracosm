use crate::{prelude::SurfaceLabel, resource::{SyncLabel, SyncLabelComponent, buffer::{Buffer, BufferLabel}, image::{Image, ImageLabel}, surface::Surface}};

use anyhow::Result;
use bevy_ecs::{query::AnyOf, system::{Query, SystemParam}};

use std::any::type_name;



// Resource access parameters
#[derive(SystemParam)]
pub struct Read<'w, 's, L: SyncLabel + 'static> {
    entity: Query<'w, 's, (
        &'static SyncLabelComponent<L>, 
        AnyOf<(&'static Buffer, &'static Image, &'static Surface)>
    )>
}

impl<'w, 's, L: BufferLabel + 'static> Read<'w, 's, L> {
    pub fn buffer(&self) -> &Buffer {
        let Ok((_, (Some(buffer), _, _))) = self.entity.single() else {
            panic!("No resource found for label {}", type_name::<L>())
        };

        buffer
    }
}

impl<'w, 's, L: ImageLabel + 'static> Read<'w, 's, L> {
    pub fn image(&self) -> &Image {
        let Ok((_, (_, Some(image), _))) = self.entity.single() else {
            panic!("No resource found for label {}", type_name::<L>())
        };

        image
    }
}

impl<'w, 's, L: SurfaceLabel + 'static> Read<'w, 's, L> {
    pub(crate) fn surface(&self) -> &Surface {
        let Ok((_, (_, _, Some(surface)))) = self.entity.single() else {
            panic!("No resource found for label {}", type_name::<L>())
        };

        surface
    }
}

#[derive(SystemParam)]
pub struct Write<'w, 's, L: SyncLabel + 'static> {
    entity: Query<'w, 's, (
        &'static SyncLabelComponent<L>, 
        AnyOf<(&'static Buffer, &'static Image, &'static Surface)>
    )>
}

impl<'w, 's, L: BufferLabel + 'static> Write<'w, 's, L> {
    pub fn buffer(&self) -> &Buffer {
        let Ok((_, (Some(buffer), _, _))) = self.entity.single() else {
            panic!("No resource found for label {}", type_name::<L>())
        };

        buffer
    }
}

impl<'w, 's, L: ImageLabel + 'static> Write<'w, 's, L> {
    pub fn image(&self) -> &Image {
        let Ok((_, (_, Some(image), _))) = self.entity.single() else {
            panic!("No resource found for label {}", type_name::<L>())
        };

        image
    }
}

impl<'w, 's, L: SurfaceLabel + 'static> Write<'w, 's, L> {
    pub(crate) fn surface(&self) -> &Surface {
        let Ok((_, (_, _, Some(surface)))) = self.entity.single() else {
            panic!("No resource found for label {}", type_name::<L>())
        };

        surface
    }
}