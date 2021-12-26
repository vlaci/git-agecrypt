{ pkgs ? import <nixpkgs> { } }:

with pkgs; mkShell {
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [
    openssl.dev
    clang
    lldb
  ];
}
