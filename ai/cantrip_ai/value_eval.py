"""Held-out value quality of trained networks on the same positions: mean squared error of the
value against the game's result, and calibration - the value read as a win probability, (v + 1) / 2, against how
often the seat won (a draw or a cap counts a half) - as the expected calibration error over ten bins, overall and by
turn band, with the reliability table.

    python -m cantrip_ai.value_eval --data DIR [DIR ...] --checkpoint NAME=CKPT [NAME=CKPT ...] [--games N] [--seed S]
        [--held-out F] [--max-games N] [--out FILE]

The positions are the held-out games of `dataset.split` at the seed and fraction given (as `train.py` holds them out),
at most `--games` of them, read once per checkpoint with that checkpoint's card index.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import torch

from . import dataset, embed, model
from .train import choose_device

BANDS = ((0, 2), (3, 5), (6, 9), (10, 14), (15, 999))


def ece(p: np.ndarray, y: np.ndarray, bins: int = 10) -> tuple[float, list]:
    edges = np.linspace(0, 1, bins + 1)
    total, table = 0.0, []
    for i in range(bins):
        m = (p >= edges[i]) & ((p < edges[i + 1]) if i < bins - 1 else (p <= 1))
        if m.sum() == 0:
            continue
        gap = abs(p[m].mean() - y[m].mean())
        total += gap * m.sum() / len(p)
        table.append(dict(bin=f"{edges[i]:.1f}-{edges[i + 1]:.1f}", n=int(m.sum()), predicted=float(p[m].mean()), won=float(y[m].mean())))
    return float(total), table


def evaluate(spec: str, data_dirs: list, games: int, seed: int, held_out: float, device, max_games: int | None = None) -> dict:
    name, path = spec.split("=", 1)
    ck = torch.load(path, map_location="cpu", weights_only=False)
    base = model.checkpoint_cards(ck, Path(path))
    data = dataset.load(data_dirs, max_games, log=lambda *_: None, seed=seed, base=base)
    _, held = dataset.split(data, held_out, seed)
    held_games = np.unique(data.game[held])[:games]
    rows = held[np.isin(data.game[held], held_games)]
    known = embed.texts()
    kept = ck["state"]["table"].numpy()
    basis = embed.Basis.from_state(ck["embed"]) if ck.get("embed") else embed.legacy_basis(base, kept, log=lambda *_: None, known=known)
    table = embed.extend(kept, data.names, basis, known)
    config = model.Config(**ck["config"])
    net = model.build(config, table)
    net.load_state_dict(dict(ck["state"], table=torch.from_numpy(np.ascontiguousarray(table, np.float32))))
    net.to(device).eval()
    values = []
    with torch.no_grad():
        for k in range(0, len(rows), 512):
            r = rows[k : k + 512]
            x = dataset.decode(data.blob, data.offset, r)
            x["card"] = dataset.cards(data, r, x["card"])
            t = {key: torch.from_numpy(np.ascontiguousarray(v)).to(device) for key, v in x.items()}
            v = net.heads(t["kind"], t["card"], t["cats"], t["ptrs"], t["nums"], t["legal"])[0]
            values.append(v.float().cpu().numpy())
    v = np.concatenate(values) if values else np.zeros(0)
    outcome = data.outcome[rows]
    p = (v + 1) / 2
    y = (outcome + 1) / 2
    turn = data.turn[rows]
    out = dict(name=name, checkpoint=path, positions=int(len(rows)), games=int(len(held_games)),
               value_mse=float(((v - outcome) ** 2).mean()), sign=float(((v > 0) == (outcome > 0)).mean()))
    out["ece"], out["reliability"] = ece(p, y)
    out["bands"] = []
    for lo, hi in BANDS:
        m = (turn >= lo) & (turn <= hi)
        if m.sum() == 0:
            continue
        e, _ = ece(p[m], y[m])
        out["bands"].append(dict(turns=f"{lo}-{hi}", n=int(m.sum()), mse=float(((v[m] - outcome[m]) ** 2).mean()), ece=e,
                                 predicted=float(p[m].mean()), won=float(y[m].mean()), spread=float(p[m].std())))
    return out


def main():
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("--data", nargs="+", required=True)
    a.add_argument("--checkpoint", nargs="+", required=True, help="NAME=PATH to a checkpoint.pt")
    a.add_argument("--games", type=int, default=400)
    a.add_argument("--seed", type=int, default=0)
    a.add_argument("--held-out", type=float, default=0.02)
    a.add_argument("--device", default="auto")
    a.add_argument("--max-games", type=int, help="read at most this many games from each directory")
    a.add_argument("--out")
    args = a.parse_args()
    device = choose_device(args.device)
    results = [evaluate(c, args.data, args.games, args.seed, args.held_out, device, args.max_games) for c in args.checkpoint]
    for r in results:
        print(f"{r['name']}: {r['positions']} positions of {r['games']} games, value MSE {r['value_mse']:.4f}, sign {r['sign']:.3f}, ECE {r['ece']:.4f}")
        for b in r["bands"]:
            print(f"  turns {b['turns']:>6}: n {b['n']:6d}  MSE {b['mse']:.4f}  ECE {b['ece']:.4f}  predicted {b['predicted']:.3f}  won {b['won']:.3f}  spread {b['spread']:.3f}")
    if args.out:
        Path(args.out).write_text(json.dumps(results, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
