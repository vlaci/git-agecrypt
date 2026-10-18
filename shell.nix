{
  pkgs ? import <nixpkgs> { },
}:

with pkgs;
mkShell {
  nativeBuildInputs = [ pkg-config ];
  buildInputs = [
    cargo-limit
    cargo-watch
    clang
    gdb
    grcov
    just
    lldb
    openssl.dev
  ];
}
