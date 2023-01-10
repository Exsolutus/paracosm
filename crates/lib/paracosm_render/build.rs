use anyhow::{Result, Context};
use spirv_builder::*;

use std::{
    env,
    fs,
    path::{ Path, PathBuf }
};


fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=../rust_shaders");

    // While OUT_DIR is set for both build.rs and compiling the crate, PROFILE is only set in
    // build.rs. So, export it to crate compilation as well.
    let profile = env::var("PROFILE").unwrap();
    println!("cargo:rustc-env=PROFILE={}", profile);

    let compile_result = SpirvBuilder::new(Path::new("../rust_shaders"), "spirv-unknown-vulkan1.2")
        .print_metadata(MetadataPrintout::Full)
        .capability(Capability::RuntimeDescriptorArray)
        .extension("SPV_EXT_descriptor_indexing")
        .preserve_bindings(true)
        //.multimodule(true)
        .build()?;

    let shader_assets_dir = Path::new("../../../assets/shaders");
    fs::create_dir_all(&shader_assets_dir).context("Creating shader assets directory")?;

    match &compile_result.module {
        ModuleResult::MultiModule(modules) => {
            for (_, spv_file) in modules.iter() {
                process_module(spv_file)?;
            }
        },
        ModuleResult::SingleModule(module) => {
            process_module(module)?;
        }
    }

    Ok(())
}

fn process_module(spv_file: &PathBuf) -> Result<()> {
    let shader_assets_dir = Path::new("../../../assets/shaders");

    let file_name = spv_file.file_name().expect("SPIR-V module file name");
    let destination_path = shader_assets_dir.join(&file_name);

    // Move SPIR-V file to assets directory
    if spv_file.exists() {
        fs::rename(spv_file, &destination_path).with_context(|| {
            format!("Renaming {:?} to {:?}", spv_file, destination_path)
        })?;
    } else {
        assert!(destination_path.exists(), "rustc failed to generate SPIR-V module {:?}. Try touching the source files or running `cargo clean` on shaders.", destination_path);
    }

    Ok(())
}
