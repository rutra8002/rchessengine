{
  description = "Simple material chess engine, cross-compiled to Windows";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay }:
    let
      system = "x86_64-linux";

      pkgs = import nixpkgs {
        inherit system;
        overlays = [
          rust-overlay.overlays.default
        ];
      };

      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
      inherit (cargoToml.package) name version;
      author = builtins.head cargoToml.package.authors;

      mingw = pkgs.pkgsCross.mingwW64;

      rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        targets = [
          "x86_64-pc-windows-gnu"
        ];
      };
    in
    {
      packages.${system} = {
        default = pkgs.rustPlatform.buildRustPackage {
          pname = name;
          inherit version;

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          meta = {
            inherit author;
          };
        };

        windows = pkgs.rustPlatform.buildRustPackage.override {
          rustc = rustToolchain;
          cargo = rustToolchain;
        } {
          pname = name;
          inherit version;

          src = ./.;

          cargoLock.lockFile = ./Cargo.lock;

          nativeBuildInputs = [
            mingw.stdenv.cc
          ];

          buildInputs = [
            mingw.windows.pthreads
          ];

          cargoBuildFlags = [
            "--target"
            "x86_64-pc-windows-gnu"
          ];

          CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER =
            "${mingw.stdenv.cc.targetPrefix}cc";

          doCheck = false;

          installPhase = ''
            mkdir -p $out/bin

            cp target/x86_64-pc-windows-gnu/release/${name}.exe \
              $out/bin/${name}.exe
          '';

          meta = {
            inherit author;
          };
        };
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = [
          rustToolchain
          mingw.stdenv.cc
        ];
      };
    };
}