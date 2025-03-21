{
  description = "A basic Rust flake";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }: let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in {
    devShell.${system} = pkgs.mkShell {
      buildInputs = with pkgs; [
        rustup
        cmake
        clang
      ];

      shellHook = ''
        # Avoid polluting the home directory
        export RUSTUP_HOME=$(pwd)/.rustup/
        export CARGO_HOME=$(pwd)/.cargo/

        # Use binaries installed with `cargo install`
        export PATH=$PATH:$CARGO_HOME/bin
        export LIBCLANG_PATH=${pkgs.libclang.lib}/lib

        # Install and display the current toolchain
        rustup show
      '';
    };
  };
}
