{ pkgs ? import <nixpkgs> { } }:

with pkgs;
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
