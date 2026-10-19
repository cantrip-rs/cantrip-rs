# The CPU environment for ai/ on the NixOS host: `nix-shell ai/shell.nix`. The GPU environments are the containers
# (Dockerfile.cuda, Dockerfile.rocm).
{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  packages = [
    (pkgs.python3.withPackages (p: [ p.torch p.onnx p.onnxruntime p.numpy ]))
    pkgs.nodejs
    pkgs.cargo
    pkgs.rustc
    pkgs.gcc
    pkgs.pkg-config
  ];
}
