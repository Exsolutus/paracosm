use spirv_builder::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() == 2, "Provide a single path argument.");

    let path = std::path::PathBuf::from(&args[1]);
    assert!(path.is_dir(), "Path argument must be a valid directory.");
    
    let builder = SpirvBuilder::new(path, "spirv-unknown-vulkan1.2")
        .print_metadata(MetadataPrintout::Full)
        .capability(Capability::RuntimeDescriptorArray)
        .capability(Capability::StorageImageReadWithoutFormat)
        .capability(Capability::StorageImageWriteWithoutFormat)
        .extension("SPV_EXT_descriptor_indexing")
        .preserve_bindings(true);

    let compile_result = match builder.build() {
        Ok(result) => result,
        Err(error) => panic!("{}", error)
    };

    let module_path = compile_result.module.unwrap_single();
    println!("{}", module_path.as_os_str().to_str().unwrap());
}