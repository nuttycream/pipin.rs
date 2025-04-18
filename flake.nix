{
  description = "simple rust flake";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
  }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs {
      inherit system;
      overlays = [rust-overlay.overlays.default self.overlays.default];
    };

    aarch64-pkgs = import nixpkgs {
      inherit system;
      crossSystem = {
        config = "aarch64-unknown-linux-gnu";
      };
    };

    aarch64-cc = "${aarch64-pkgs.stdenv.cc}/bin/aarch64-unknown-linux-gnu-cc";
    aarch64-ar = "${aarch64-pkgs.stdenv.cc}/bin/aarch64-unknown-linux-gnu-ar";
  in {
    overlays.default = final: prev: {
      rustToolchain =
        prev.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
    };

    devShell.${system} = pkgs.mkShell {
      buildInputs = with pkgs; [
        rustToolchain
        pkg-config
        openssl
        cargo-watch
        systemfd
        qemu
        aarch64-pkgs.stdenv.cc
      ];

      CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = aarch64-cc;
      CC_aarch64 = aarch64-cc;
      AR_aarch64 = aarch64-ar;

      shellHook = ''
        export PATH=$PATH:$HOME/.cargo/bin
      '';
    };
  };
}
