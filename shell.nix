{
  pkgs ? import <nixpkgs> { },
}:

with pkgs;
mkShell {
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [
    cargo
    cargo-limit
    cargo-watch
    clang
    clippy
    gdb
    grcov
    just
    lldb
    openssl.dev
    rustc
    rustfmt
  ];
}
