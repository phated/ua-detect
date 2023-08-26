{
  # All of these inputs (a.k.a. dependencies) need to align with the other inputs
  # we use so they use the `inputs.*.follows` syntax to reference the others
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };

    crane = {
      url = "github:ipetkov/crane";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
        rust-overlay.follows = "rust-overlay";
      };
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlays.default
          ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          # We include rust-src to ensure rust-analyzer works.
          # See https://discourse.nixos.org/t/rust-src-not-found-and-other-misadventures-of-developing-rust-on-nixos/11570/4
          extensions = [ "rust-src" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # We use the `tonic::include_proto!` macro to embed our protobuf definitions so we need to create a special
        # source filter to include .proto files in addition to usual rust/cargo source files
        protoFilter = path: _type: builtins.match ".*proto$" path != null;
        sourceFilter = path: type:
          (protoFilter path type) || (craneLib.filterCargoSources path type);

        my-crate = craneLib.buildPackage {
          src = pkgs.lib.cleanSourceWith {
            src = craneLib.path ./.;
            filter = sourceFilter;
          };

          nativeBuildInputs = [
            pkgs.protobuf
          ];

          buildInputs = [
            # Add additional build inputs here
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];
        };
      in
      {
        checks = {
          inherit my-crate;
        };

        packages.default = my-crate;

        apps.default = flake-utils.lib.mkApp {
          drv = my-crate;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = builtins.attrValues self.checks.${system};

          # Extra inputs can be added here
          nativeBuildInputs = with pkgs; [
            nil
            nixpkgs-fmt
          ];
        };
      });
}
