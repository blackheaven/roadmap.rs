{
  description = "A basic Rust learning environment";

  inputs = {
    nixpkgs.url =
      "github:nixos/nixpkgs/nixos-unstable"; # We want to use packages from the binary cache
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs@{ self, nixpkgs, flake-utils, ... }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ inputs.rust-overlay.overlays.rust-overlay ];
        };

        rustPlatform = pkgs.makeRustPlatform {
          cargo = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
          rustc = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default);
        };
        rustlings = let
          rustlingsSrc = pkgs.fetchFromGitHub {
            owner = "rust-lang";
            repo = "rustlings";
            rev = "v6.0.1";
            hash = "sha256-2Z6KG640b6IUkL+YiXAl2Jj2/MV8MImTxzPaMrBeCNg=";
          };
        in rustPlatform.buildRustPackage rec {
          pname = "rustlings";
          version = "6.0.1";
          cargoLock.lockFile = "${rustlingsSrc}/Cargo.lock";
          src = pkgs.lib.cleanSource rustlingsSrc;
          # postPatch = "sed -i '1i\#![feature(result_option_inspect)]' src/main.rs";
          doCheck = false;
        };
      in rec {
        devShell = pkgs.mkShell {
          CARGO_INSTALL_ROOT = "${toString ./.}/.cargo";

          buildInputs = with pkgs; [
            # cargo
            # rustc
            # rust-analyzer
            git
            pkg-config
            openssl
            trunk
            cargo-binutils
            cargo-watch
            rustlings
            lld
            wasm-pack
            wasm-bindgen-cli
            (rust-bin.fromRustupToolchain {
              channel = "stable";
              components =
                [ "rust-analyzer" "rust-src" "rustfmt" "rustc" "cargo" ];
              targets = [ "wasm32-unknown-unknown" ];
            })
          ];
        };
      });
}
