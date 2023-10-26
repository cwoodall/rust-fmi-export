extern crate cargo;
extern crate zip;
extern crate walkdir;

use zip::write::ZipWriter;
use std::env;
use cargo::core::{Workspace};
use std::io::prelude::*;

use std::fs::{create_dir_all, remove_dir};
use std::fs;
use std::fs::File;
use std::ffi::OsStr;

use std::path::{Path, PathBuf};
use cargo::{Config};
use cargo::util::{important_paths, CargoResult};

#[macro_use]
extern crate dlopen_derive;
use dlopen::wrapper::{Container, WrapperApi};


#[derive(WrapperApi)]
struct PluginApi {
    get_model_description: extern fn() -> String,
    get_model_name: extern fn() -> &'static str,
}

fn main() {
    let package_name = env!("CARGO_PKG_NAME");

    let version = format!(
		"{}.{}.{}{}",
		env!("CARGO_PKG_VERSION_MAJOR"),
		env!("CARGO_PKG_VERSION_MINOR"),
		env!("CARGO_PKG_VERSION_PATCH"),
		option_env!("CARGO_PKG_VERSION_PRE").unwrap_or("")
    );
    
    println!("The current {} package version is {}",package_name, version);

    // using cargo-bitbake as a reference https://github.com/meta-rust/cargo-bitbake/blob/master/src/main.rs
    let config = Config::default().unwrap();
    let manifest_path = config.cwd().to_path_buf();
    let root = important_paths::find_root_manifest_for_wd(&manifest_path).unwrap();
    println!("The current manifest path is {:?}", manifest_path);
    let ws = Workspace::new(&root, &config).unwrap();

    println!("The current workspace is {:?}", ws.current().unwrap().library().unwrap().binary_filename());

    let res = cargo::ops::compile(&ws, &cargo::ops::CompileOptions::new(&config, cargo::core::compiler::CompileMode::Build).unwrap()).unwrap();

    // TODO: fix to deal with multiple outputs. This assumes 1 dylib
    let dylib_path = res.cdylibs.first().unwrap().path.clone();
    println!("{:?}", dylib_path);
    
    // Now lets load the plugin and use the get_model_description call to get the xml description of the module

    // See https://github.com/zicklag/rust-tutorials/blob/master/book/rust-plugins.md
    let plugin_api_wrapper: Container<PluginApi> = unsafe { Container::load(&dylib_path) }.unwrap();
    let model_description_xml = plugin_api_wrapper.get_model_description();
    
    let model_name = plugin_api_wrapper.get_model_name();
    let path = Path::new(&manifest_path);
    let path = path.join("target").join("fmu").join(format!("{}.fmu", model_name));
    println!("The current path is {:?}", path);
    
    match (create_dir_all(path.to_str().unwrap())) {
        Ok(_) => {
            println!("Created directory")
        },
        Err(_) => {
            remove_dir(path.to_str().unwrap()).expect("Could not remove directory");
            create_dir_all(path.to_str().unwrap()).expect("Could not create directory");
            println!("Directory already exists")
        },
    }

    // Need a better way to get the target path in here... hardcoding to darwin64 for now.

    let bin_dir = path.join("binaries").join("darwin64");
    let bin_dir_str = bin_dir.to_str().unwrap();
    create_dir_all(bin_dir_str).expect("Could not create directory binaries");

    let dylib_file_ext = Path::new(&dylib_path).extension().and_then(OsStr::to_str).unwrap();
    std::fs::copy(&dylib_path, bin_dir.join(format!("{}.{}", model_name, dylib_file_ext))).expect("Could not copy modelDescription.xml");

    let model_description_file = path.join("modelDescription.xml");
    let model_description_file = model_description_file.to_str().unwrap();
    let mut file = File::create(&model_description_file).unwrap();
    file.write_all(model_description_xml.as_bytes()).expect("Could not write modelDescription.xml");


    // Now lets zip up the fmu
    let mut zip_writer = ZipWriter::new(File::create(format!("{}.zip", model_name)).unwrap());
    let options = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);

    zip_writer.start_file("modelDescription.xml", options).unwrap();
    zip_writer.write(model_description_xml.as_str().as_bytes()).unwrap();
    zip_writer.add_directory("binaries/darwin64", options).unwrap();
    let bin_filename = format!("{}.{}", model_name, dylib_file_ext);
    let bin_file = std::fs::read(dylib_path).unwrap();
    println!("{}", bin_filename);

    // let x = std::str::from_utf8(&bin_file).unwrap();
    zip_writer.start_file(format!("binaries/darwin64/{}", bin_filename), options).unwrap();
    zip_writer.write(bin_file.as_slice()).unwrap();

}   
