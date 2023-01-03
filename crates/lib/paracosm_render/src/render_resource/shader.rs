
use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, Handle};
use bevy_ecs::system::Resource;
use bevy_reflect::{TypeUuid};
use bevy_utils::HashMap;

use paracosm_gpu::{
    resource::shader_module::ShaderModule
};

use std::{
    borrow::Cow,
};


#[derive(Clone, TypeUuid)]
#[uuid = "d95bc916-6c55-4de3-9622-37e7b6969fda"]
pub struct Shader {
    pub module: ShaderModule,
    pub entry_point: Cow<'static, str>
}

#[derive(Clone, Debug, Resource)]
pub struct ShaderManager {
    pub shaders: HashMap<String, Handle<Shader>>
}


pub struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Shader>()
            .add_debug_asset::<Shader>();

        app.insert_resource(ShaderManager {
            shaders: HashMap::new()
        });
    }
}
