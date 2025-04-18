use std::{
    env,
    path::PathBuf,
    process::Command,
};

fn main() {
    println!("cargo:rerun-if-changed=hardware/gpio.h");
    println!("cargo:rerun-if-changed=hardware/gpio.c");

    let libdir_path = PathBuf::from("hardware")
        .canonicalize()
        .expect("cannot canonicalize path");

    // Get target architecture
    let target = env::var("TARGET").unwrap_or_else(|_| String::from("x86_64-unknown-linux-gnu"));

    // Use target-specific compiler if cross-compiling
    let compiler = if target.contains("aarch64") {
        env::var("CC_aarch64").unwrap_or_else(|_| String::from("aarch64-unknown-linux-gnu-gcc"))
    } else {
        String::from("gcc")
    };

    // Use target-specific archiver if cross-compiling
    let archiver = if target.contains("aarch64") {
        env::var("AR_aarch64").unwrap_or_else(|_| String::from("aarch64-unknown-linux-gnu-ar"))
    } else {
        String::from("ar")
    };

    println!("cargo:warning=compiler: {}", compiler);

    // compile
    let status = Command::new(&compiler)
        .current_dir(&libdir_path)
        .arg("-c")
        .arg("-o")
        .arg("gpio.o")
        .arg("gpio.c")
        .status()
        .unwrap_or_else(|_| panic!("Failed to execute {}", compiler));

    if !status.success() {
        panic!("Failed to compile gpio.c");
    }

    // static library
    let status = Command::new(&archiver)
        .current_dir(&libdir_path)
        .arg("rcs")
        .arg("libgpio.a")
        .arg("gpio.o")
        .status()
        .unwrap_or_else(|_| panic!("Failed to execute {}", archiver));

    if !status.success() {
        panic!("Failed to create static library");
    }

    println!("cargo:rustc-link-search={}", libdir_path.to_str().unwrap());
    println!("cargo:rustc-link-lib=gpio");
}
