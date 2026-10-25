# The CPU environment for ai/ on a Nix host: `nix-shell ai/shell.nix`. The GPU environments are `shell-cuda.nix` and
# `shell-rocm.nix`, or the containers (Dockerfile.cuda, Dockerfile.rocm); docs/ai-training.md covers every way of
# setting up.
{ pkgs ? import <nixpkgs> { } }:
pkgs.mkShell {
  packages = [
    (pkgs.python3.withPackages (p: [ p.torch p.onnx p.onnxruntime p.numpy p.zstandard ]))
    pkgs.nodejs
    pkgs.cargo
    pkgs.rustc
    pkgs.gcc
    pkgs.pkg-config
  ];
}
