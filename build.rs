use std::path::PathBuf;
use std::{env, panic};

fn main() {
    println!("cargo:rerun-if-changed=hardware/gpio.h");

    let libdir_path = PathBuf::from("hardware")
        .canonicalize()
        .expect("cannot canonicalize path");

    let headers_path = libdir_path.join("gpio.h");
    let headers_path_str = headers_path.to_str().expect("Path is not a valid string");

    let obj_path = libdir_path.join("gpio.o");
    let lib_path = libdir_path.join("libgpio.a");

    println!("cargo:rustc-link-search={}", libdir_path.to_str().unwrap());

    println!("cargo:rustc-link-lib=gpio");

    if !std::process::Command::new("clang")
        .arg("-c")
        .arg("-o")
        .arg(&obj_path)
        .arg(libdir_path.join("gpio.c"))
        .output()
        .expect("could not spawn 'clang'")
        .status
        .success()
    {
        panic!("could not compile object file");
    }

    if !std::process::Command::new("ar")
        .arg("rcs")
        .arg(lib_path)
        .arg(obj_path)
        .output()
        .expect("could not spawn 'ar'")
        .status
        .success()
    {
        panic!("could not emit library path");
    }

    let bindings = bindgen::Builder::default()
        .header(headers_path_str)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs");

    bindings
        .write_to_file(out_path)
        .expect("couldnt write bindings");
}
