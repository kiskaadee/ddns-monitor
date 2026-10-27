{
  description = "ddns-monitor — provider-agnostic DDNS monitor with plugin architecture";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    # Pinned Rust toolchain — gives us stable + clippy + rustfmt + rust-analyzer
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, ... }:
    let
      system = "x86_64-linux";

      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rust-overlay.overlays.default ];
      };

      # Pin to stable Rust. Edit the date to roll forward.
      # rustc, cargo, clippy, rustfmt, and rust-analyzer are all included.
      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
      };

      # ureq bundles rustls — no system OpenSSL or pkg-config needed
      nativeBuildInputs = with pkgs; [ rustToolchain ];
      buildInputs = [ ];

    in
    {
      # ── Development shell ─────────────────────────────────────────────────────
      # Enter with: nix develop
      devShells.${system}.default = pkgs.mkShell {
        inherit nativeBuildInputs buildInputs;

        packages = with pkgs; [
          # Linting & formatting (also available via rustToolchain, listed
          # here for clarity)
          cargo-watch      # cargo watch -x check -- live recompile on save
          cargo-nextest    # faster test runner (drop-in for cargo test)

          # Dependency auditing
          cargo-audit

          # Useful during DDNS API development
          curl
          jq
        ];

        shellHook = ''
          echo ""
          echo "  ddns-monitor dev shell"
          echo "  ─────────────────────────────────────────────"
          echo "  rustc   $(rustc --version)"
          echo "  cargo   $(cargo --version)"
          echo ""
          echo "  Useful commands:"
          echo "    cargo build          compile the binary"
          echo "    cargo test           run unit tests"
          echo "    cargo nextest run    run tests with nextest"
          echo "    cargo watch -x check live check on save"
          echo "    cargo audit          scan dependencies for CVEs"
          echo "    cargo clippy         lint"
          echo "    cargo fmt            format"
          echo ""
        '';
      };

      # ── Package ───────────────────────────────────────────────────────────────
      # Build the binary: nix build
      packages.${system} = rec {
        ddns-monitor = pkgs.rustPlatform.buildRustPackage {
          pname = "ddns-monitor";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          inherit nativeBuildInputs buildInputs;

          meta = with pkgs.lib; {
            description = "Provider-agnostic DDNS monitor with plugin architecture";
            license = licenses.unlicense;
            maintainers = [ ];
          };
        };

        default = ddns-monitor;
      };

      # ── NixOS Module ──────────────────────────────────────────────────────────
      nixosModules = rec {
        ddns-monitor = import ./nixos/module.nix;
        default = ddns-monitor;
      };

      # ── Checks ────────────────────────────────────────────────────────────────
      # Run with: nix flake check
      checks.${system} = {
        # cargo test inside a sandbox — proves tests pass without network/fs
        cargo-test = pkgs.rustPlatform.buildRustPackage {
          pname = "ddns-monitor-tests";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          inherit nativeBuildInputs buildInputs;
          doCheck = true;
        };
      };
    };
}
