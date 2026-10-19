"""Exports the spike model to ONNX, one file per token bucket with a free batch dimension, checks the graph against
the operator allow-list, and writes the reference positions and outputs every other backend checks parity against.

    python export.py [--out DIR]

Writes DIR/spike-t{T}.onnx, DIR/ref-t{T}/{features.f32,kind.i64,present.u8,option.u8,value.f32,policy.f32} (raw
little-endian, row-major) and DIR/manifest.json.
"""

import argparse
import warnings
import json
import pathlib

import numpy as np
import onnx
import onnxruntime as ort
import torch

import model as spike

OPSET = 17
TOKENS = (32, 64, 128)
BATCHES = (1, 16, 64, 512)
REFERENCE = 512
ALLOWED = {
    "Add", "Cast", "Clip", "Constant", "Div", "Erf", "Gather", "Gemm", "Identity", "LayerNormalization", "MatMul",
    "Mul", "ReduceSum", "Reshape", "Softmax", "Split", "Squeeze", "Sub", "Tanh", "Transpose", "Unsqueeze", "Where",
}
INPUTS = ("features", "kind", "present", "option")
OUTPUTS = ("value", "policy")


def export(net, tokens, path):
    net.tokens = tokens
    warnings.filterwarnings("ignore", category=DeprecationWarning, message=".*legacy TorchScript")
    example = spike.positions(2, tokens, seed=1)
    torch.onnx.export(
        net, example, str(path), dynamo=False, opset_version=OPSET, input_names=list(INPUTS),
        output_names=list(OUTPUTS), dynamic_axes={n: {0: "batch"} for n in INPUTS + OUTPUTS},
    )
    net.tokens = 0
    graph = onnx.load(str(path))
    onnx.checker.check_model(graph, full_check=True)
    used = {n.op_type for n in graph.graph.node}
    if used - ALLOWED:
        raise SystemExit(f"{path.name}: operators outside the allow-list: {sorted(used - ALLOWED)}")
    return sorted(used)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--out", default=str(pathlib.Path(__file__).parent / "out"))
    out = pathlib.Path(parser.parse_args().out)
    out.mkdir(parents=True, exist_ok=True)
    net = spike.build()
    manifest = {"opset": OPSET, "torch": torch.__version__, "parameters": spike.parameters(net), "features": spike.FEATURES,
                "tokens": list(TOKENS), "batches": list(BATCHES), "reference": REFERENCE, "models": {}}
    for tokens in TOKENS:
        path = out / f"spike-t{tokens}.onnx"
        ops = export(net, tokens, path)
        features, kind, present, option = spike.positions(REFERENCE, tokens, seed=tokens)
        with torch.inference_mode():
            value, policy = net(features, kind, present, option)
        ref = out / f"ref-t{tokens}"
        ref.mkdir(exist_ok=True)
        for name, tensor, dtype in [("features.f32", features, np.float32), ("kind.i64", kind, np.int64),
                                    ("present.u8", present, np.uint8), ("option.u8", option, np.uint8),
                                    ("value.f32", value, np.float32), ("policy.f32", policy, np.float32)]:
            tensor.numpy().astype(dtype).tofile(ref / name)

        session = ort.InferenceSession(str(path), providers=["CPUExecutionProvider"])
        got_value, got_policy = session.run(None, {"features": features.numpy(), "kind": kind.numpy(),
                                                   "present": present.numpy(), "option": option.numpy()})
        drift = float(np.abs(got_value - value.numpy()).max())
        same = float((got_policy.argmax(1) == policy.numpy().argmax(1)).mean())
        manifest["models"][f"t{tokens}"] = {"file": path.name, "bytes": path.stat().st_size, "ops": ops,
                                           "ort_python_value_drift": drift, "ort_python_same_argmax": same}
        print(f"T={tokens}: {path.name} {path.stat().st_size // 1024} KB, ops {', '.join(ops)}")
        print(f"  onnxruntime (python, CPU) against torch: value drift {drift:.2e}, same argmax {same:.2%}")
    (out / "manifest.json").write_text(json.dumps(manifest, indent=2))
    print(f"{manifest['parameters']} parameters, opset {OPSET}, torch {torch.__version__}")


if __name__ == "__main__":
    main()
