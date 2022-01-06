{ pkgs ? import <nixpkgs> { } }:

with pkgs;
let
  grcov = rustPlatform.buildRustPackage rec {
    pname = "grcov";
    version = "v0.8.2";

    doCheck = false;

    src = fetchFromGitHub {
      owner = "mozilla";
      repo = pname;
      rev = version;
      sha256 = "t1Gj5u4MmXPbQ5jmO9Sstn7aXJ6Ge+AnsmmG2GiAGKE=";
    };

    cargoSha256 = "DRAUeDzNUMg0AGrqU1TdrqBZJw4A2o3YJB0MdwwzefQ=";
  };
in
mkShell {
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [
    openssl.dev
    clang
    gdb
    lldb
    just
    grcov
    cargo-limit
    cargo-watch
  ];
}
