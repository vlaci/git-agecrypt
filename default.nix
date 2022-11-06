{ lib, rustPlatform, pkg-config, libgit2, zlib, git }:

let
  cargo_toml = lib.importTOML ./Cargo.toml;
in
rustPlatform.buildRustPackage rec {
  pname = cargo_toml.package.name;
  version = cargo_toml.package.version;

  src = ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  nativeBuildInputs = [ pkg-config ];

  buildInputs = [ libgit2 zlib ];
  checkInputs = [ git ];
}
