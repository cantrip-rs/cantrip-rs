# The AMD (ROCm) training environment on a Nix host: `nix-shell ai/shell-rocm.nix`. PyTorch is the nixpkgs
# `torchWithRocm` build, so `torch.cuda` is HIP. The ROCm user space comes with the shell; only the host's amdgpu kernel
# driver is shared. Running the generation driver in a shell rather than a container keeps `cargo` and `dotnet` (its
# build and gate stages) on the host. docs/ai-training.md covers every way of setting up.
{ system ? builtins.currentSystem }:
let
  pkgs = import (builtins.getFlake "nixpkgs") { inherit system; };
in
pkgs.mkShell {
  # cargo, rustc and dotnet are the host's; leaving them out keeps this shell from shadowing the host's toolchain (and
  # the generation driver's build stage runs with the host's rustc).
  packages = [
    (pkgs.python3.withPackages (p: [ p.torchWithRocm p.onnx p.onnxruntime p.numpy p.zstandard ]))
  ];
  shellHook = ''
    echo "ROCm PyTorch: $(python -c 'import torch; print(torch.__version__, "HIP", torch.version.hip, "device", torch.cuda.is_available() and torch.cuda.get_device_name(0))')"
  '';
}
