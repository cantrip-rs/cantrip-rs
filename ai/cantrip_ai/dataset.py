"""Shards into training batches (docs/ai-player-plan.md, "Training").

`load` reads every shard of some directories through `ai/shards.py` into memory: the compact positions in one byte
array, and per sample its labels and game. The compact form is about 2 kB a sample, so generation 0's 4.95 M samples
take about 10 GB. `split` holds games out (by game, never by sample), and `batches` deals a list of samples into
batches of one token length each: samples are binned by length rounded up to `GRAIN` tokens, and a batch is padded
to its bin, not to its bucket (32/64/128), since padding is masked and a model has no positions; the exported graphs
take the buckets. `decode` turns a batch's compact positions into the model's inputs, and `cards` maps their card
rows onto the training index.

Each shard names the card index its rows are of (its header's `cards`; for a shard recorded before that, the nearest
`cards.json` above it). `load` makes one training index of them, starting from a checkpoint's (`base`, whose rows a
fine-tuned network keeps) and adding each name new to it, and keeps each shard index's map onto it (`Data.luts`).

A directory may carry a weight, `DIR:WEIGHT` (Phase 4's window of generations): below 1, that fraction of its games
is loaded (chosen by game, from the seed), so an old generation costs memory in proportion; above 1, each of its
samples is dealt that many times an epoch (the fraction by chance). A sample's policy target is its visit
distribution (`visits`, the search's root visits in self-play; the chosen option alone in imitation data), and a
self-played game's searched decisions carry the search's root value (`root`, NaN elsewhere); a root value that is null
or not finite in a shard is absent, as one that was never searched. A position's numbers are read through f16, so an
old shard can carry a number too large for it as an infinity; `decode` replaces a non-finite number with the largest
finite f16, or with zero for a NaN, so a corrupt position cannot turn a batch's loss into NaN.
"""

from __future__ import annotations

import json
import math
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np

AI = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(AI))
import shards  # noqa: E402

LAYOUT = shards.LAYOUT
CATS, PTRS, NUMS = LAYOUT["cats"], LAYOUT["ptrs"], LAYOUT["nums"]
NO_POINTER = LAYOUT["no_pointer"]
HEAD = 3 + CATS + PTRS + NUMS // 8  # a token's fixed bytes: kind, card u16, cats, pointers, the numbers' mask
GRAIN = 16
POOL_CARD = LAYOUT["kinds"].index("pool_card")
F16_MAX = float(np.finfo(np.float16).max)  # what an overflowing number is held at when a shard is decoded


def fnv1a64(data: bytes) -> str:
    h = 0xCBF29CE484222325
    for b in data:
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return f"{h:016x}"


def encoder_hash() -> str:
    """`cantrip_obs::encode::encoder_hash`: FNV-1a 64 of the layout JSON, which ai/encoding.json is."""
    return fnv1a64((AI / "encoding.json").read_bytes())


@dataclass
class Data:
    header: dict  # the shards' header values that every shard shares (schema, hashes, commit, producer, mode)
    seeds: list  # each shard's seeds
    blob: np.ndarray  # u8, every compact position end to end
    offset: np.ndarray  # i64 [N + 1]
    length: np.ndarray  # u8 [N], tokens
    legal: np.ndarray  # u8 [N], legal options
    target: np.ndarray  # u8 [N], the chosen option's token
    outcome: np.ndarray  # f32 [N]
    swing: np.ndarray  # f32 [N]
    dead: np.ndarray  # f32 [N]
    game: np.ndarray  # i32 [N]
    hand_offset: np.ndarray  # i64 [N + 1]
    hand: np.ndarray  # u32, the opponents' hands' card rows, end to end
    games: int
    visit_offset: np.ndarray  # i64 [N + 1]
    visit_token: np.ndarray  # u8, each visited option's token, end to end
    visit_prob: np.ndarray  # f32, its share of the visits
    root: np.ndarray  # f32 [N], the search's root value for the seat, NaN where none
    repeat: np.ndarray  # f32 [N], how many times a sample is dealt an epoch (its directory's weight, when above 1)
    directories: list  # (directory, weight, games, samples, nonfinite_roots)
    names: list  # the training index: names[i] is row i + 1 (`base` first, then each name new to it as the shards name it)
    index_of: np.ndarray  # u16 [N], the shard index each sample's position rows are of: `luts[index_of[r]]` maps them
    luts: list  # each shard index's rows to the training index's (u32, row 0 to 0)

    def __len__(self):
        return len(self.length)


# What every shard of a training set must share; the producer and the commit may differ (a window of generations),
# and are kept as the list of those seen. The card index may differ too: each shard names its own (`cards`), and its
# rows are mapped onto the training index (`Data.names`).
SHARED = ("format", "schema", "encoder_hash", "mode")


def index_hash(names: list) -> str:
    """`cantrip_ai::CardIndex::hash`: FNV-1a 64 over the names joined by newlines."""
    return fnv1a64("\n".join(names).encode("utf-8"))


def nearest_cards(path: Path) -> Path | None:
    """The `cards.json` nearest above `path` (its directory, then each parent): where the card index of a shard recorded
    before shards named theirs is kept, and a model directory's own."""
    for d in Path(path).resolve().parents:
        if (d / "cards.json").is_file():
            return d / "cards.json"
    return None


def shard_cards(path: Path, header: dict) -> list:
    """The card index a shard's rows are of: its header's `cards`, or, for an older shard, the nearest `cards.json`
    above it whose names are its `text_hash`."""
    if header.get("cards"):
        return list(header["cards"])
    found = nearest_cards(path)
    if found is not None:
        names = json.loads(found.read_text())["names"]
        if index_hash(names) == header.get("text_hash"):
            return names
    raise SystemExit(f"{path} names no card index (it was recorded before shards did): put the cards.json of index "
                     f"{header.get('text_hash')} beside it or in a directory above (docs/ai-training.md)")


def card_offsets(p: bytes) -> np.ndarray:
    """Where each token's card row (u16) is in a compact position."""
    length, options = p[2], p[4]
    at = 6 + (options + 7) // 8
    out = np.empty(length, np.int64)
    for t in range(length):
        out[t] = at + 1
        k = int.from_bytes(p[at + HEAD - NUMS // 8 : at + HEAD], "little").bit_count()
        at += HEAD + 2 * k
    return out


def with_rows(p: bytes, lut: np.ndarray) -> bytes:
    """The compact position `p` with its card rows mapped through `lut`."""
    data = bytearray(p)
    for at in card_offsets(p):
        row = int(lut[data[at] | (data[at + 1] << 8)])
        data[at], data[at + 1] = row & 0xFF, row >> 8
    return bytes(data)


def cards(data: "Data", rows: np.ndarray, card: np.ndarray) -> np.ndarray:
    """A decoded batch's card rows (`decode`'s `card`, its positions `rows`), mapped onto the training index."""
    out = np.empty_like(card)
    which = data.index_of[rows]
    for k in np.unique(which):
        at = which == k
        out[at] = data.luts[k][card[at]]
    return out


def parse_directory(spec: str | Path) -> tuple[str, float]:
    """`DIR` or `DIR:WEIGHT`."""
    spec = str(spec)
    head, sep, tail = spec.rpartition(":")
    if sep and head:
        try:
            return head, float(tail)
        except ValueError:
            pass
    return spec, 1.0


def _keep(seed: int, directory: int, game: int, weight: float) -> bool:
    if weight >= 1.0:
        return True
    h = (seed * 0x9E3779B97F4A7C15 + directory * 0xBF58476D1CE4E5B9 + game * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
    h ^= h >> 31
    h = (h * 0xD6E8FEB86659FD93) & 0xFFFFFFFFFFFFFFFF
    h ^= h >> 32
    return (h & 0xFFFFFFFF) / 2**32 < weight


def _roots(meta: dict) -> tuple[dict[int, float], int]:
    """A game's root values by decision, and how many were absent: null (as serde_json writes a non-finite f32) or
    otherwise not a finite number, from a shard recorded before the encoder stopped overflowing f16."""
    roots = {}
    dropped = 0
    for pair in meta.get("root_values") or []:
        try:
            decision, value = int(pair[0]), float(pair[1])
        except (TypeError, ValueError, IndexError):
            dropped += 1
            continue
        if math.isfinite(value):
            roots[decision] = value
        else:
            dropped += 1
    return roots, dropped


def load(directories: list[str | Path], max_games: int | None = None, log=print, seed: int = 0, base: list = ()) -> Data:
    """Every sample of every shard in `directories` (`DIR` or `DIR:WEIGHT`; at most `max_games` games from each), its
    card rows onto the training index: `base` (a checkpoint's names, whose rows it keeps), then each name the shards
    name that it lacks, in the order they first name it."""
    names = list(base)
    row_of = {n: i + 1 for i, n in enumerate(names)}
    luts: list[np.ndarray] = []
    lut_of: dict[str, int] = {}
    index_of: list[int] = []
    header: dict | None = None
    producers: list[str] = []
    commits: list[str] = []
    visit_offset = [0]
    visit_token: list[int] = []
    visit_prob: list[float] = []
    root: list[float] = []
    repeat: list[float] = []
    summary = []
    seeds = []
    blob = bytearray()
    offset = [0]
    cols = {k: [] for k in ("length", "legal", "target", "outcome", "swing", "dead", "game")}
    hand_offset = [0]
    hand: list[int] = []
    games = 0
    for d, spec in enumerate(directories):
        directory, weight = parse_directory(spec)
        taken = 0
        seen = 0
        dropped_roots = 0
        before = (games, len(offset) - 1)
        for path in shards.shards(directory):
            if max_games is not None and taken >= max_games:
                break
            h, gs = shards.read(path)
            theirs = shard_cards(path, h)
            key = index_hash(theirs)
            if key not in lut_of:
                for n in theirs:
                    if n not in row_of:
                        names.append(n)
                        row_of[n] = len(names)
                lut_of[key] = len(luts)
                luts.append(np.array([0] + [row_of[n] for n in theirs], np.uint32))
            which = lut_of[key]
            lut = luts[which]
            shared = {k: h.get(k) for k in SHARED}
            if header is None:
                header = shared
            elif shared != header:
                raise SystemExit(f"{path}: header {shared} differs from {header}")
            for k, into in (("producer", producers), ("commit", commits)):
                if h.get(k) not in into:
                    into.append(h.get(k))
            seeds.append(h.get("seeds"))
            for g in gs:
                if max_games is not None and taken >= max_games:
                    break
                taken += 1
                seen += 1
                if not _keep(seed, d, seen, weight):
                    continue
                roots, dropped = _roots(g.meta)
                dropped_roots += dropped
                for s in g.samples:
                    p = s.position
                    options = p[4]
                    bits = int.from_bytes(p[6 : 6 + (options + 7) // 8], "little")
                    blob += p
                    offset.append(len(blob))
                    cols["length"].append(p[2])
                    cols["legal"].append(bits.bit_count())
                    cols["target"].append(p[3] + s.chosen)
                    cols["outcome"].append(s.outcome)
                    cols["swing"].append(s.life_swing)
                    cols["dead"].append(s.dead_next_turn)
                    cols["game"].append(games)
                    hand.extend(int(lut[c]) if c < len(lut) else 0 for c in s.opponent_hand)
                    index_of.append(which)
                    hand_offset.append(len(hand))
                    visits = s.visits if s.visits else [(s.chosen, 1.0)]
                    visit_token.extend(p[3] + o for o, _ in visits)
                    visit_prob.extend(v for _, v in visits)
                    visit_offset.append(len(visit_token))
                    root.append(roots.get(s.decision, float("nan")))
                    repeat.append(max(weight, 1.0))
                games += 1
            log(f"  {path}: {games} games, {len(offset) - 1} samples")
        summary.append(dict(directory=directory, weight=weight, games=games - before[0], samples=len(offset) - 1 - before[1],
                            nonfinite_roots=dropped_roots))
    if header is None:
        raise SystemExit(f"no shards in {directories}")
    if header["schema"] != LAYOUT["schema"]:
        raise SystemExit(f"the shards are schema {header['schema']}, the encoding {LAYOUT['schema']}")
    if header["encoder_hash"] != encoder_hash():
        raise SystemExit(f"the shards' encoder hash {header['encoder_hash']} is not ai/encoding.json's {encoder_hash()}")
    header = dict(header, producer="; ".join(map(str, producers)), commit="; ".join(map(str, commits)))
    return Data(
        header=header,
        seeds=seeds,
        blob=np.frombuffer(blob, np.uint8),  # no copy: 10 GB for generation 0
        offset=np.array(offset, np.int64),
        length=np.array(cols["length"], np.uint8),
        legal=np.array(cols["legal"], np.uint8),
        target=np.array(cols["target"], np.uint8),
        outcome=np.array(cols["outcome"], np.float32),
        swing=np.array(cols["swing"], np.float32),
        dead=np.array(cols["dead"], np.float32),
        game=np.array(cols["game"], np.int32),
        hand_offset=np.array(hand_offset, np.int64),
        hand=np.array(hand, np.uint32),
        games=games,
        visit_offset=np.array(visit_offset, np.int64),
        visit_token=np.array(visit_token, np.uint8),
        visit_prob=np.array(visit_prob, np.float32),
        root=np.array(root, np.float32),
        repeat=np.array(repeat, np.float32),
        directories=summary,
        names=names,
        index_of=np.array(index_of, np.uint16),
        luts=luts,
    )


def split(data: Data, held_out: float, seed: int = 0) -> tuple[np.ndarray, np.ndarray]:
    """Sample indices for training and held out, held out by whole games."""
    rng = np.random.default_rng(seed)
    out = rng.random(data.games) < held_out
    held = out[data.game]
    return np.flatnonzero(~held), np.flatnonzero(held)


def batches(data: Data, indices: np.ndarray, batch: int, rng: np.random.Generator | None, trivial: float = 1.0) -> list[np.ndarray]:
    """`indices` dealt into batches of samples of one length bin, in a random order (`rng`), or in order (`None`).
    Samples with a single legal option are kept at the rate `trivial`: they teach the value, not the policy."""
    if rng is not None and trivial < 1.0:
        keep = (data.legal[indices] > 1) | (rng.random(len(indices)) < trivial)
        indices = indices[keep]
    if rng is not None and len(indices) and (data.repeat[indices] > 1).any():
        r = data.repeat[indices]
        times = np.floor(r).astype(np.int64) + (rng.random(len(indices)) < (r - np.floor(r)))
        indices = np.repeat(indices, times)
    if rng is not None:
        indices = indices[rng.permutation(len(indices))]
    bins = (data.length[indices].astype(np.int64) + GRAIN - 1) // GRAIN
    out = []
    for b in np.unique(bins):
        members = indices[bins == b]
        out.extend(members[i : i + batch] for i in range(0, len(members), batch))
    if rng is not None:
        out = [out[i] for i in rng.permutation(len(out))]
    return out


def decode(blob: np.ndarray, offset: np.ndarray, rows: np.ndarray, tokens: int | None = None,
           sanitize: bool = True) -> dict[str, np.ndarray]:
    """The compact positions `rows` as the model's inputs, padded to `tokens` (default: the longest, rounded up to
    `GRAIN`): kind, card, cats, ptrs (i64), nums (f32), legal (bool). The inverse of `cantrip_obs::encode::to_compact`,
    as `shards.position` is, for a whole batch at once. With `sanitize`, a non-finite number is held at the largest
    finite f16 (zero for a NaN), which an old shard may need: an overflowing feature was written as an infinity."""
    raw = [blob[offset[r] : offset[r + 1]].tobytes() for r in rows]
    if tokens is None:
        tokens = max(-(-max(p[2] for p in raw) // GRAIN) * GRAIN, GRAIN)
    n = len(rows)
    kind = np.zeros((n, tokens), np.int64)
    card = np.zeros((n, tokens), np.int64)
    cats = np.zeros((n, tokens, CATS), np.int64)
    ptrs = np.full((n, tokens, PTRS), NO_POINTER, np.int64)
    nums = np.zeros((n, tokens, NUMS), np.float32)
    legal = np.zeros((n, tokens), bool)
    lane = np.arange(HEAD)
    for i, p in enumerate(raw):
        length, first, options = p[2], p[3], p[4]
        if length > tokens:
            raise ValueError(f"a position of {length} tokens in {tokens}")
        at = 6 + (options + 7) // 8
        bits = np.unpackbits(np.frombuffer(p, np.uint8, (options + 7) // 8, 6), bitorder="little")[:options]
        legal[i, first : first + options] = bits.astype(bool)
        starts = np.empty(length, np.int64)
        counts = np.empty(length, np.int64)
        for t in range(length):
            starts[t] = at
            k = int.from_bytes(p[at + HEAD - NUMS // 8 : at + HEAD], "little").bit_count()
            counts[t] = k
            at += HEAD + 2 * k
        if at != len(p):
            raise ValueError("a position with bytes left over")
        data = np.frombuffer(p, np.uint8)
        head = data[starts[:, None] + lane]
        kind[i, :length] = head[:, 0]
        card[i, :length] = head[:, 1].astype(np.int64) | (head[:, 2].astype(np.int64) << 8)
        cats[i, :length] = head[:, 3 : 3 + CATS]
        ptrs[i, :length] = head[:, 3 + CATS : 3 + CATS + PTRS]
        mask = np.unpackbits(head[:, 3 + CATS + PTRS :], axis=1, bitorder="little").astype(bool)
        total = int(counts.sum())
        if total:
            first_value = np.repeat(starts + HEAD, counts)
            within = np.arange(total) - np.repeat(np.cumsum(counts) - counts, counts)
            at_value = first_value + 2 * within
            halves = (data[at_value].astype(np.uint16) | (data[at_value + 1].astype(np.uint16) << 8)).view(np.float16)
            block = np.zeros((length, NUMS), np.float32)
            block[mask] = halves.astype(np.float32)
            nums[i, :length] = block
    if sanitize:
        nums = np.nan_to_num(nums, nan=0.0, posinf=F16_MAX, neginf=-F16_MAX)
    return dict(kind=kind, card=card, cats=cats, ptrs=ptrs, nums=nums, legal=legal)


def labels(data: Data, rows: np.ndarray, inputs: dict[str, np.ndarray]) -> dict[str, np.ndarray]:
    """The batch's targets: the chosen option's token, the outcome, the life swing, dead next turn, whether the
    position has more than one legal option, and per token whether it is a pool card in an opponent's hand."""
    hand_card = np.zeros_like(inputs["card"], dtype=np.float32)
    for i, r in enumerate(rows):
        hand = data.hand[data.hand_offset[r] : data.hand_offset[r + 1]]
        if len(hand):
            hand_card[i] = np.isin(inputs["card"][i], hand) & (inputs["kind"][i] == POOL_CARD)
    visits = np.zeros(inputs["card"].shape, np.float32)
    for i, r in enumerate(rows):
        a, b = data.visit_offset[r], data.visit_offset[r + 1]
        visits[i, data.visit_token[a:b]] = data.visit_prob[a:b]
    return dict(
        target=data.target[rows].astype(np.int64),
        visits=visits,
        root=data.root[rows],
        outcome=data.outcome[rows],
        swing=data.swing[rows],
        dead=data.dead[rows],
        choice=(data.legal[rows] > 1),
        in_hand=hand_card,
        pool=(inputs["kind"] == POOL_CARD),
    )


def describe(data: Data) -> str:
    n = len(data)
    trivial = float((data.legal <= 1).mean())
    spread = float((np.diff(data.visit_offset) > 1).mean()) if n else 0.0
    rooted = float((~np.isnan(data.root)).mean()) if n else 0.0
    return (f"{data.games} games, {n} samples, {len(data.blob) / max(n, 1):.0f} bytes a position, "
            f"{trivial:.1%} with one legal option, {spread:.1%} with a visit distribution over several options, "
            f"{rooted:.1%} with a root value, mode {data.header['mode']}")


if __name__ == "__main__":
    d = load(sys.argv[1:], max_games=20)
    print(describe(d))
    print(json.dumps(d.header))
