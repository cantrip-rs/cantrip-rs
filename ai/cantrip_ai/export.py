"""Export a trained network to ONNX (docs/ai-player-plan.md, "Network" and "Serving and measuring").

    python -m cantrip_ai.export --checkpoint DIR/checkpoint.pt --out DIR [--positions FILE]

Writes into DIR:
- `model-t{32,64,128}.onnx`: one graph per token bucket, the batch dimension free; the TorchScript exporter
  (`dynamo=False`), opset 17, every operator on `ALLOWED` (the spike's list, and `Concat`).
  Inputs `kind` [B,T] i64, `card` [B,T] i64, `cats` [B,T,9] i64, `ptrs` [B,T,4] i64, `nums` [B,T,96] f32,
  `legal` [B,T] bool; outputs `value` [B], `policy` [B,T] (masked to the legal options), `aux` [B,2] (life swing,
  P(dead next turn)), `belief` [B,T].
- `cards.json`: the card index the model was trained with, `{"model", "dim", "rows", "hash", "names"}` (the runtime's
  index is the engine's own cards, mapped onto these by name; a card the model never saw gets row 0).
- `reference/`: the parity references. `positions.bin` (u32 count, then each position's u32 length and compact
  bytes, `cantrip_obs::encode::to_compact`); what PyTorch makes of each, padded to its own bucket: `value.f32` [N],
  `auxiliary.f32` [N,2] (the `aux` output), `policy.f32` (each position's option logits, `options` of them, end to end; `MASKED` where not
  legal); and `decoded.fnv` [N] u64, FNV-1a 64 of the position as `ai/shards.py` decodes it (kind u8, card u32,
  cats u8, ptrs u8, nums f32, legal u8, each at the bucket's size, little-endian), which the Rust side computes from
  `from_compact` to show the two decoders agree.
- `manifest.json`: schema, encoder hash, card index hash, engine commit, the training data's header, the buckets'
  files, the parameter count, the configuration and the held-out metrics.
- `model.pt`: what the inference server (`serve.py`) runs, so a model directory serves itself: the configuration, the
  weights (the card table included) and whether the table is folded, as the ONNX graphs were made. Only tensors and
  plain values, so it loads with `torch.load(weights_only=True)`.

`--weights-only` writes `model.pt` alone into a model directory exported before export wrote it, and changes nothing
else there.

The positions are `--positions` (that format), else the `reference.bin` train.py wrote beside the checkpoint (held-out
positions).
"""

from __future__ import annotations

import argparse
import json
import os
import struct
import sys
import warnings
from pathlib import Path

import numpy as np
import onnx
import onnxruntime as ort
import torch

from . import dataset, model

AI = model.AI
sys.path.insert(0, str(AI))
import shards  # noqa: E402

OPSET = 17
BUCKETS = (32, 64, 128)
ALLOWED = {
    "Add", "Cast", "Clip", "Concat", "Constant", "Div", "Erf", "Gather", "Gemm", "Identity", "LayerNormalization",
    "MatMul", "Mul", "ReduceSum", "Reshape", "Softmax", "Split", "Squeeze", "Sub", "Tanh", "Transpose", "Unsqueeze",
    "Where",
}


def read_positions(path: Path) -> list[bytes]:
    data = path.read_bytes()
    (n,) = struct.unpack_from("<I", data, 0)
    at, out = 4, []
    for _ in range(n):
        (k,) = struct.unpack_from("<I", data, at)
        out.append(data[at + 4 : at + 4 + k])
        at += 4 + k
    if at != len(data):
        raise SystemExit(f"{path}: bytes left over")
    return out


def write_positions(path: Path, positions: list[bytes]):
    with path.open("wb") as f:
        f.write(struct.pack("<I", len(positions)))
        for p in positions:
            f.write(struct.pack("<I", len(p)))
            f.write(p)


def example(tokens: int, n: int = 2):
    return (torch.ones(n, tokens, dtype=torch.long), torch.zeros(n, tokens, dtype=torch.long),
            torch.zeros(n, tokens, model.CATS, dtype=torch.long), torch.full((n, tokens, model.PTRS), model.NO_POINTER),
            torch.zeros(n, tokens, model.NUMS), torch.ones(n, tokens, dtype=torch.bool))


def export(net, tokens: int, path: Path) -> list[str]:
    net.tokens = tokens
    warnings.filterwarnings("ignore", category=DeprecationWarning)
    warnings.filterwarnings("ignore", category=UserWarning)
    with torch.no_grad():
        torch.onnx.export(net, example(tokens), str(path), dynamo=False, opset_version=OPSET,
                          input_names=list(model.INPUTS), output_names=list(model.OUTPUTS),
                          dynamic_axes={n: {0: "batch"} for n in model.INPUTS + model.OUTPUTS})
    net.tokens = 0
    graph = onnx.load(str(path))
    onnx.checker.check_model(graph, full_check=True)
    used = {n.op_type for n in graph.graph.node}
    if used - ALLOWED:
        raise SystemExit(f"{path.name}: operators outside the allow-list: {sorted(used - ALLOWED)}")
    return sorted(used)


def fnv1a64(arrays) -> int:
    data = b"".join(np.ascontiguousarray(a).tobytes() for a in arrays)
    h = 0xCBF29CE484222325
    for b in data:
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


def digest(p: dict) -> int:
    return fnv1a64([p["kind"].astype("<u1"), p["card"].astype("<u4"), p["cats"].astype("<u1"), p["ptrs"].astype("<u1"),
                    p["nums"].astype("<f4"), p["legal"].astype("<u1")])


def inputs_of(positions: list[bytes], tokens: int) -> dict[str, np.ndarray]:
    blob = np.frombuffer(b"".join(positions), np.uint8)
    offset = np.concatenate([[0], np.cumsum([len(p) for p in positions])]).astype(np.int64)
    return dataset.decode(blob, offset, np.arange(len(positions)), tokens)


def references(net, positions: list[bytes], out: Path, paths: dict[int, Path]) -> dict:
    """PyTorch's outputs for each position at its own bucket, written to `out`; the Python decoders checked
    against each other; and onnxruntime (Python, CPU) against PyTorch."""
    out.mkdir(parents=True, exist_ok=True)
    write_positions(out / "positions.bin", positions)
    n = len(positions)
    value = np.zeros(n, np.float32)
    aux = np.zeros((n, 2), np.float32)
    logits: list[np.ndarray] = [None] * n
    digests = np.zeros(n, "<u8")
    drift = dict(value=0.0, aux=0.0, policy=0.0)
    same = 0
    for i, p in enumerate(positions):
        d = shards.position(p)
        digests[i] = digest(d)
        mine = inputs_of([p], d["bucket"])
        for k in model.INPUTS:
            if not np.array_equal(mine[k][0], d[k].astype(mine[k].dtype)):
                raise SystemExit(f"dataset.decode and shards.position disagree on {k} of position {i}")
    for bucket in BUCKETS:
        rows = [i for i, p in enumerate(positions) if p[1] == bucket]
        if not rows:
            continue
        x = inputs_of([positions[i] for i in rows], bucket)
        with torch.no_grad():
            v, pol, a, _ = net(*(torch.from_numpy(x[k]) for k in model.INPUTS))
        v, pol, a = v.numpy(), pol.numpy(), a.numpy()
        session = ort.InferenceSession(str(paths[bucket]), providers=["CPUExecutionProvider"])
        ov, opol, oa, _ = session.run(None, {k: x[k] for k in model.INPUTS})
        drift["value"] = max(drift["value"], float(np.abs(ov - v).max()))
        drift["aux"] = max(drift["aux"], float(np.abs(oa - a).max()))
        for j, i in enumerate(rows):
            first, options = positions[i][3], positions[i][4]
            value[i] = v[j]
            aux[i] = a[j]
            logits[i] = pol[j, first : first + options]
            legal = x["legal"][j]
            drift["policy"] = max(drift["policy"], float(np.abs(opol[j][legal] - pol[j][legal]).max()))
            same += int(opol[j].argmax() == pol[j].argmax())
    value.tofile(out / "value.f32")
    aux.tofile(out / "auxiliary.f32")  # not aux.f32: AUX is a reserved file name on Windows
    np.concatenate(logits).astype(np.float32).tofile(out / "policy.f32")
    digests.tofile(out / "decoded.fnv")
    return dict(positions=n, ort_python=dict(**drift, same_argmax=same / max(n, 1)))


def write_weights(net, checkpoint: dict, path: Path):
    """`model.pt`: the network's configuration and weights for the inference server, with the folded card table the
    graphs were made with (`model.fold`) when it is folded, so the server runs what the graphs run."""
    folded = net.folded is not None
    weights = dict(format=1, config=dict(checkpoint["config"]), folded=folded,
                   state={k: v.detach().cpu().contiguous() for k, v in checkpoint["state"].items()},
                   table=net.folded.detach().cpu().contiguous() if folded else None)
    tmp = path.with_name(path.name + ".tmp")
    torch.save(weights, tmp)
    os.replace(tmp, path)


def main(argv=None):
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("--checkpoint", required=True)
    a.add_argument("--out", required=True)
    a.add_argument("--positions", help="reference positions (default: reference.bin beside the checkpoint)")
    a.add_argument("--weights-only", action="store_true", help="write model.pt alone into an exported model directory")
    args = a.parse_args(argv)
    torch.set_num_threads(os.cpu_count() or 4)

    checkpoint = torch.load(args.checkpoint, map_location="cpu", weights_only=False)
    names = model.checkpoint_cards(checkpoint, args.checkpoint)
    table = checkpoint["state"]["table"].numpy()
    if table.shape[0] != len(names) + 1:
        raise SystemExit(f"the checkpoint's table has {table.shape[0]} rows for {len(names)} names")
    header = checkpoint["header"]
    if header["schema"] != model.LAYOUT["schema"] or header["encoder_hash"] != dataset.encoder_hash():
        raise SystemExit(f"the checkpoint's encoding ({header['schema']}, {header['encoder_hash']}) is not ai/encoding.json's")
    net = model.Net(model.Config(**checkpoint["config"]), table)
    net.load_state_dict(checkpoint["state"])
    net.eval()
    folded = net.config.width < table.shape[1]
    if folded:
        model.fold(net)

    out = Path(args.out)
    if args.weights_only:
        if not (out / "manifest.json").exists():
            raise SystemExit(f"{out} is not an exported model directory (no manifest.json)")
        write_weights(net, checkpoint, out / "model.pt")
        print(f"wrote {out / 'model.pt'}")
        return
    out.mkdir(parents=True, exist_ok=True)
    files, ops = {}, set()
    for tokens in BUCKETS:
        path = out / f"model-t{tokens}.onnx"
        ops |= set(export(net, tokens, path))
        files[str(tokens)] = path.name
        print(f"T={tokens}: {path.name} {path.stat().st_size // 1024} KB")
    text_hash = dataset.index_hash(names)
    basis = checkpoint.get("embed") or {}
    index = dict(model=basis.get("model", "legacy"), dim=int(table.shape[1]), rows=len(names) + 1, hash=text_hash,
                 names=names)
    (out / "cards.json").write_text(json.dumps(index, ensure_ascii=False, indent=0) + "\n", encoding="utf-8")

    positions_path = Path(args.positions) if args.positions else Path(args.checkpoint).parent / "reference.bin"
    positions = read_positions(positions_path)
    # A reference from an old shard can hold a number too large for f16. dataset.decode saturates it and the Rust decoder
    # does not, so leave it out rather than fail the parity check.
    finite = [p for p in positions if np.isfinite(shards.position(p)["nums"]).all()]
    if len(finite) != len(positions):
        print(f"references: skipped {len(positions) - len(finite)} positions with non-finite numbers")
        positions = finite
    if not positions:
        raise SystemExit(f"{positions_path}: no reference position with finite numbers")
    ref = references(net, positions, out / "reference", {t: out / files[str(t)] for t in BUCKETS})
    print(f"references: {ref}")

    manifest = dict(
        schema=model.LAYOUT["schema"],
        encoder_hash=dataset.encoder_hash(),
        text_hash=text_hash,
        commit=header.get("commit"),
        data=dict(header=header, **checkpoint.get("data", {})),
        buckets=files,
        inputs=list(model.INPUTS),
        outputs=list(model.OUTPUTS),
        opset=OPSET,
        ops=sorted(ops),
        torch=torch.__version__,
        parameters=model.parameters(net),
        config=checkpoint["config"],
        trained=dict(step=checkpoint["step"], args=checkpoint.get("args")),
        metrics=checkpoint["metrics"].get("held_out"),
        card_table_folded=folded,
        belief_trained=header.get("mode") != "full",
        reference=ref,
    )
    (out / "manifest.json").write_text(json.dumps(manifest, indent=1))
    write_weights(net, checkpoint, out / "model.pt")
    print(f"{manifest['parameters']} parameters, ops {', '.join(sorted(ops))}")


if __name__ == "__main__":
    main()
