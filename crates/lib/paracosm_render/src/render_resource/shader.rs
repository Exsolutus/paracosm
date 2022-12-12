use anyhow::{Result, bail};

use ash::vk;
use bevy_asset::{AssetLoader, AssetPath, Handle, LoadContext, LoadedAsset};
use bevy_ecs::system::Resource;
use bevy_log::prelude::*;
use bevy_reflect::{TypeUuid, Uuid};
use bevy_utils::{tracing::error, BoxedFuture, HashMap};

use spirv_builder::{MetadataPrintout, SpirvBuilder};
use std::{
    borrow::Cow,
    boxed::Box,
    path::Path, ops::Deref
};


#[derive(Clone, Debug, Resource)]
pub struct ShaderHandle(pub Handle<Shader>);

impl Deref for ShaderHandle {
    type Target = Handle<Shader>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Debug, TypeUuid)]
#[uuid = "d95bc916-6c55-4de3-9622-37e7b6969fda"]
pub struct Shader {
    pub name: String,
    pub path: Cow<'static, Path>,
    pub module: Option<vk::ShaderModule>
}



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

            let shader = match ext {
                "rs" => compile_rust_shader(&load_context.path())?,
                "spv" => Shader { 
                    name: "".to_string(),
                    path: Cow::from(load_context.path().to_path_buf()),
                    module: None
                },
                _ => panic!("Unsupported shader extension: {ext}")
            };

            let asset = LoadedAsset::new(shader);

            load_context.set_default_asset(asset);
            Ok(())
        })
    }

    fn extensions(&self) -> &[&str] {
        &["rs", "spv"]
    }
}

fn compile_rust_shader(path: &Path) -> Result<Shader> {
    if !path.starts_with("rust_shaders/src") {
        bail!("Shader to compile must be in 'assets/rust_shaders/src'.")
    }

    let name = path.file_name().unwrap().to_str().unwrap();
    
    // Hack: spirv_builder builds into a custom directory if running under cargo, to not
    // deadlock, and the default target directory if not. However, packages like `proc-macro2`
    // have different configurations when being built here vs. when building
    // rustc_codegen_spirv normally, so we *want* to build into a separate target directory, to
    // not have to rebuild half the crate graph every time we run. So, pretend we're running
    // under cargo by setting these environment variables.
    std::env::set_var("OUT_DIR", env!("OUT_DIR"));
    std::env::set_var("PROFILE", env!("PROFILE"));

    // Send shader path to build.rs
    std::env::set_var("BUILD_SHADER", "test.rs");

    // Compile Rust to SpirV
    let compiled_shader = SpirvBuilder::new(Path::new("assets/rust_shaders"), "spirv-unknown-spv1.5")
        .print_metadata(MetadataPrintout::Full)
        .build()
        .unwrap();

    compiled_shader.entry_points.iter().for_each(|entry_point| {
        debug!("{}", entry_point);
    });
    
    let path = compiled_shader.module.unwrap_single().to_path_buf();

    Ok(Shader { 
        name: name.to_string(),
        path: Cow::from(path),
        module: None
    })
}