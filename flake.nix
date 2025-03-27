{
  description = "A basic Rust flake";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { 
      inherit system; 
      overlays = [ rust-overlay.overlays.default self.overlays.default ];
    };
  in {
    overlays.default = final: prev: {
      rustToolchain =
        let
          rust = prev.rust-bin;
        in
        if builtins.pathExists ./rust-toolchain.toml then
          rust.fromRustupToolchainFile ./rust-toolchain.toml
        else if builtins.pathExists ./rust-toolchain then
          rust.fromRustupToolchainFile ./rust-toolchain
        else
          rust.stable.latest.default.override {
            extensions = [ "rust-src" "rust-analyzer" "rustfmt" ];
          };
    };

    devShell.${system} = 
      let 
        targetName = {
          musl = "aarch64-unknown-linux-musl";
        };

        pkgsCross = builtins.mapAttrs (name: value: import pkgs.path {
          system = system;
          crossSystem = {
            config = value;
          };
        }) targetName;

        ccPkgs = builtins.mapAttrs (name: value: value.stdenv.cc) pkgsCross;
        cc = builtins.mapAttrs (name: value: "${value}/bin/${targetName.${name}}-cc") ccPkgs;

      in pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            rust-analyzer
            cmake
            clang
            libclang
            pkg-config
            openssl
            cargo-deny
            cargo-edit
            cargo-watch
          ] ++ builtins.attrValues ccPkgs;

          CARGO_BUILD_TARGET = let 
            toolchainStr = builtins.readFile ./rust-toolchain.toml;
            targets = (builtins.fromTOML toolchainStr).toolchain.targets;
          in builtins.head targets;

          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GCC_LINKER = cc.musl;
          CC_AARCH64_UNKNOWN_LINUX_MUSL = cc.musl;
          LIBCLANG_PATH="${pkgs.libclang.lib}/lib";
          RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            export PATH=$PATH:$HOME/.cargo/bin
          '';
        };
  };
}
