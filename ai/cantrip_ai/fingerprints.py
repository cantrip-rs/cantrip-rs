"""Card fingerprints as the network's card table: what each card does to a game, measured by the engine
(`cantrip-fingerprint`, crates/cantrip-ai-host/src/bin/fingerprint.rs), in place of (or beside) the words of its text.

The tool plays each card in every way it can in a few fixed positions and records, per position, how many ways it had
(`roots`), how many outcomes they led to (`leaves`), and the max, min and mean of each outcome feature (life, cards,
each tracked permanent's fate, the permanents made, the mana spent). A card's raw vector is those, position by position:
whether it could act, log counts, and the features signed-log scaled. The table is the raw vectors standardised and
projected onto their top `dim` principal directions, fitted on every card the file holds - the engine's whole content,
not the cards of any game - so it is the same table whichever games a network trains on, and a card no game played is
as described as one every game did.

A name the tool could not deal (a split card's half, an adventure, a face) takes the card it belongs to; a token or an
emblem, which no deck holds, has no fingerprint and gets the unknown row, zeros.

`python -m cantrip_ai.fingerprints --file FILE [--card NAME] [--near NAME]` prints the table's statistics, a card's
nearest neighbours among the cards of the three decks and the content.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from functools import lru_cache
from pathlib import Path

import numpy as np

from .card_features import FACES, REPO

DIM = 96


def _faces(root: Path = REPO) -> dict:
    """{face or half name: its card's name}, for the names the tool cannot deal on their own."""
    out = {}
    for path in sorted((root / "content").glob("*/cards/*.json")):
        try:
            card = json.loads(path.read_text(encoding="utf-8-sig"))
        except (OSError, ValueError):
            continue
        if not isinstance(card, dict) or "name" not in card:
            continue
        for k in FACES:
            if isinstance(card.get(k), dict) and card[k].get("name"):
                out.setdefault(card[k]["name"], card["name"])
        for h in card.get("halves", []) or []:
            if h.get("name"):
                out.setdefault(h["name"], card["name"])
    return out


def _slog(x: np.ndarray) -> np.ndarray:
    return np.sign(x) * np.log1p(np.abs(x))


def raw(path: str | Path) -> tuple[list, np.ndarray, dict]:
    """The names with a fingerprint, their raw vectors, and the file's header."""
    lines = Path(path).read_text(encoding="utf-8").splitlines()
    header = json.loads(lines[0])
    probes = header["probes"]
    width = len(header["features"])
    names, vectors = [], []
    for line in lines[1:]:
        r = json.loads(line)
        if not any(v.get("leaves", 0) > 0 for v in r.get("probes", {}).values()):
            continue
        parts = []
        for p in probes:
            v = r["probes"].get(p)
            if v is None:
                parts.append(np.zeros(3 + 3 * width, np.float32))
                continue
            head = [float(v["roots"] > 0), np.log1p(v["roots"]), np.log1p(v["leaves"])]
            body = _slog(np.concatenate([v["max"], v["min"], v["mean"]]).astype(np.float32))
            parts.append(np.concatenate([np.array(head, np.float32), body]))
        names.append(r["name"])
        vectors.append(np.concatenate(parts))
    return names, np.stack(vectors).astype(np.float32), header


@lru_cache(maxsize=4)
def table_of(path: str, dim: int = DIM) -> tuple[dict, np.ndarray, str]:
    """{name: row} over every fingerprinted card and every face naming one, the projected rows, and a hash of the
    file and `dim` (what a checkpoint records)."""
    names, x, _ = raw(path)
    mean, std = x.mean(0), x.std(0)
    keep = std > 1e-6
    z = (x[:, keep] - mean[keep]) / std[keep]
    # The principal directions, signs fixed so the table is the same on every machine.
    _, s, vt = np.linalg.svd(z, full_matrices=False)
    vt = vt[:dim]
    vt *= np.sign(vt[np.arange(len(vt)), np.abs(vt).argmax(1)])[:, None]
    rows = (z @ vt.T) / np.sqrt(len(z))
    rows = rows / max(float(np.abs(rows).std()), 1e-6) * 0.5
    index = {n: i for i, n in enumerate(names)}
    for face, card in _faces().items():
        if face not in index and card in index:
            index[face] = index[card]
    digest = hashlib.sha256(Path(path).read_bytes() + str(dim).encode()).hexdigest()[:16]
    return index, rows.astype(np.float32), digest


def rows(names: list, path: str | Path, dim: int = DIM) -> np.ndarray:
    """The table for the index `names` (`names[i]` is row `i + 1`; row 0, the unknown card, is zeros)."""
    index, table, _ = table_of(str(path), dim)
    out = np.zeros((len(names) + 1, table.shape[1]), np.float32)
    for i, n in enumerate(names):
        j = index.get(n)
        if j is not None:
            out[i + 1] = table[j]
    return out


def state(path: str | Path, dim: int, with_text: bool) -> dict:
    """What a checkpoint records of its card table, to make the same rows for names it did not have."""
    return dict(source="fingerprints", path=str(path), dim=dim, text=with_text, hash=table_of(str(path), dim)[2])


def main():
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("--file", required=True)
    a.add_argument("--dim", type=int, default=DIM)
    a.add_argument("--near", nargs="*", default=[], help="cards whose nearest neighbours to print")
    args = a.parse_args()
    names, x, header = raw(args.file)
    index, table, digest = table_of(args.file, args.dim)
    print(f"{len(names)} cards fingerprinted, raw width {x.shape[1]} ({len(header['probes'])} probes), "
          f"table {table.shape[1]} wide, hash {digest}")
    unit = table / np.maximum(np.linalg.norm(table, axis=1, keepdims=True), 1e-6)
    for n in args.near:
        if n not in index:
            print(f"{n}: no fingerprint")
            continue
        sims = unit @ unit[index[n]]
        order = [i for i in np.argsort(-sims) if names[i] != n][:8]
        print(f"{n}: " + ", ".join(f"{names[i]} {sims[i]:.2f}" for i in order))


if __name__ == "__main__":
    main()
