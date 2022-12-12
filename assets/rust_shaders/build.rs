use std::{env, path::Path, fs};
use regex::Regex;

fn main() {
    select_shader();
}

fn select_shader() {
    // Get shader to compile from environment variable
    let shader_env = match env::var("BUILD_SHADER") {
        Ok(value) => value,
        Err(_) => return println!("BUILD_SHADER environment variable not valid")
    };
    let shader_file = Path::new(&shader_env);
    let name = shader_file.file_stem().unwrap().to_str().unwrap();
    let ext = Path::new(&shader_file).extension().unwrap().to_str().unwrap();
    debug_assert_eq!(ext, "rs");

    // Get Cargo.toml content to edit
    let root = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("test: {}", root);
    let cargo_toml = Path::new(&root.as_str()).join("Cargo.toml");
    println!("test: {}", cargo_toml.clone().display());
    let content = fs::read_to_string(&cargo_toml)
        .expect("Something went wrong reading the Cargo.toml file.");
    
    // Edit Cargo.toml content with new shader name and shader path
    let name_regex = Regex::new(r#"(\bname = "\b([\w,\s-]+\b)"( # Shader name\b))"#).unwrap();
    let path_regex = Regex::new(r#"(\bpath = "src/\b([\w,\s-]+\b\.rs)"( # Shader path\b))"#).unwrap();
    debug_assert!(name_regex.is_match(&content));
    debug_assert!(path_regex.is_match(&content));

    let with_name = name_regex.replace(&content, format!("name = \"{}\" # Shader name", name));
    let with_path = path_regex.replace(&with_name, format!("path = \"src/{}\" # Shader path", shader_file.display()));

    println!("test: {}", with_path.clone());

    // Write changes to Cargo.toml file
    fs::write(cargo_toml, with_path.as_bytes())
        .expect("Something went wrong writing the Cargo.toml file.");
}
