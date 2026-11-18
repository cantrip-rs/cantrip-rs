"""How networks play a deck by its own decisions: policy cross-entropy and top-1 against the recorded choice, value
MSE against the result, on every position where a seat playing `--deck` decides, in the games of `--data` (for a
network trained without that deck: how its card input carries over to cards and plans it never saw).

    python -m cantrip_ai.transfer_eval --data DIR [DIR ...] --deck NAME --checkpoint NAME=CKPT [...] [--max-games N]
        [--out FILE]
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import numpy as np
import torch

from . import dataset, embed, model
from .train import card_inputs, choose_device, evaluate


def run(spec: str, dirs: list, deck: str, device, max_games: int | None, other: bool) -> dict:
    name, path = spec.split("=", 1)
    ck = torch.load(path, map_location="cpu", weights_only=False)
    base = model.checkpoint_cards(ck, Path(path))
    decks: list = []
    data = dataset.load(dirs, max_games, log=lambda *_: None, base=base, game_decks=decks)
    seat_deck = np.array([(decks[g][s] if s < len(decks[g]) else "?") for g, s in zip(data.game, data.seat)])
    rows = np.flatnonzero((seat_deck != deck) if other else (seat_deck == deck))
    table, _, features = card_inputs(ck, Path(path), base, data.names, "text", embed.texts(), log=lambda *_: None)
    net = model.build(model.Config(**ck["config"]), table, features=features.buffers() if features is not None else None)
    net.load_state_dict(model.grow_state(net, dict(ck["state"], table=torch.from_numpy(np.ascontiguousarray(table, np.float32)))))
    net.to(device)
    m = evaluate(net, data, dataset.batches(data, rows, 512, None), 0, data.header["mode"] != "full", device)
    games = len(np.unique(data.game[rows]))
    return dict(name=name, checkpoint=path, deck=deck, other=other, games=games, **m)


def main():
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("--data", nargs="+", required=True)
    a.add_argument("--deck", required=True, help="the deck whose seats' decisions are measured")
    a.add_argument("--others", action="store_true", help="measure every other seat's decisions instead")
    a.add_argument("--checkpoint", nargs="+", required=True, help="NAME=PATH to a checkpoint.pt")
    a.add_argument("--device", default="auto")
    a.add_argument("--max-games", type=int)
    a.add_argument("--out")
    args = a.parse_args()
    device = choose_device(args.device)
    results = []
    for c in args.checkpoint:
        r = run(c, args.data, args.deck, device, args.max_games, args.others)
        results.append(r)
        print(f"{r['name']}: {'not ' if r['other'] else ''}{r['deck']}, {r['choices']} choices / {r['samples']} positions of "
              f"{r['games']} games: policy CE {r['policy_ce']:.4f}, top-1 {r['top1']:.4f}, value MSE {r['value_mse']:.4f}, "
              f"sign {r['value_sign']:.3f}", flush=True)
    if args.out:
        Path(args.out).write_text(json.dumps(results, indent=1), encoding="utf-8")


if __name__ == "__main__":
    main()
