{
  description = "Tlock dev shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    unstable.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      unstable,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        unstablePkgs = import unstable {
          inherit system;
        };

        rustToolchain = pkgs.rust-bin.stable."1.93.0".default.override {
          extensions = [
            "rust-src"
            "llvm-tools"
            "rust-analyzer"
          ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        rustfmtNightly = pkgs.rust-bin.nightly.latest.rustfmt;
      in
      {
        devShells = {
          default = pkgs.mkShell {
            packages = [
              # Rust toolchain and extensions
              rustToolchain
              rustfmtNightly

              # Dioxus CLI
              pkgs.dioxus-cli
              pkgs.wabt
              pkgs.wasm-tools
              pkgs.lld
              pkgs.pkg-config
              pkgs.gtk3
              pkgs.cairo
              pkgs.glib
              pkgs.webkitgtk_4_1
              pkgs.libsoup_3
              pkgs.xdotool
              pkgs.openssl
              pkgs.binaryen
              unstablePkgs.wasm-bindgen-cli_0_2_121
            ];

            env.WASM_BINDGEN = "${unstablePkgs.wasm-bindgen-cli_0_2_106}/bin/wasm-bindgen";
          };
        };
      }
    );
}
