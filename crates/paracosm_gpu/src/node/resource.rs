use crate::resource::ResourceLabel;

use bevy_ecs::{
    prelude::Resource,
    system::{Res, ResMut, SystemParam}
};

use std::{marker::PhantomData, ops::Deref};


// Resource access parameters
#[derive(SystemParam)]
pub struct Read<'w, L: ResourceLabel + 'static> {
    res: Res<'w, ResourceIndex<L>>,
}

impl<'w, L: ResourceLabel + 'static> Deref for Read<'w, L> {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.res.index
    }
}


#[derive(SystemParam)]
pub struct Write<'w, L: ResourceLabel + 'static> {
    res: ResMut<'w, ResourceIndex<L>>,
}

impl<'w, L: ResourceLabel + 'static> Deref for Write<'w, L> {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.res.index
    }
}

#[derive(Resource)]
pub(crate) struct ResourceIndex<L: ResourceLabel> {
    pub index: u32,
    pub _marker: PhantomData<L>
}