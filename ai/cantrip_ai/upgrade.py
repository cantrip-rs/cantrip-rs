"""A schema-1 checkpoint as a schema-4 one that computes what it did (`model.upgrade_schema1`), and the proof.

    python -m cantrip_ai.upgrade --checkpoint IN.pt --out OUT.pt --shards DIR [--positions 512]

The upgraded network reads schema 4's new inputs through projections of their own, at zero, and keeps them out of the
old ones (`Config.schema1_inputs`); the categories schema 4 appends start as the rows they were read as before. The
proof: on positions from schema-1 shards (`--shards`), each given schema-4 inputs at random where schema 1 had none
(new numbers in the columns schema 4 fills, the new categories, the option's cost pointers), the upgraded network on
them must equal the original on the same positions with those inputs read as schema 1 read them (the numbers absent,
the categories as before, no pointer). Every output, to float precision; the checkpoint is written only then.

`--decks DIR` adds to its card table every card of those decklists it has no row for, as training's `--init` adds a
card new to it (`train.card_inputs`: the card's text row, and its fingerprint where the fingerprints have one), so a
network that plays those decks before training sees them knows their cards. Its optimizer state is left out (its
shapes are schema 1's)."""

from __future__ import annotations

import argparse
import random
import sys
from pathlib import Path

import numpy as np
import torch

from . import dataset, embed, model, train

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import shards  # noqa: E402


def positions(directory: Path, n: int, seed: int) -> list[dict]:
    """`n` compact positions from the shards of `directory`, decoded, the first shards' first games' samples."""
    out = []
    for path in shards.shards(directory):
        _, games = shards.read(path)
        for game in games:
            for s in game.samples:
                out.append(shards.position(s.position))
                if len(out) >= n * 4:
                    break
            if len(out) >= n * 4:
                break
        if len(out) >= n * 4:
            break
    random.Random(seed).shuffle(out)
    return out[:n]


def batch(ps: list[dict], table_rows: int, rng: np.random.Generator, with_new: bool) -> tuple[dict, dict]:
    """The positions as a batch at the largest bucket among them: `raw`, with schema 4's new inputs drawn at random
    where `with_new`, and `old`, those inputs as schema 1 read them."""
    t = max(p["bucket"] for p in ps)
    b = len(ps)
    kind = np.zeros((b, t), np.int64)
    card = np.zeros((b, t), np.int64)
    cats = np.zeros((b, t, model.CATS), np.int64)
    ptrs = np.full((b, t, model.PTRS), model.NO_POINTER, np.int64)
    nums = np.zeros((b, t, model.NUMS), np.float32)
    legal = np.zeros((b, t), bool)
    for i, p in enumerate(ps):
        n = p["bucket"]
        kind[i, :n], card[i, :n], cats[i, :n] = p["kind"], np.minimum(p["card"], table_rows - 1), p["cats"]
        ptrs[i, :n], nums[i, :n], legal[i, :n] = p["ptrs"], p["nums"], p["legal"]
    kinds = model.LAYOUT["kinds"]
    old = dict(kind=kind, card=card, cats=cats.copy(), ptrs=ptrs.copy(), nums=nums.copy(), legal=legal)
    if with_new:
        for name, cols in model.SCHEMA1_NEW_NUMS.items():
            at = kind == kinds.index(name)
            cols = list(cols)
            nums[..., cols] = np.where(at[..., None], rng.normal(0, 1, (b, t, len(cols))).astype(np.float32), nums[..., cols])
        for slot, added in model.SCHEMA1_NEW_CATS.items():
            old_vocab = model.SCHEMA1_VOCABS[slot]
            hit = (kind > 0) & (rng.random((b, t)) < 0.2)
            for j, read_as in enumerate(added):
                cats[..., slot] = np.where(hit, old_vocab + j, cats[..., slot])
                old["cats"][..., slot] = np.where(hit, read_as, old["cats"][..., slot])
        for name, slots in model.SCHEMA1_NEW_PTRS.items():
            at = kind == kinds.index(name)
            for slot in slots:
                lengths = (kind > 0).sum(1, keepdims=True)
                target = (rng.random((b, t)) * lengths).astype(np.int64)
                ptrs[..., slot] = np.where(at, target, ptrs[..., slot])
    raw = dict(kind=kind, card=card, cats=cats, ptrs=ptrs, nums=nums, legal=legal)
    to = lambda d: {k: torch.from_numpy(v) for k, v in d.items()}  # noqa: E731
    return to(raw), to(old)


def original(config: dict, state: dict) -> model.Net:
    """The schema-1 network as it was: built with schema 1's category vocabularies."""
    vocabs = model.VOCABS
    model.VOCABS = model.SCHEMA1_VOCABS
    try:
        net = model.from_state(model.Config(**config), state)
    finally:
        model.VOCABS = vocabs
    net.load_state_dict(state)
    return net.eval()


def main(argv=None) -> int:
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("--checkpoint", required=True)
    a.add_argument("--out", required=True)
    a.add_argument("--shards", required=True, help="a directory of schema-1 shards to prove the upgrade on")
    a.add_argument("--positions", type=int, default=512)
    a.add_argument("--seed", type=int, default=1)
    a.add_argument("--decks", help="add the cards of these decklists (.txt) that the card table has no row for")
    args = a.parse_args(argv)
    ck = torch.load(args.checkpoint, map_location="cpu", weights_only=False)
    header = ck.get("header", {})
    if (header.get("schema"), header.get("encoder_hash")) != (1, model.SCHEMA1_HASH):
        raise SystemExit(f"{args.checkpoint}: encoding ({header.get('schema')}, {header.get('encoder_hash')}), not schema 1's")
    config, state = model.upgrade_schema1(ck["config"], ck["state"])
    before = original(ck["config"], ck["state"])
    after = model.from_state(model.Config(**config), state)
    after.load_state_dict(state)
    after.eval()

    ps = positions(Path(args.shards), args.positions, args.seed)
    rng = np.random.default_rng(args.seed)
    worst = {}
    with torch.no_grad():
        for with_new in (False, True):
            for k in range(0, len(ps), 64):
                raw, old = batch(ps[k : k + 64], state["table"].shape[0], rng, with_new)
                ours = after(**raw)
                theirs = before(**old)
                for name, x, y in zip(model.OUTPUTS, ours, theirs):
                    legal = raw["legal"]
                    d = (x - y).abs()
                    d = d[legal] if name in ("policy",) else d
                    key = f"{name}{' (new inputs)' if with_new else ''}"
                    worst[key] = max(worst.get(key, 0.0), float(d.max()) if d.numel() else 0.0)
    for k, v in worst.items():
        print(f"  {k}: max |upgraded - original| {v:.2e}")
    if max(worst.values()) > 1e-4:
        raise SystemExit("the upgraded network does not compute what the original did; nothing written")
    names = model.checkpoint_cards(ck, args.checkpoint)
    if args.decks:
        wanted = []
        for path in sorted(Path(args.decks).glob("*.txt")):
            for line in path.read_text(encoding="utf-8").splitlines():
                m = __import__("re").match(r"^\d+\s+(.+?)\s*$", line.strip())
                if m and m.group(1) not in names and m.group(1) not in wanted:
                    wanted.append(m.group(1))
        if wanted:
            grown = names + wanted
            table, _, _ = train.card_inputs(dict(ck, config=config, state=state), args.checkpoint, names, grown, "text",
                                            embed.texts())
            if not np.array_equal(table[: len(names) + 1], state["table"].numpy()):
                raise SystemExit("extending the card table changed the rows it had")
            state = dict(state, table=torch.from_numpy(np.ascontiguousarray(table, np.float32)))
            names = grown
            print(f"  card table: {len(wanted)} cards added ({', '.join(wanted)})")
    ck = dict(ck, config=config, state=state, optimizer=None, param_names=None, cards=names,
              header=dict(header, schema=model.LAYOUT["schema"], encoder_hash=dataset.encoder_hash(),
                          upgraded_from=f"1/{model.SCHEMA1_HASH}"))
    Path(args.out).parent.mkdir(parents=True, exist_ok=True)
    torch.save(ck, args.out)
    # The reference positions export checks its graphs on (`reference.bin`): schema 1's, the same bytes as schema 4's
    # with nothing new in them, relabelled (the Rust reader takes a position's schema byte as it is).
    ref = Path(args.checkpoint).parent / "reference.bin"
    if ref.is_file():
        from . import export  # noqa: PLC0415
        ps = export.read_positions(ref)
        if any(q[0] != 1 for q in ps):
            raise SystemExit(f"{ref}: not every position is schema 1's")
        export.write_positions(Path(args.out).parent / "reference.bin", [bytes([model.LAYOUT["schema"]]) + q[1:] for q in ps])
        print(f"  reference.bin: {len(ps)} positions, relabelled schema {model.LAYOUT['schema']}")
    print(f"{args.out}: schema {model.LAYOUT['schema']} ({dataset.encoder_hash()}), {len(ps)} positions proved")
    return 0


if __name__ == "__main__":
    sys.exit(main())
