{
  description = "Nix Devshell for Spin";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";

    fenix = {
      url = "github:nix-community/fenix?rev=6f2fec850f569d61562d3a47dc263f19e9c7d825";
      #inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    flake-utils,
    fenix,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustTarget = pkgs.rust-bin.stable.latest.default.override {
          extensions = ["rust-src" "rust-analyzer"];
          targets = ["wasm32-wasi" "wasm32-unknown-unknown"];
        };

        RustToolchain = with fenix.packages.${pkgs.system};
        with latest;
          combine [
            cargo
            clippy
            rust-analyzer
            rust-src
            rustc
            rustfmt
            targets.wasm32-wasi.latest.rust-std
          ];
      in
        with pkgs; {
          devShells.default = mkShellNoCC {
            buildInputs =
              [
                openssl
                pkg-config
                rustTarget
              ]
              ++ lib.optionals stdenv.isDarwin [
                darwin.apple_sdk.frameworks.Accelerate
              ];

            packages = [
              RustToolchain
            ];

            shellHook = ''
              export LD_LIBRARY_PATH=${pkgs.lib.makeLibraryPath [
                pkgs.stdenv.cc.cc
                openssl
              ]}
            '';

            RUST_SRC_PATH = "${rustTarget}/lib/rustlib/src/rust/library";
          };
        }
    );
}
