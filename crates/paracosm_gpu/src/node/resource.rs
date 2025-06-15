use crate::resource::ResourceLabel;

use bevy_ecs::{
    prelude::Resource,
    system::{Res, ResMut, SystemParam}
};

use std::marker::PhantomData;


// Resource access parameters
#[derive(SystemParam)]
pub struct Read<L: ResourceLabel + 'static> {
    //res: Res<'w, ResourceIndex<L>>,
    _marker: PhantomData<L>
}

#[derive(SystemParam)]
pub struct Write<L: ResourceLabel + 'static> {
    //res: ResMut<'w, ResourceIndex<L>>,
    _marker: PhantomData<L>
}

#[derive(Resource)]
pub(crate) enum ResourceIndex<L: ResourceLabel> {
    Buffer { index: u32, label: PhantomData<L> },
    ImageView { index: u32, label: PhantomData<L> },
    AccelerationStructure { index: u32, label: PhantomData<L> }
}