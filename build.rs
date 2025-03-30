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
    println!("cargo:warning=target: {}", target);

    let (compiler, archiver) = if target == "aarch64-unknown-linux-musl" {
        let compiler = env::var("CC_AARCH64_UNKNOWN_LINUX_MUSL")
            .unwrap_or_else(|_| String::from("aarch64-unknown-linux-musl-gcc"));
        let archiver = env::var("AR_AARCH64_UNKNOWN_LINUX_MUSL")
            .unwrap_or_else(|_| String::from("aarch64-unknown-linux-musl-ar"));
        (compiler, archiver)
    } else {
        (String::from("clang"), String::from("ar"))
    };

    println!("cargo:warning=compiler: {}", compiler);

    let target_suffix = target.replace('-', "_");
    let obj_path = libdir_path.join(format!("gpio_{}.o", target_suffix));
    let lib_path = libdir_path.join(format!("libgpio_{}.a", target_suffix));

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

    println!("cargo:warning=archiver: {}", archiver);

    let status = Command::new(&archiver)
        .current_dir(&libdir_path)
        .arg("rcs")
        .arg(&lib_path)
        .arg(&obj_path)
        .status()
        .expect(&format!("failed to execute {}", archiver));

    if !status.success() {
        panic!("failed to create static library");
    }

    println!("cargo:rustc-link-search={}", libdir_path.to_str().unwrap());
    println!("cargo:rustc-link-lib=gpio_{}", target_suffix);
}
