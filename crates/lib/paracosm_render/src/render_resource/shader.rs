use paracosm_gpu::{
    device::Device,
};

use anyhow::{Result, bail};
use ash::vk;
use bevy_app::{App, AppExit, Plugin, CoreStage};
use bevy_asset::{AddAsset, AssetEvent, AssetLoader, Assets, AssetServer, Handle, LoadContext, LoadedAsset};
use bevy_ecs::{prelude::*, schedule::ShouldRun, system::Resource};
use bevy_log::prelude::*;
use bevy_reflect::{TypeUuid};
use bevy_utils::{BoxedFuture, HashSet};

use spirv_builder::{MetadataPrintout, SpirvBuilder};
use std::{
    borrow::Cow,
    boxed::Box,
    path::Path
};


#[derive(Clone, Debug, Eq, Hash, PartialEq, TypeUuid)]
#[uuid = "d95bc916-6c55-4de3-9622-37e7b6969fda"]
pub struct Shader {
    pub name: String,
    path: Cow<'static, Path>,
    pub module: Option<vk::ShaderModule>
}

#[derive(Clone, Debug, Resource)]
pub struct ShaderManager {
    pub shaders: HashSet<Handle<Shader>>
}


pub struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<Shader>()
            .add_debug_asset::<Shader>()
            .init_asset_loader::<ShaderLoader>()
            .init_debug_asset_loader::<ShaderLoader>();

        app.add_system(
            process_shader_events
                .at_end()
                .with_run_criteria(run_if_shader_events)
        );

        app.add_system_to_stage(
            CoreStage::Last, 
            on_exit
                .with_run_criteria(run_on_exit)
        );

        // Load shader assets
        let asset_server = app.world.get_resource::<AssetServer>().unwrap();
        let assets = match asset_server.load_folder("rust_shaders/src/") {
            Ok(result) => result,
            Err(error) => panic!("Failed to load shaders: {}", error.to_string())
        };
        let shaders: HashSet<Handle<Shader>> = assets.iter().map(|handle| {
            handle.clone().typed::<Shader>()
        })
        .collect();
        app.insert_resource(ShaderManager {
            shaders
        });
    }
}


/// An [`AssetLoader`] for shaders. Supports loading precompiled SpirV and runtime compiling from Rust.
#[derive(Default)]
pub struct ShaderLoader;

impl AssetLoader for ShaderLoader {
    fn load<'a>(
        &'a self,
        _bytes: &'a [u8],
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
        bail!("Target shader must be in 'assets/rust_shaders/src'.")
    }
    let path = path.strip_prefix("rust_shaders/src").unwrap();

    let name = path.file_name().unwrap().to_str().unwrap();
    
    // Hack: spirv_builder builds into a custom directory if running under cargo, to not
    // deadlock, and the default target directory if not. However, packages like `proc-macro2`
    // have different configurations when being built here vs. when building
    // rustc_codegen_spirv normally, so we *want* to build into a separate target directory, to
    // not have to rebuild half the crate graph every time we run. So, pretend we're running
    // under cargo by setting these environment variables.
    std::env::set_var("OUT_DIR", env!("OUT_DIR"));
    std::env::set_var("PROFILE", env!("PROFILE"));

    // Send asset path of target shader to build.rs
    std::env::set_var("TARGET_SHADER", path.display().to_string());

    // Compile Rust to SpirV
    let compiled_shader = SpirvBuilder::new(Path::new("assets/rust_shaders"), "spirv-unknown-vulkan1.2")
        .print_metadata(MetadataPrintout::None)
        .build()
        .unwrap();

    compiled_shader.entry_points.iter().for_each(|entry_point| {
        debug!("{}", entry_point);
    });
    
    let spv_path = compiled_shader.module.unwrap_single().to_path_buf();

    Ok(Shader { 
        name: name.to_string(),
        path: Cow::from(spv_path),
        module: None
    })
}


pub fn run_if_shader_events(
    ev_asset: EventReader<AssetEvent<Shader>>,
) -> ShouldRun {
    match ev_asset.is_empty() {
        true => ShouldRun::No,
        false => ShouldRun::Yes
    }
}

/// Event processing system to respond to AppExit events and Shader asset events
pub fn process_shader_events(
    mut ev_asset: EventReader<AssetEvent<Shader>>,
    mut assets: ResMut<Assets<Shader>>,
    device: Res<Device>,
) {
    // Process Shader asset events
    for ev in ev_asset.iter() {
        match ev {
            AssetEvent::Created { handle } => {
                debug!("Finalize loaded shader");
                let shader = assets.get_mut(handle).unwrap();
                let module = match device.create_shader_module(&shader.path) {
                    Ok(result) => result,
                    Err(error) => panic!("Failed to create shader module: {}", error.to_string())
                };
                shader.module = Some(module);
            },
            AssetEvent::Removed { handle } => {
                debug!("Cleanup unloaded shader");
                let shader = assets.get(handle).unwrap();
                match shader.module {
                    Some(value) => unsafe {
                        debug!("Destroying shader module");
                        device.destroy_shader_module(value, None);
                    },
                    None => ()
                };
            }
            _ => ()
        }
    }
}

fn run_on_exit(
    ev_exit: EventReader<AppExit>,
) -> ShouldRun {
    match ev_exit.is_empty() {
        true => ShouldRun::No,
        false => ShouldRun::Yes
    }
}

fn on_exit(
    ev_exit: EventReader<AppExit>,
    mut assets: ResMut<Assets<Shader>>,
    device: Res<Device>,
) {
    // Clean up shader modules on AppExit
    if !ev_exit.is_empty() {
        debug!("AppExit event");
        assets.iter_mut().for_each(|(_, shader)| {
            match shader.module.take() {
                Some(value) => unsafe {
                    device.destroy_shader_module(value, None);
                },
                None => ()
            }
        });
        return
    }
}
