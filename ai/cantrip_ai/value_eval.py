"""Held-out value quality of trained networks on the same positions: mean squared error of the
value against the game's result, and calibration - the value read as a win probability, (v + 1) / 2, against how
often the seat won (a draw or a cap counts a half) - as the expected calibration error over ten bins, overall and by
turn band, with the reliability table.

    python -m cantrip_ai.value_eval --data DIR [DIR ...] --checkpoint NAME=CKPT [NAME=CKPT ...] [--games N] [--seed S]
        [--held-out F] [--every-game] [--policy] [--max-games N] [--out FILE]

The positions are the held-out games of `dataset.split` at the seed and fraction given (as `train.py` holds them out),
at most `--games` of them, read once per checkpoint with that checkpoint's card index. `--every-game` takes the
directories' games as held out already (a permanent held-out set no network trains on: the generation driver's
`ROOT/heldout`), at most `--games` of them.

`--policy` scores the policy too, as `train.py` scores its held-out set (`train.evaluate`): the cross-entropy against
the visit targets, top-1 against the most visited option, and top-1 against the option played, over the positions with
more than one legal option whose visits are a policy target. The games are read with `dataset.load`'s
`searched_policy`, so in self-played games those are the full searches' targets alone (a fast search of playout-cap
randomisation, and a decision no search answered, are not policy targets).

Each checkpoint is also read by the seat's deck (`seats`: over the positions a deck's seat answered, the mean outcome
and the network's mean value, and with a heuristic head, `model.Config.residual`, that head's mean log-odds and the
residual's: a value head gone wrong for one deck shows here and in no total) and by game (`by_game`: each game's
squared error and positions and, with `--policy`, its policy cross-entropy and targets), from which the first
checkpoint less each other is printed paired by game (`reports.ratio_difference`), with a 95% half-width.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import torch

from . import dataset, embed, model, reports
from .train import card_inputs, choose_device

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


def visit_targets(data, rows: np.ndarray, tokens: int) -> np.ndarray:
    """Each of `rows`' policy target over its tokens [len(rows), tokens]: the visits' shares (`dataset.labels`')."""
    visits = np.zeros((len(rows), tokens), np.float32)
    for i, r in enumerate(rows):
        a, b = data.visit_offset[r], data.visit_offset[r + 1]
        np.add.at(visits[i], data.visit_token[a:b], data.visit_prob[a:b])
    return visits


def seats(decks: list, game: np.ndarray, seat: np.ndarray, outcome: np.ndarray, value: np.ndarray,
          heuristic: np.ndarray | None = None, residual: np.ndarray | None = None) -> list[dict]:
    """The positions by the answering seat's deck (`decks[game][seat]`, `GameMeta::decks`; "?" for a game that names
    none): how many, of how many games, the mean outcome for the seat and the mean value, and with a heuristic head its
    mean log-odds and the residual's. A network that values a deck's seat far from how its games end (`reports.SEAT_GAP`)
    has a value head gone wrong for that deck, which the value's total error barely shows."""
    name = np.array([(decks[g][s] if g < len(decks) and s < len(decks[g]) else "?")
                     for g, s in zip(game.tolist(), seat.tolist())], dtype=object)
    out = []
    for deck in sorted(set(name.tolist())):
        m = name == deck
        row = dict(deck=deck, positions=int(m.sum()), games=int(len(np.unique(game[m]))), outcome=float(outcome[m].mean()),
                   value=float(value[m].mean()))
        if heuristic is not None:
            row.update(heuristic=float(heuristic[m].mean()), residual=float(residual[m].mean()))
        out.append(row)
    return out


def by_game(game: np.ndarray, squared_error: np.ndarray, ce: np.ndarray | None, choice: np.ndarray) -> dict:
    """Each game's totals, for pairing two networks by game (`reports.ratio_difference`): its positions and their
    squared error, and with the policy scored (`ce`, each position's cross-entropy) its policy targets (`choice`) and
    their cross-entropy."""
    ids, at = np.unique(game, return_inverse=True)
    n = len(ids)
    out = dict(games=ids.tolist(), positions=np.bincount(at, minlength=n).tolist(),
               squared_error=np.bincount(at, weights=squared_error.astype(np.float64), minlength=n).tolist())
    if ce is not None:
        out.update(choices=np.bincount(at, weights=choice.astype(np.float64), minlength=n).astype(np.int64).tolist(),
                   policy_ce=np.bincount(at, weights=np.where(choice, ce, 0.0).astype(np.float64), minlength=n).tolist())
    return out


def evaluate(spec: str, data_dirs: list, games: int, seed: int, held_out: float, device, max_games: int | None = None,
             every_game: bool = False, policy: bool = False) -> dict:
    name, path = spec.split("=", 1)
    ck = torch.load(path, map_location="cpu", weights_only=False)
    base = model.checkpoint_cards(ck, Path(path))
    decks: list = []
    data = dataset.load(data_dirs, max_games, log=lambda *_: None, seed=seed, base=base, searched_policy=policy,
                        game_decks=decks)
    held = np.arange(len(data.game)) if every_game else dataset.split(data, held_out, seed)[1]
    held_games = np.unique(data.game[held])[:games]
    rows = held[np.isin(data.game[held], held_games)]
    table, _, features = card_inputs(ck, Path(path), base, data.names, "text", embed.texts(), log=lambda *_: None)
    config = model.Config(**ck["config"])
    net = model.build(config, table, features=features.buffers() if features is not None else None)
    net.load_state_dict(model.grow_state(net, dict(ck["state"], table=torch.from_numpy(np.ascontiguousarray(table, np.float32)))))
    net.to(device).eval()
    values, heads, residuals, ces = [], [], [], []
    with torch.no_grad():
        for k in range(0, len(rows), 512):
            r = rows[k : k + 512]
            x = dataset.decode(data.blob, data.offset, r)
            x["card"] = dataset.cards(data, r, x["card"])
            t = {key: torch.from_numpy(np.ascontiguousarray(v)).to(device) for key, v in x.items()}
            # `Net.heads`' value and policy, with the value's parts kept.
            tokens, summary = net.body(t["kind"], t["card"], t["cats"], t["ptrs"], t["nums"])
            v, parts = net.value_parts(summary)
            values.append(v.float().cpu().numpy())
            if parts is not None:
                heads.append(parts[0].float().cpu().numpy())
                residuals.append(parts[1].float().cpu().numpy())
            if policy:
                # Each position's cross-entropy against its visits, as `train.losses` takes it.
                logits = torch.where(t["legal"], net.policy(tokens).squeeze(-1), net.masked)
                logp = torch.log_softmax(logits.float(), -1).cpu().numpy()
                target = visit_targets(data, r, logp.shape[1])
                ces.append(-np.where(target > 0, target * logp, 0.0).sum(-1))
    v = np.concatenate(values) if values else np.zeros(0)
    outcome = data.outcome[rows]
    p = (v + 1) / 2
    y = (outcome + 1) / 2
    turn = data.turn[rows]
    out = dict(name=name, checkpoint=path, positions=int(len(rows)), games=int(len(held_games)),
               value_mse=float(((v - outcome) ** 2).mean()), sign=float(((v > 0) == (outcome > 0)).mean()))
    out["ece"], out["reliability"] = ece(p, y)
    if policy:
        from .train import evaluate as scored  # noqa: PLC0415 (train.py's own held-out scoring)

        m = scored(net, data, dataset.batches(data, rows, 512, None), 0, False, device)
        out.update(policy_ce=m["policy_ce"], top1=m["top1_visits"], top1_played=m["top1"], choices=m["choices"])
    choice = (data.legal[rows] > 1) & (data.policy[rows] > 0)
    out["seats"] = seats(decks, data.game[rows], data.seat[rows], outcome, v, np.concatenate(heads) if heads else None,
                         np.concatenate(residuals) if residuals else None)
    out["by_game"] = by_game(data.game[rows], (v - outcome) ** 2, np.concatenate(ces) if ces else None, choice)
    out["bands"] = []
    for lo, hi in BANDS:
        m = (turn >= lo) & (turn <= hi)
        if m.sum() == 0:
            continue
        e, _ = ece(p[m], y[m])
        out["bands"].append(dict(turns=f"{lo}-{hi}", n=int(m.sum()), mse=float(((v[m] - outcome[m]) ** 2).mean()), ece=e,
                                 predicted=float(p[m].mean()), won=float(y[m].mean()), spread=float(p[m].std())))
    return out


def main(argv=None):
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("--data", nargs="+", required=True)
    a.add_argument("--checkpoint", nargs="+", required=True, help="NAME=PATH to a checkpoint.pt")
    a.add_argument("--games", type=int, default=400)
    a.add_argument("--seed", type=int, default=0)
    a.add_argument("--held-out", type=float, default=0.02)
    a.add_argument("--device", default="auto")
    a.add_argument("--max-games", type=int, help="read at most this many games from each directory")
    a.add_argument("--every-game", action="store_true",
                   help="the directories are a held-out set: every game of them, at most --games, not dataset.split's")
    a.add_argument("--policy", action="store_true",
                   help="also the policy's cross-entropy and top-1 against the policy targets (searched_policy: in "
                        "self-played games the full searches')")
    a.add_argument("--out")
    args = a.parse_args(argv)
    device = choose_device(args.device)
    results = [evaluate(c, args.data, args.games, args.seed, args.held_out, device, args.max_games, args.every_game, args.policy)
               for c in args.checkpoint]
    for r in results:
        print(f"{r['name']}: {r['positions']} positions of {r['games']} games, value MSE {r['value_mse']:.4f}, sign {r['sign']:.3f}, ECE {r['ece']:.4f}")
        if "policy_ce" in r:
            print(f"  policy on {r['choices']} targets: CE {r['policy_ce']:.4f}, top-1 {r['top1']:.3f} (of the option played {r['top1_played']:.3f})")
        for b in r["bands"]:
            print(f"  turns {b['turns']:>6}: n {b['n']:6d}  MSE {b['mse']:.4f}  ECE {b['ece']:.4f}  predicted {b['predicted']:.3f}  won {b['won']:.3f}  spread {b['spread']:.3f}")
    table = reports.heldout_seats(results)
    for line in (reports.seat_table(table) if table else []) + [f"WARNING: {f}" for f in reports.seat_flags(table)]:
        print(line)
    for other in results[1:]:
        if text := reports.pairs_text(reports.heldout_pairs([results[0], other])):
            print(text)
    if args.out:
        Path(args.out).write_text(json.dumps(results, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
