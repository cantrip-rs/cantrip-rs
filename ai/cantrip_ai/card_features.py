"""Card templates: what a card says, as abstractions cards share (`model.Config.card_features = "templates"`).

The card-text table (`embed`) reads a card as a bag of words, so "deals 3 damage" and "deals 2 damage" are two
unrelated words and nothing one card teaches the network carries to another beyond word overlap. Here a card is the
abilities `tools/maintenance/ability-index.py` already splits every card into - the index card agents use to find the
card that already implements a line, so its templates follow the engine's shared code - each one an item of the
card's bag:

- `ability|T`: a whole ability at the index's `numbers` level ("~ deals N damage to any target"), and `types|T` at its
  `types` level ("create N N/N COL SUB TYPE token");
- `keyword|K`: a keyword ability, by its keyword alone ("kicker", "ward", "flying");
- `part:KIND|T`: each part of an ability at the `numbers` level - its cost, its trigger condition, each effect
  sentence, each modal frame and each target phrase - which is where "whenever ~ attacks" or "draw N card" is the same
  item on every card that says it.

Each item carries the values its template abstracted away, in order (`ability-index.py`'s `values`): every number,
number word and mana run, one slot each, as `FEATS` numbers (`value_features`). The slot count is the largest any
item of the content has, so no value is ever left out.

A name's text is its own face's, as `embed.texts` reads it: a card's front, each back face, adventure, prepare and
omen face and half by its own name, and a split card's whole name with both halves; a card's own names read as ~.
A name with no text (a token, an emblem) has no items. The network learns an embedding per item and a map from it
and its values to the card's vector (`model.Net.card_table`); export folds that into one row per card, so nothing
here exists at inference.

    python -m cantrip_ai.card_features [--names cards.json] [--out card-features.json]   # statistics, and the file
    python -m cantrip_ai.card_features --card "Hero of Bladehold"                         # one card's items
"""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import math
import re
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np

VERSION = 1
MODEL = f"templates-{VERSION}"
REPO = Path(__file__).resolve().parents[2]
TOOLS = REPO / "tools" / "maintenance"
FACES = ("backFace", "adventure", "prepare", "omen")

# A value slot: present, the number (signed log1p), variable (X), a mana run, its mana value / 4, its W U B R G C
# symbols (a hybrid symbol shared among its colours), a tap or untap symbol, and any other symbol (energy, snow ...).
FEATS = 13
WORDS = {"a": 1, "an": 1, "one": 1, "once": 1, "two": 2, "twice": 2, "three": 3, "four": 4, "five": 5, "six": 6,
         "seven": 7, "eight": 8, "nine": 9, "ten": 10, "eleven": 11, "twelve": 12, "thirteen": 13, "fifteen": 15,
         "twenty": 20}
COLOURS = "wubrgc"


def ability_index():
    """`tools/maintenance/ability-index.py` as a module: the parser itself, never a copy of it."""
    name = "cantrip_ability_index"
    if name in sys.modules:
        return sys.modules[name]
    if str(TOOLS) not in sys.path:
        sys.path.insert(0, str(TOOLS))  # it imports scryfall beside it
    spec = importlib.util.spec_from_file_location(name, TOOLS / "ability-index.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def _symbol(sym: str, out: np.ndarray):
    """One mana symbol's contribution to a mana run's slot (without the braces, lowercase)."""
    parts = sym.split("/")
    if sym.isdigit():
        out[4] += int(sym) / 4.0
    elif sym in ("x", "y", "z"):
        out[2] = 1.0
    elif sym in ("t", "q"):
        out[11] += 1.0
    elif all(p in COLOURS or p == "p" or p.isdigit() for p in parts) and any(p in COLOURS for p in parts):
        colours = [p for p in parts if p in COLOURS]
        for p in colours:
            out[5 + COLOURS.index(p)] += 1.0 / len(colours)
        generic = [int(p) for p in parts if p.isdigit()]
        out[4] += max([1] + generic) / 4.0
    else:
        out[12] += 1.0


def value_features(value: str) -> np.ndarray:
    """A value's slot: `FEATS` numbers."""
    out = np.zeros(FEATS, np.float32)
    out[0] = 1.0
    if value.startswith("{"):
        out[3] = 1.0
        for sym in re.findall(r"\{([^}]+)\}", value):
            _symbol(sym, out)
    elif value == "x":
        out[2] = 1.0
    elif value in WORDS:
        out[1] = math.log1p(WORDS[value])
    else:
        n = int(value.replace("−", "-"))
        out[1] = math.copysign(math.log1p(abs(n)), n)
    return out


def _items(ai, text: str, names: list) -> list:
    """The items of one face's Oracle text: [(key, [values])]."""
    out = []
    for ability in ai.split_abilities(text, names):
        if ability.keyword:
            out.append((f"keyword|{ability.keyword}", ai.values(ability.text)))
            continue
        out.append((f"ability|{ai.template(ability.text, 'numbers')}", ai.values(ability.text)))
        out.append((f"types|{ai.template(ability.text, 'types')}", ai.values(ability.text)))
        for kind, part in ability.parts:
            out.append((f"part:{kind}|{ai.template(part, 'numbers')}", ai.values(part)))
    return out


def card_items(root: Path = REPO) -> dict:
    """{name: [(key, [values])]} for every name the content gives text, keyed as `embed.texts` keys them."""
    ai = ability_index()
    out = {}
    for path in sorted((root / "content").glob("*/cards/*.json")):
        try:
            text = "\n".join(line for line in path.read_text(encoding="utf-8-sig").splitlines()
                             if not line.lstrip().startswith("//"))
            card = json.loads(text)
        except (OSError, ValueError):
            continue
        if not isinstance(card, dict) or "name" not in card:
            continue
        faces = [card] + [card[k] for k in FACES if isinstance(card.get(k), dict)] + list(card.get("halves", []))
        names = [f.get("name", "") for f in faces]
        if card.get("halves"):
            out[card["name"]] = [i for h in card["halves"] for i in _items(ai, h.get("oracleText", ""), names)]
            faces = faces[1:]
        for face in faces:
            if face.get("name") and face["name"] not in out:
                out[face["name"]] = _items(ai, face.get("oracleText") or "", names)
    if not out:
        raise SystemExit(f"no card content under {root / 'content'}")
    return out


@dataclass
class Features:
    """The card index's items, flat: item `i` is `vocab[ids[i]]` on table row `rows[i]` (row `r` is `names[r - 1]`;
    row 0, the unknown card, has none), with its values `values[i]` [slots, FEATS] (zeros past its last)."""

    vocab: list
    ids: np.ndarray
    rows: np.ndarray
    values: np.ndarray
    model: str = MODEL

    @property
    def slots(self) -> int:
        return self.values.shape[1]

    def state(self) -> dict:
        """What a checkpoint keeps beside its weights (the arrays are the network's buffers)."""
        return dict(model=self.model, vocab=list(self.vocab))

    def buffers(self) -> dict:
        return dict(ids=self.ids, rows=self.rows, values=self.values, vocab=len(self.vocab))


def slot_count(items: dict) -> int:
    """The most values any item has: every item fits."""
    return max([1] + [len(v) for its in items.values() for _, v in its])


def _arrays(names: list, items: dict, index: dict, slots: int, first_row: int = 0):
    ids, rows, values = [], [], []
    for r in range(first_row, len(names)):
        for key, vals in items.get(names[r], []):
            ids.append(index[key])
            rows.append(r + 1)
            v = np.zeros((slots, FEATS), np.float32)
            for k, value in enumerate(vals):
                v[k] = value_features(value)
            values.append(v)
    return (np.array(ids, np.int64), np.array(rows, np.int64),
            np.stack(values) if values else np.zeros((0, slots, FEATS), np.float32))


def build(names: list, items: dict | None = None) -> Features:
    """The features of the index `names` for a fresh network: its items' keys, sorted, are the vocabulary."""
    items = card_items() if items is None else items
    vocab = sorted({key for n in names for key, _ in items.get(n, [])})
    ids, rows, values = _arrays(names, items, {k: i for i, k in enumerate(vocab)}, slot_count(items))
    return Features(vocab, ids, rows, values)


def extend(old: Features, names: list, kept: int, items: dict | None = None) -> Features:
    """`old`, built for `names[:kept]`, with the names after them: their items, each key new to the vocabulary
    appended to it. The old rows' items and values are kept exactly (values gain slots if a new item needs more); a
    shorter `names` drops the rows past it."""
    items = card_items() if items is None else items
    vocab = list(old.vocab)
    index = {k: i for i, k in enumerate(vocab)}
    start = min(kept, len(names))
    for n in names[start:]:
        for key, _ in items.get(n, []):
            if key not in index:
                index[key] = len(vocab)
                vocab.append(key)
    slots = max(old.slots, slot_count({n: items.get(n, []) for n in names[start:]}))
    ids, rows, values = _arrays(names, items, index, slots, first_row=start)
    old_values = np.pad(old.values, ((0, 0), (0, slots - old.slots), (0, 0)))
    keep = old.rows <= len(names)
    return Features(vocab, np.concatenate([old.ids[keep], ids]), np.concatenate([old.rows[keep], rows]),
                    np.concatenate([old_values[keep], values]), old.model)


def from_checkpoint(checkpoint: dict) -> Features | None:
    """A templates checkpoint's features: its vocabulary, and its network's buffers."""
    kept = checkpoint.get("card_features")
    state = checkpoint["state"]
    if not kept or "feat_ids" not in state:
        return None
    return Features(list(kept["vocab"]), state["feat_ids"].numpy(), state["feat_rows"].numpy(),
                    state["feat_values"].numpy(), kept.get("model", MODEL))


def content_hash(items: dict) -> str:
    h = hashlib.sha256()
    for name in sorted(items):
        h.update(json.dumps([name, items[name]], ensure_ascii=False).encode())
    return h.hexdigest()[:16]


def describe(names: list, items: dict) -> str:
    f = build(names, items)
    counts = np.bincount(f.ids, minlength=len(f.vocab)) if len(f.ids) else np.zeros(len(f.vocab), np.int64)
    # How many rows each key is on (a key twice on one card counts once).
    cards = np.zeros(len(f.vocab), np.int64)
    for k in range(len(f.vocab)):
        cards[k] = len(set(f.rows[f.ids == k].tolist()))
    kinds = {}
    for k, key in enumerate(f.vocab):
        kind = key.split("|", 1)[0]
        n, shared = kinds.get(kind, (0, 0))
        kinds[kind] = (n + 1, shared + int(cards[k] >= 2))
    with_items = len(set(f.rows.tolist()))
    on_shared = int(np.isin(f.ids, np.nonzero(cards >= 2)[0]).sum())
    per_row = np.bincount(f.rows, minlength=len(names) + 1)[1:]
    lines = [f"{len(names)} names, {with_items} with items; {len(f.ids)} items over {len(f.vocab)} keys, "
             f"{int((cards >= 2).sum())} of them on two names or more ({on_shared} of the items, "
             f"{on_shared / max(1, len(f.ids)):.0%}); {f.slots} value slots; at most {int(per_row.max()) if len(per_row) else 0} items a name"]
    for kind in sorted(kinds):
        n, shared = kinds[kind]
        lines.append(f"  {kind}: {n} keys, {shared} shared")
    return "\n".join(lines)


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--names", help="a cards.json ({'names': [...]}), or a decklist folder; default: every name with text")
    parser.add_argument("--card", action="append", help="print this card's items (repeatable)")
    parser.add_argument("--out", help="write the features as JSON: the vocabulary, each name's items and values")
    args = parser.parse_args()
    items = card_items()
    if args.card:
        for name in args.card:
            if name not in items:
                print(f"{name}: no text in content/")
                continue
            print(name)
            for key, values in items[name]:
                print(f"  {key}" + (f"   = {', '.join(values)}" if values else ""))
        return
    if args.names and Path(args.names).is_dir():
        names = sorted({line.split(" ", 1)[1].strip() for p in Path(args.names).glob("*.txt")
                        for line in p.read_text(encoding="utf-8").splitlines()
                        if re.match(r"^\d+ \S", line)})
    elif args.names:
        names = json.loads(Path(args.names).read_text(encoding="utf-8"))["names"]
    else:
        names = sorted(items)
    print(describe(names, items))
    if args.out:
        f = build(names, items)
        doc = dict(model=MODEL, content=content_hash(items), slots=f.slots, feats=FEATS, vocab=f.vocab,
                   names={n: [[k, v] for k, v in items.get(n, [])] for n in names})
        Path(args.out).write_text(json.dumps(doc, ensure_ascii=False, indent=0) + "\n", encoding="utf-8")
        print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
