
use bevy_asset::{AssetLoader, AssetPath, Handle, LoadContext, LoadedAsset};
use bevy_reflect::{TypeUuid, Uuid};
use bevy_utils::{tracing::error, BoxedFuture, HashMap};



#[derive(Default)]
pub struct ShaderLoader;

impl AssetLoader for ShaderLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
        Box::pin(async move {
            let ext = load_context.path().extension().unwrap().to_str().unwrap();

            let mut shader = match ext {
                "spv" => Shader { source: Source::SprirV(Vec::from(bytes)) },
                _ => panic!("Unsupported shader extension: {ext}")
            };


        })
    }
}