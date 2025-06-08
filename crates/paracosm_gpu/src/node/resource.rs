use crate::resource::ResourceLabel;

use bevy_ecs::system::{Res, ResMut, Resource, SystemParam};

use std::marker::PhantomData;


// Resource access parameters
#[derive(SystemParam)]
pub struct Read<'w, L: ResourceLabel + 'static> {
    res: Res<'w, ResourceIndex<L>>,
}

#[derive(SystemParam)]
pub struct Write<'w, L: ResourceLabel + 'static> {
    res: ResMut<'w, ResourceIndex<L>>,
}

#[derive(Resource)]
pub(crate) enum ResourceIndex<L: ResourceLabel> {
    Buffer { index: u32, label: PhantomData<L> },
    ImageView { index: u32, label: PhantomData<L> },
    AccelerationStructure { index: u32, label: PhantomData<L> }
}