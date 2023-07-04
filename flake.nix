{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils, ... }:
    {
      overlay = final: prev: {
        git-agecrypt = final.callPackage ./default.nix {};
      };
    } //
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ self.overlay ];
        };
      in
      {
        packages.default = pkgs.git-agecrypt;
        devShells.default = import ./shell.nix { inherit pkgs; };
      });
}
