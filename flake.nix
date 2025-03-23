{
  description = "A basic Rust flake";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in {
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
            rustup
            cmake
            clang
            libclang
          ] ++ builtins.attrValues ccPkgs;

          CARGO_BUILD_TARGET = let 
            toolchainStr = builtins.readFile ./rust-toolchain.toml;
            targets = (builtins.fromTOML toolchainStr).toolchain.targets;
          in builtins.head targets;

          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GCC_LINKER = cc.musl;
          CC_AARCH64_UNKNOWN_LINUX_MUSL = cc.musl;
          LIBCLANG_PATH="${pkgs.libclang.lib}/lib";

          shellHook = ''
          export RUSTUP_HOME=$(pwd)/.rustup/
          export CARGO_HOME=$(pwd)/.cargo/

          export PATH=$PATH:$CARGO_HOME/bin

          rustup show
          '';
        };
  };
}
