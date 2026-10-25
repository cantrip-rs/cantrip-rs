# The NVIDIA (CUDA) training environment on a Nix host: `nix-shell ai/shell-cuda.nix`. PyTorch is nixpkgs' `torch-bin`,
# the prebuilt CUDA wheel (unfree, so the shell allows it), which needs only the host's NVIDIA kernel driver. The CPU
# shell is `shell.nix` and the AMD one `shell-rocm.nix`; docs/ai-training.md covers every way of setting up.
{ system ? builtins.currentSystem }:
let
  pkgs = import (builtins.getFlake "nixpkgs") {
    inherit system;
    config.allowUnfree = true;
  };
in
pkgs.mkShell {
  # Like shell-rocm.nix, this leaves cargo, rustc and dotnet to the host, so the generation driver's build stage uses
  # the host's toolchain.
  packages = [
    (pkgs.python3.withPackages (p: [ p.torch-bin p.onnx p.onnxruntime p.numpy p.zstandard ]))
  ];
  shellHook = ''
    echo "CUDA PyTorch: $(python -c 'import torch; print(torch.__version__, "CUDA", torch.version.cuda, "device", torch.cuda.is_available() and torch.cuda.get_device_name(0))')"
  '';
}
