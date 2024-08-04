{
  description = "NixBlitz dev env";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
      };
    in {
      devShell = with pkgs;
        mkShell {
          buildInputs = [
            nixd
            alejandra # nix formatter
            cargo # rust package manager
            rust-analyzer
            vscode-extensions.vadimcn.vscode-lldb.adapter # for rust debugging
            rustc # rust compiler
            rustfmt
            rustPackages.clippy # rust linter
          ];
          RUST_SRC_PATH = rustPlatform.rustLibSrc;
        };
    });
}
