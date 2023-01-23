use anyhow::{Result, bail};

use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetEvent, Assets, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::*,
    system::{StaticSystemParam, SystemParam, SystemParamItem},
};
use bevy_log::prelude::*;

use std::{
    collections::HashMap,
    marker::PhantomData
};



pub enum PrepareAssetError {
    RetryNextUpdate,
}

/// Describes how an asset gets prepared for rendering.
///
/// Before ___ step the asset is transformed into its 
/// GPU-representation of type [`RenderAsset::PreparedAsset`].
pub trait RenderAsset: Asset {
    type PreparedAsset: Send + Sync +'static;
    /// Specifies all ECS data required by [`RenderAsset::prepare_asset`].
    /// For convenience use the [`lifetimeless`](bevy_ecs::system::lifetimeless) [`SystemParam`].
    type Param: SystemParam;
    /// Prepares the `extracted asset` for the GPU by transforming it into
    /// a [`RenderAsset::PreparedAsset`]. Therefore ECS data may be accessed via the `param`.
    fn prepare_asset(
        source_asset: &Self,
        param: &mut SystemParamItem<Self::Param>,
    ) -> Result<Self::PreparedAsset, PrepareAssetError>;
}



#[derive(Clone, Hash, Debug, Default, PartialEq, Eq, SystemLabel)]
pub enum PrepareAssetLabel {
    PreAssetPrepare,
    #[default]
    AssetPrepare,
    PostAssetPrepare,
}

pub struct RenderAssetPlugin<A: RenderAsset> {
    prepare_asset_label: PrepareAssetLabel,
    phantom: PhantomData<fn() -> A>,
}

impl<A: RenderAsset> RenderAssetPlugin<A> {
    pub fn with_prepare_asset_label(prepare_asset_label: PrepareAssetLabel) -> Self {
        Self {
            prepare_asset_label,
            phantom: PhantomData,
        }
    }
}

impl<A: RenderAsset> Default for RenderAssetPlugin<A> {
    fn default() -> Self {
        Self {
            prepare_asset_label: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<A: RenderAsset> Plugin for RenderAssetPlugin<A> {
    fn build(&self, app: &mut App) {
        let prepare_asset_system = prepare_assets::<A>.label(self.prepare_asset_label.clone());

        let prepare_asset_system = match self.prepare_asset_label {
            PrepareAssetLabel::PreAssetPrepare => prepare_asset_system,
            PrepareAssetLabel::AssetPrepare => {
                prepare_asset_system.after(PrepareAssetLabel::PreAssetPrepare)
            }
            PrepareAssetLabel::PostAssetPrepare => {
                prepare_asset_system.after(PrepareAssetLabel::AssetPrepare)
            }
        };

        app.init_resource::<RenderAssets<A>>()
            .init_resource::<PrepareAssetsQueue<A>>()
            .add_system(prepare_asset_system.at_start());
    }
}


/// Stores all GPU representations ([`RenderAsset::PreparedAssets`](RenderAsset::PreparedAsset))
/// of [`RenderAssets`](RenderAsset) as long as they exist.
#[derive(Resource, Deref, DerefMut)]
pub struct RenderAssets<A: RenderAsset>(HashMap<Handle<A>, A::PreparedAsset>);

impl<A: RenderAsset> Default for RenderAssets<A> {
    fn default() -> Self {
        Self(Default::default())
    }
}



// TODO: consider storing inside system?
/// All assets that should be prepared next frame.
#[derive(Resource)]
pub struct PrepareAssetsQueue<A: RenderAsset> {
    assets: Vec<Handle<A>>,
}

impl<A: RenderAsset> Default for PrepareAssetsQueue<A> {
    fn default() -> Self {
        Self {
            assets: Default::default(),
        }
    }
}



fn prepare_assets<A: RenderAsset>(
    mut render_assets: ResMut<RenderAssets<A>>,
    mut prepare_queue: ResMut<PrepareAssetsQueue<A>>,
    mut events: EventReader<AssetEvent<A>>,
    assets: Res<Assets<A>>,
    param: StaticSystemParam<<A as RenderAsset>::Param>,
) {
    for event in events.iter() {
        match event {
            AssetEvent::Created { handle } |
            AssetEvent::Modified { handle } => {
                info!("Mesh Asset created or modified!");
                prepare_queue.assets.push(handle.clone_weak());
            },
            AssetEvent::Removed { handle } => {
                render_assets.remove(&handle);
            }
        }
    }

    let mut param = param.into_inner();
    let queued_assets = std::mem::take(&mut prepare_queue.assets);
    for handle in queued_assets {
        let asset = assets.get(&handle)
            .expect("Failed to get asset from handle");
        match A::prepare_asset(asset, &mut param) {
            Ok(prepared_asset) => {
                render_assets.insert(handle, prepared_asset);
            },
            Err(PrepareAssetError::RetryNextUpdate) => {
                error!("PrepareAssetError");
                prepare_queue.assets.push(handle);
            }
        }
    }
}