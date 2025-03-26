use std::{
    env,
    path::PathBuf,
    process::Command,
};

// no longer depends on bindgen
// will write bindings on my own
fn main() {
    println!("cargo:rerun-if-changed=hardware/gpio.h");
    println!("cargo:rerun-if-changed=hardware/gpio.c");

    let libdir_path = PathBuf::from("hardware")
        .canonicalize()
        .expect("cannot canonicalize path");

    let target = env::var("TARGET").unwrap_or_else(|_| String::from("x86_64-unknown-linux-gnu"));
    println!("cargo:warning=Building for target: {}", target);

    let compiler = if target == "aarch64-unknown-linux-musl" {
        env::var("CC_AARCH64_UNKNOWN_LINUX_MUSL")
            .unwrap_or_else(|_| String::from("aarch64-unknown-linux-musl-gcc"))
    } else {
        String::from("clang")
    };
    println!("cargo:warning=Using compiler: {}", compiler);

    let obj_path = libdir_path.join("gpio.o");
    let lib_path = libdir_path.join("libgpio.a");

    let status = Command::new(&compiler)
        .current_dir(&libdir_path)
        .arg("-c")
        .arg("-o")
        .arg(&obj_path)
        .arg("gpio.c")
        .status()
        .expect(&format!("Failed to execute {}", compiler));

    if !status.success() {
        panic!("Failed to compile gpio.c");
    }

    let archiver = if target == "aarch64-unknown-linux-musl" {
        env::var("AR_AARCH64_UNKNOWN_LINUX_MUSL")
            .unwrap_or_else(|_| String::from("aarch64-unknown-linux-musl-ar"))
    } else {
        String::from("ar")
    };
    println!("cargo:warning=archiver: {}", archiver);

    let status = Command::new(&archiver)
        .current_dir(&libdir_path)
        .arg("rcs")
        .arg(lib_path)
        .arg(obj_path)
        .status()
        .expect(&format!("Failed to execute {}", archiver));

    if !status.success() {
        panic!("Failed to create static library");
    }

    println!("cargo:rustc-link-search={}", libdir_path.to_str().unwrap());
    println!("cargo:rustc-link-lib=gpio");
}
