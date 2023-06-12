use std::process::Command;
use std::env;
use std::path::Path;

use std::fs::File;
use std::io::prelude::*;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let target = env::var("TARGET").unwrap();

    println!("cargo:warning=OUT_DIR: {}", out_dir);
    let mut file = File::create(out_dir + "/hello.txt").expect("Error encountered while creating file!");
    file.write_all(b"Hello, world!").expect("Error encountered while writing to file!");
}
