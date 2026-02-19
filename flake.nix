{
  description = "A Rust project built with Crane";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    flake-utils.url = "github:numtide/flake-utils";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    # aspen-automerge and its transitive crate deps
    aspen-src = {
      url = "github:brittonr/aspen";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, flake-utils, rust-overlay, aspen-src, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Filter aspen to only cargo-relevant files (.rs, Cargo.toml, Cargo.lock)
        aspenFiltered = pkgs.lib.cleanSourceWith {
          src = aspen-src;
          filter = path: type: craneLib.filterCargoSources path type;
        };

        # Common arguments for crane builds
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;
          strictDeps = true;

          # Cargo.toml references ../aspen/crates/aspen-automerge as a path dep.
          # In the Nix sandbox the source unpacks to /build/source/, so ../aspen
          # resolves to /build/aspen/. postUnpack runs in /build/ before cd to
          # sourceRoot, so this places it exactly where cargo expects.
          postUnpack = ''
            cp -r ${aspenFiltered} aspen
            chmod -R u+w aspen
          '';

          buildInputs = [
            # Add additional build inputs here
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
            pkgs.darwin.apple_sdk.frameworks.Security
          ];

          nativeBuildInputs = [
            pkgs.pkg-config
          ];
        };

        # Build only the cargo dependencies for caching
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the actual package (integration tests under tests/ need real
        # P2P networking unavailable in the Nix sandbox — run via cargo test)
        package = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoTestExtraArgs = "--lib --bins";
        });
      in
      {
        checks = {
          inherit package;

          clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          fmt = craneLib.cargoFmt {
            src = craneLib.cleanCargoSource ./.;
          };

          test = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });
        };

        packages = {
          default = package;
        };

        apps.default = {
          type = "app";
          program = "${package}/bin/irohscii";
        };

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};

          packages = with pkgs; [
            rustToolchain
            cargo-watch
            cargo-nextest
          ];
        };
      }
    );
}
