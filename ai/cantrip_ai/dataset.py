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

`drop_damage_before` leaves out the combat damage assignments of a shard written before a time: their `Damage`
options meant one point of damage until the answer was factored a blocker at a time (`cantrip_obs::factor`, commit
c50710e01), and a network must not learn the old meaning. A sample is told by its game token's ask (`ask_of`), read
straight from the compact bytes.

Per sample, `load` also keeps where its policy target came from (`searched`, one of `NOT_SEARCHED`, `SEARCHED` and
`RECORDED`), whether its decision was a fast search of playout-cap randomisation (`fast`), the search's root value of
each option it measured (`q_offset`, `q_token`, `q_value`, ragged as the visits are), and whether its seat was the
network being trained (`learner`). The shard reader (`ai/shards.py`) fills the first and the third from shard format 2
(crates/cantrip-ai/src/shard.rs: a flags byte per sample, and a root value beside each visit), as two optional
attributes of `shards.Sample`; a format 1 shard says neither:

- `Sample.searched` (bool, or absent / None where the shard does not record it): whether this sub-step's visits are a
  search's root visits. Absent, the decision decides: a sub-step of a decision that has a root value counts as
  searched (the fallback, which takes a sub-step the search never reached for a searched one, review 2026-11-13 item 1).
  A game with no root values at all is `RECORDED` (imitation: the recorded player's answer) whatever the flag says.
- `Sample.option_q` (a list of `(option index, value)`, or absent / empty): the root's mean value of each option the
  search measured, for the seat answering, in the value's units (-1 to 1); a non-finite value is left out. `Data`
  keeps the option's token, as it keeps the visits'.

`learner` (u8 [N]) is 1 where the sample's seat was played by the network being trained, 0 where an earlier champion
played it (`GameMeta::learners`, self-play's `--opponents`; a game without the list was the learner's in every seat).
An earlier champion's decisions are recorded one-hot with no root value, and are no policy target (`policy` 0).

A window may mix the two formats: `format` is not among the header values every shard must share, and the data's
header names the newest.

A sample's noted heuristic is its game's `heuristic_values` by decision (any game's, self-played ones included), NaN
where the game notes none for its decision. Only a recording table notes one (the tactical model's log-odds; a
self-played position would need a Rust port of that model to carry one).

A reanalysed game (`GameMeta::reanalysed`, `cantrip-reanalyse`) holds chosen positions of a game already in a
self-play shard, searched again: its samples (`Data.reanalysed`) are policy targets alone, their value, auxiliary,
belief and heuristic losses skipped as a policy-only copy's are (`unmark`), since the source game's samples carry
those already; and `split` holds a reanalysed game out with its source (`Data.game_key`: the game's seed, start, decks
and names, all four of which a reanalysed game keeps), so a held-out game's positions never train through its copy.

`load` reads the shards in spawned processes (`workers`, a shard each at a time; `_read_shard`), and puts what they
read together in order here, so the data is the same as one process reading them would make: the training index, the
games' numbers and the thinning of single-option positions are settled in order, and a directory whose games are
chosen by their count (a weight below 1, `max_games`) has its shards counted first (`_count_games`), each read from
its place in the count. A game passed over (another deck, not chosen by the weight) is skipped unread (`shards.scan`):
most of an imitation corpus's games, at a fraction of the cost of reading them.
"""

from __future__ import annotations

import atexit
import functools
import json
import math
import multiprocessing
import os
import sys
import tempfile
from collections import deque
from dataclasses import dataclass, field
from multiprocessing import shared_memory
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
GAME = LAYOUT["kinds"].index("game")
ASK_SLOT = next(c["slot"] for c in LAYOUT["categorical"] if c["name"] == "ask")
DAMAGE_ASK = 1 + LAYOUT["ask_kinds"].index("AssignCombatDamage")  # the game token's ask categorical: 1 + the kind's index
# `Data.searched`: a self-played sub-step no search answered (the network's own answer), one whose visits are a
# search's, and a sample of a game with no search in it (imitation).
NOT_SEARCHED, SEARCHED, RECORDED = 0, 1, 2


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
    seat: np.ndarray  # u8 [N], the answering seat
    decision: np.ndarray  # u32 [N], the answer's place on the game's line (a decision's sub-steps share it)
    turn: np.ndarray  # u16 [N]
    hand_offset: np.ndarray  # i64 [N + 1]
    hand: np.ndarray  # u32, the opponents' hands' card rows, end to end
    games: int
    visit_offset: np.ndarray  # i64 [N + 1]
    visit_token: np.ndarray  # u8, each visited option's token, end to end
    visit_prob: np.ndarray  # f32, its share of the visits
    root: np.ndarray  # f32 [N], the search's root value for the seat, NaN where none
    heuristic: np.ndarray  # f32 [N], a noted heuristic's log-odds of winning for the seat (the tactical model), NaN where none
    policy: np.ndarray  # u8 [N], whether the sample's visits are a policy target (0: a fast search of playout-cap randomisation)
    distill: np.ndarray  # f32 [N], a perfect-information value's mean over the seat's worlds (`distill_values`), NaN where none
    learner: np.ndarray  # u8 [N], 1 where the seat was the network being trained, 0 an earlier champion (`--opponents`)
    repeat: np.ndarray  # f32 [N], how many times a sample is dealt an epoch (its directory's weight, when above 1)
    directories: list  # (directory, weight, games, samples, nonfinite_roots, dropped_damage, per_sample_searched)
    names: list  # the training index: names[i] is row i + 1 (`base` first, then each name new to it as the shards name it)
    index_of: np.ndarray  # u16 [N], the shard index each sample's position rows are of: `luts[index_of[r]]` maps them
    luts: list  # each shard index's rows to the training index's (u32, row 0 to 0)
    searched: np.ndarray  # u8 [N], where the policy target came from: NOT_SEARCHED, SEARCHED or RECORDED (module docstring)
    fast: np.ndarray  # u8 [N], whether the sample's decision was a fast search (`GameMeta::fast_decisions`)
    q_offset: np.ndarray  # i64 [N + 1], each sample's measured options in `q_token` and `q_value`
    q_token: np.ndarray  # u8, each measured option's token, end to end
    q_value: np.ndarray  # f32, its root mean value for the seat (-1 to 1)
    reanalysed: np.ndarray | None = None  # u8 [N], 1 where the sample's game is reanalysed (`GameMeta::reanalysed`)
    game_key: np.ndarray | None = None  # u64 [games], each game's `source_key`: a reanalysed game's is its source's
    game_reanalysed: np.ndarray | None = None  # u8 [games], 1 for a reanalysed game
    blocks: dict = field(default_factory=dict)  # field: the shared memory its array is in, once `share`d
    mapped: dict = field(default_factory=dict)  # field: the file its array is a read-only map of (`load(blob_file=True)`)

    def __len__(self):
        return len(self.length)

    def __getstate__(self):
        # Shared, each array goes by its block's name, shape and type: a spawned worker attaches to it, never copies it.
        state = dict(self.__dict__)
        for k, block in self.blocks.items():
            state[k] = _Block(block.name, state[k].dtype.str, state[k].shape)
        for k, path in self.mapped.items():
            state[k] = _Mapped(str(path), state[k].dtype.str, state[k].shape)
        state["blocks"] = {}
        return state

    def __setstate__(self, state):
        blocks = {}
        for k, v in state.items():
            if isinstance(v, _Block):
                blocks[k] = _attach(v.name)
                state[k] = np.ndarray(v.shape, np.dtype(v.dtype), blocks[k].buf)
            elif isinstance(v, _Mapped):
                state[k] = np.memmap(v.path, np.dtype(v.dtype), "r", shape=v.shape)
        state["blocks"] = blocks
        self.__dict__.update(state)


@dataclass(frozen=True)
class _Block:
    name: str
    dtype: str
    shape: tuple


@dataclass(frozen=True)
class _Mapped:
    path: str
    dtype: str
    shape: tuple


def share(data: Data) -> Data:
    """`data`'s arrays moved into named shared memory, for DataLoader workers that are spawned rather than forked
    (Windows, macOS): a spawned worker is handed `data` by pickling, which then carries each block's name instead of
    its bytes, so every worker reads the one copy rather than holding its own (generation 0's is 14 GB). A forked
    worker shares the parent's memory already, and needs none of this. Each array is copied once and the original
    dropped; the blocks live while this process holds them, and are released at its exit."""
    for k in list(data.__dict__):
        v = getattr(data, k)
        if k in data.blocks or k in data.mapped or not isinstance(v, np.ndarray) or not v.nbytes:
            continue
        block = shared_memory.SharedMemory(create=True, size=v.nbytes)
        view = np.ndarray(v.shape, v.dtype, block.buf)
        view[...] = v
        data.blocks[k] = block
        setattr(data, k, view)
        del v, view
    atexit.register(_release, list(data.blocks.values()))
    return data


def _attach(name: str) -> shared_memory.SharedMemory:
    block = shared_memory.SharedMemory(name=name)
    if sys.platform != "win32":
        # Python before 3.13 has an attaching process's resource tracker unlink the block when that process exits,
        # which would take it from the trainer and the other workers: only `share`'s process releases it.
        from multiprocessing import resource_tracker
        resource_tracker.unregister(block._name, "shared_memory")
    return block


def _release(blocks):
    for b in blocks:
        try:
            b.unlink()  # POSIX; on Windows a block goes when its last handle closes
        except (FileNotFoundError, OSError):
            pass


# What every shard of a training set must share; the producer and the commit may differ (a window of generations),
# and are kept as the list of those seen, and so may the format (each shard is read in its own; the newest is kept).
# The card index may differ too: each shard names its own (`cards`), and its rows are mapped onto the training index
# (`Data.names`).
SHARED = ("schema", "encoder_hash", "mode")
# Shards of an earlier encoding read as this one: (schema, encoder hash) of encodings this one only added to. Schema 1
# (the fp line's gen0-gen7) reads as schema 4 with nothing new in it (`model.SCHEMA1_HASH`).
UPGRADABLE = {(1, "6fed5023a1c55ffc"): 4}


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


def ask_of(p: bytes) -> int:
    """A compact position's ask, as its game token's ask categorical holds it (1 + the kind's index in `ask_kinds`, 0
    for none), without decoding it: the game is the position's first token (`cantrip_obs::encode::encode` pushes it
    before any other), so its categoricals are at a fixed place. 0 for a position whose first token is not the game."""
    if p[2] == 0:
        return 0
    at = 6 + (p[4] + 7) // 8
    return p[at + 3 + ASK_SLOT] if p[at] == GAME else 0


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


def _noted(meta: dict) -> dict[int, float]:
    """A game's noted heuristic values by decision (`heuristic_values`: the tactical model's log-odds of winning for
    the seat answering, where a recording table noted them); empty for a game with none."""
    out = {}
    for pair in meta.get("heuristic_values") or []:
        try:
            decision, value = int(pair[0]), float(pair[1])
        except (TypeError, ValueError, IndexError):
            continue
        if math.isfinite(value):
            out[decision] = value
    return out


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


def chunk_paths(directories: list[str | Path], part: int, parts: int) -> set[Path]:
    """The shard files of chunk `part` of `parts`: every `parts`-th of all the directories' shards, in order."""
    every = [p for spec in directories for p in shards.shards(parse_directory(spec)[0])]
    return {p for i, p in enumerate(every) if i % parts == part}


# A shard reader's per-sample columns (`_read_shard`), in the types `Data` keeps them in; `pos_len`, `hand_count` and
# `visit_count` are how much of the blob, the hands and the visits each sample has.
_COLUMNS = dict(length=np.uint8, legal=np.uint8, target=np.uint8, outcome=np.float32, swing=np.float32, dead=np.float32,
                seat=np.uint8, decision=np.uint32, turn=np.uint16, root=np.float32, heuristic=np.float32, policy=np.uint8,
                distill=np.float32, searched=np.uint8, fast=np.uint8, learner=np.uint8, reanalysed=np.uint8,
                pos_len=np.int64,
                hand_count=np.int64, visit_count=np.int64, q_count=np.int64)


@dataclass
class _Shard:
    """A shard as `_read_shard` reads it: the samples of the games it keeps, in the shard's own card rows, and what
    `load`'s bookkeeping in order needs of it. Its single-option positions are all there: `trivial` thins them by the
    game's number in the whole load, which only `load` knows."""
    path: Path
    header: dict | None = None  # its header but its card index
    cards: list | None = None  # its card index (`shard_cards`)
    key: str = ""  # the card index's hash (`index_hash`)
    error: BaseException | None = None  # what reading it raised: `load` raises it where a read in order would have
    taken: int = 0  # the directory's games among the decks, counted to the end of this shard
    nonfinite_roots: int = 0
    dropped_damage: int = 0
    flagged: int = 0  # self-played samples whose reader recorded `Sample.searched`
    game_samples: np.ndarray | None = None  # i64, each kept game's samples
    game_key: np.ndarray | None = None  # u64, each kept game's `source_key`
    game_reanalysed: np.ndarray | None = None  # u8, whether each kept game is reanalysed
    game_decks: list | None = None  # each kept game's decks, when asked for
    cols: dict | None = None  # `_COLUMNS`, a value a sample
    blob: bytes = b""
    hand: np.ndarray | None = None  # u32, the opponents' hands' rows in the shard's own index
    visit_token: np.ndarray | None = None
    visit_prob: np.ndarray | None = None
    q_token: np.ndarray | None = None
    q_value: np.ndarray | None = None
    format: int = 1  # the shard's format


def _learners(meta: dict) -> set[int] | None:
    """The seats (from 0) the network being trained played (`GameMeta::learners`, from 1), or None: every seat."""
    seats = meta.get("learners")
    if not seats:
        return None
    return {int(x) - 1 for x in seats}


def source_key(meta: dict) -> int:
    """The game a game's samples came from, as a 64-bit hash of its seed, its start (a scenario's name, empty for a dealt
    game), its decks and its seats' names: what a reanalysed game (`cantrip-reanalyse`, which keeps all four of its
    source's) shares with the self-played game it searched again."""
    who = [meta.get("seed"), meta.get("start") or "", list(meta.get("decks") or []), list(meta.get("names") or [])]
    return int(fnv1a64(json.dumps(who).encode("utf-8")), 16)


def _searched(meta: dict) -> set[int] | None:
    """The decisions a self-played game's seats searched (each has a root value, finite or not), or None for a game
    with no root values at all (imitation: every answer is the recorded player's)."""
    out = set()
    for pair in meta.get("root_values") or []:
        try:
            out.add(int(pair[0]))
        except (TypeError, ValueError, IndexError):
            continue
    return out or None


def _read_shard(path: Path, d: int, weight: float, taken: int, *, seed: int, decks: set[str] | None,
                max_games: int | None, drop_damage_before: float | None, want_decks: bool,
                searched_policy: bool = False, fast_policy: bool = False) -> _Shard:
    """Shard `path` of directory `d`, `taken` of the directory's games among the decks before it: the games `load`
    keeps of it, read as `load` reads them. Module level, so a spawned process can be handed it."""
    out = _Shard(path)
    gs = None
    try:
        h, gs = shards.scan(path)
        out.cards = shard_cards(path, h)
        out.key = index_hash(out.cards)
        out.header = {k: v for k, v in h.items() if k != "cards"}
        if UPGRADABLE.get((out.header.get("schema"), out.header.get("encoder_hash"))) == LAYOUT["schema"]:
            out.header = dict(out.header, schema=LAYOUT["schema"], encoder_hash=encoder_hash(),
                              upgraded_from=f'{out.header["schema"]}/{out.header["encoder_hash"]}')
        # The file's own time: a corpus directory may be a junction to another place.
        stale = drop_damage_before is not None and os.path.getmtime(path) < drop_damage_before
        cols = {k: [] for k in _COLUMNS}
        out.format = int(h.get("format", 1))
        blob, hand, visit_token, visit_prob, game_samples, game_decks = [], [], [], [], [], []
        q_token, q_value = [], []
        game_key, game_reanalysed = [], []
        # Each column's append, bound once: a sample is a few microseconds, and a self-played shard all kept.
        (length, legal, target, outcome, swing, dead, seat, decision_add, turn, root, heuristic, policy, distill,
         searched_add, fast_add, learner_add, reanalysed_add, pos_len, hand_count, visit_count, q_count) = (
            cols[k].append for k in _COLUMNS)
        blob_add, hand_add, token_add, prob_add = blob.append, hand.extend, visit_token.extend, visit_prob.extend
        q_token_add, q_value_add = q_token.extend, q_value.extend
        nan = float("nan")
        isfinite = math.isfinite
        for g in gs:
            if max_games is not None and taken >= max_games:
                break
            if decks is not None and not set(g.meta.get("decks") or ["?"]) <= decks:
                continue  # its samples are skipped unread
            taken += 1
            if not _keep(seed, d, taken, weight):
                continue
            roots, dropped = _roots(g.meta)
            noted = _noted(g.meta)
            # Playout-cap randomisation's fast searches: value targets, no policy target (`GameMeta::fast_decisions`).
            fast = set(int(d) for d in (g.meta.get("fast_decisions") or []))
            # `fast_policy`: a fast search whose target is Gumbel's (`gumbel_decisions`) is a policy target too.
            gumbel = set(int(d) for d in (g.meta.get("gumbel_decisions") or [])) if fast_policy else set()
            # The decisions a search answered (None: a game with no search, imitation). Under `searched_policy` a
            # self-played sub-step no search answered (the network's own answer: the priority shortcut, a payment, a
            # sub-step the search never reached where the reader says so) is no policy target either.
            searched = _searched(g.meta)
            # Against an earlier champion (`GameMeta::learners`) its decisions are no policy target: only the learner's.
            learners = _learners(g.meta)
            distilled = {int(d): float(v) for d, v in (g.meta.get("distill_values") or [])}
            # A reanalysed game's samples are policy targets alone (`unmark`).
            again = bool(g.meta.get("reanalysed"))
            out.nonfinite_roots += dropped
            samples = g.samples
            if stale:
                samples = [s for s in samples if ask_of(s.position) != DAMAGE_ASK]
                out.dropped_damage += len(g.samples) - len(samples)
            for s in samples:
                p = s.position
                bits = int.from_bytes(p[6 : 6 + (p[4] + 7) // 8], "little")
                visits = s.visits if s.visits else [(s.chosen, 1.0)]
                decision = s.decision
                mine = learners is None or s.seat in learners
                blob_add(p)
                pos_len(len(p))
                length(p[2])
                legal(bits.bit_count())
                target(p[3] + s.chosen)
                outcome(s.outcome)
                swing(s.life_swing)
                dead(s.dead_next_turn)
                seat(s.seat)
                decision_add(decision)
                turn(s.turn)
                hand_add(s.opponent_hand)
                hand_count(len(s.opponent_hand))
                token_add(p[3] + o for o, _ in visits)
                prob_add(v for _, v in visits)
                visit_count(len(visits))
                root(roots.get(decision, nan))
                heuristic(noted.get(decision, nan))
                if searched is None:
                    where = RECORDED
                else:
                    flag = getattr(s, "searched", None)
                    if flag is None:
                        where = SEARCHED if decision in searched else NOT_SEARCHED
                    else:
                        where = SEARCHED if flag else NOT_SEARCHED
                        out.flagged += 1
                searched_add(where)
                fast_add(decision in fast)
                learner_add(mine)
                reanalysed_add(again)
                policy(0 if (decision in fast and decision not in gumbel) or (searched_policy and where == NOT_SEARCHED)
                       or not mine else 1)
                distill(distilled.get(decision, nan))
                qs = [(p[3] + o, v) for o, v in (getattr(s, "option_q", None) or ()) if isfinite(v)]
                q_token_add(t for t, _ in qs)
                q_value_add(v for _, v in qs)
                q_count(len(qs))
            game_samples.append(len(samples))
            game_key.append(source_key(g.meta))
            game_reanalysed.append(again)
            if want_decks:
                game_decks.append(list(g.meta.get("decks") or []))
        out.taken = taken
        out.cols = {k: np.array(v, _COLUMNS[k]) for k, v in cols.items()}
        out.blob = b"".join(blob)
        out.hand = np.array(hand, np.uint32)
        out.visit_token = np.array(visit_token, np.uint8)
        out.visit_prob = np.array(visit_prob, np.float32)
        out.q_token = np.array(q_token, np.uint8)
        out.q_value = np.array(q_value, np.float32)
        out.game_samples = np.array(game_samples, np.int64)
        out.game_key = np.array(game_key, np.uint64)
        out.game_reanalysed = np.array(game_reanalysed, np.uint8)
        out.game_decks = game_decks if want_decks else None
    except (Exception, SystemExit) as e:
        out.error = e
    finally:
        if gs is not None:
            gs.close()  # a shard left at `max_games` is closed now, not when collected
    return out


def _count_games(path: Path, decks: set[str] | None) -> int | BaseException:
    """How many of a shard's games are among `decks` (every game, for none), their samples skipped unread."""
    try:
        _, gs = shards.scan(path)
        return sum(1 for g in gs if decks is None or set(g.meta.get("decks") or ["?"]) <= decks)
    except (Exception, SystemExit) as e:
        return e


def _shard_results(plan: list, read, max_games: int | None, decks: set[str] | None, workers: int):
    """Each directory's shards read (`read`, a `_read_shard`), in order: a `_Shard` each, then None at each directory's
    end. With `workers` above 1 they are read by that many spawned processes, at most two each ahead of the one wanted.
    A directory whose games are counted in order (a weight below 1, or `max_games`) has each shard's start in that
    count summed from the shards before it, which are counted first (`_count_games`), in parallel as well."""
    if workers <= 1:
        for d, _, weight, paths in plan:
            taken = 0
            for path in paths:
                if max_games is not None and taken >= max_games:
                    break
                r = read(path, d, weight, taken)
                yield r
                taken = r.taken
            yield None
        return
    with multiprocessing.get_context("spawn").Pool(workers) as pool:
        counts = {(d, i): pool.apply_async(_count_games, (path, decks))
                  for d, _, weight, paths in plan if weight < 1.0 or max_games is not None
                  for i, path in enumerate(paths[:-1])}

        def jobs():
            for d, _, weight, paths in plan:
                taken = 0
                for i, path in enumerate(paths):
                    if max_games is not None and taken >= max_games:
                        break
                    yield (path, d, weight, taken)
                    if (d, i) in counts:
                        c = counts[(d, i)].get()
                        if isinstance(c, BaseException):
                            yield c  # the shard's own read, before it, fails in any case
                            break
                        taken += c
                yield None

        pending = deque()
        reading = 0
        todo = jobs()
        more = True
        while True:
            while more and reading < 2 * workers:
                job = next(todo, StopIteration)
                if job is StopIteration:
                    more = False
                elif isinstance(job, tuple):
                    pending.append(pool.apply_async(read, job))
                    reading += 1
                else:
                    pending.append(job)
            if not pending:
                return
            item = pending.popleft()
            if isinstance(item, BaseException):
                raise item
            if item is None:
                yield None
            else:
                reading -= 1
                yield item.get()


def _ragged(counts: np.ndarray, keep: np.ndarray) -> np.ndarray:
    """The indices of the elements of the kept runs (`keep`) of runs `counts` long, laid end to end."""
    starts = np.cumsum(counts) - counts
    kept = counts[keep]
    return np.repeat(starts[keep] - (np.cumsum(kept) - kept), kept) + np.arange(int(kept.sum()), dtype=np.int64)


def _joined(spill, size: int) -> np.ndarray:
    """The positions `load` spilled (`size` bytes) read back into one array, a piece at a time, and the file closed: one
    copy of the blob (tens of GB for a window of generations)."""
    out = np.empty(size, np.uint8)
    spill.seek(0)
    view = memoryview(out)
    at = 0
    with spill:
        while at < size:
            got = spill.readinto(view[at : at + (1 << 30)])
            if not got:
                raise OSError(f"the spilled positions end at {at} of {size} bytes")
            at += got
    return out


def spill_dir() -> Path:
    """Where `load(blob_file=True)` keeps its position files: `CANTRIP_SPILL_DIR`, or a folder in the temporary
    directory."""
    d = Path(os.environ.get("CANTRIP_SPILL_DIR") or Path(tempfile.gettempdir()) / "cantrip-spill")
    d.mkdir(parents=True, exist_ok=True)
    return d


def _remove(path: Path):
    try:
        path.unlink(missing_ok=True)
    except OSError:
        pass  # still mapped here (Windows): the next load removes it


def _remove_stale_spills():
    """Position files a finished process left (a file still mapped by a live one cannot be removed, and stays)."""
    for f in spill_dir().glob("blob-*.u8"):
        _remove(f)


def _cat(parts: list, dtype) -> np.ndarray:
    return np.concatenate(parts).astype(dtype, copy=False) if parts else np.zeros(0, dtype)


def default_workers() -> int:
    """`load`'s processes reading shards when it is not told."""
    return min(16, os.cpu_count() or 1)


def load(directories: list[str | Path], max_games: int | None = None, log=print, seed: int = 0, base: list = (),
         chunk: tuple[int, int] | None = None, trivial: float = 1.0, decks: set[str] | None = None,
         game_decks: list | None = None, drop_damage_before: float | None = None, workers: int | None = None,
         blob_file: bool = False, searched_policy: bool = False, fast_policy: bool = False) -> Data:
    """Every sample of every shard in `directories` (`DIR` or `DIR:WEIGHT`; at most `max_games` games from each), its
    card rows onto the training index: `base` (a checkpoint's names, whose rows it keeps), then each name the shards
    name that it lacks, in the order they first name it.

    `chunk=(part, parts)`: only the shards of that chunk (`chunk_paths`), for a training set larger than memory; the
    training index is still every shard's names, read from their headers, so it is the same in every chunk.
    `blob_file`: the blob is a read-only map of a file in `spill_dir()` instead of an array in memory, and a spawned
    worker maps the same file (by its path) instead of being handed a copy. It takes no commit charge (the file cache
    keeps it in memory), and Windows maps a file of any size, where it refuses a pagefile-backed shared block of tens
    of GB once the machine has been up a while (WinError 1450, gen2 of the templates line, 28 to 35 GB). The file is
    removed at exit, or by the next such load if this process could not.
    `trivial`: the rate a single-option position is kept at (by a hash of its game and place, so a chunk loads the same
    positions each time); training keeps them at a rate in any case (`batches`), and dropping them here saves memory.
    `decks`: only the games whose decks (`GameMeta::decks`) are all among these names, a subset of a corpus recorded
    over more decks; a game it passes over does not count towards `max_games` or a directory's weight.
    `game_decks`: a list each loaded game's decks (`GameMeta::decks`, by seat) are appended to, in game order.
    `drop_damage_before` (POSIX seconds): a shard file modified before it loses its combat damage assignments, as if
    they had never been recorded (its games stay); a directory's `dropped_damage` counts them.
    `searched_policy`: in a self-played game (one with root values), only the decisions a search answered are policy
    targets; the others (the priority shortcut, payments, sub-steps the search never reached: the network's own answer,
    90.6% of a gen3 self-play shard's policy targets in the templates line) keep their value and aux targets. A game
    with no root values (imitation) is unchanged.
    `fast_policy`: a fast search's decisions (playout-cap randomisation's `fast_decisions`) whose root target is
    Gumbel's (`gumbel_decisions`) are policy targets too, as the full searches' are: Gumbel's target improves on the
    prior at any budget, where a fast PUCT search's visit counts mostly echo it (so older self-play is unchanged). They
    stay marked `fast` (the value's mix by budget).
    `workers`: how many processes read the shards (`default_workers()` for None; 0 or 1, this one alone). The data is
    the same whatever their number: what depends on the order (the training index, the games' numbers, a weight's
    choice of games, `max_games`, `trivial`) is settled here, in order. A game passed over is skipped unread.
    Games are numbered within what is loaded."""
    names = list(base)
    row_of = {n: i + 1 for i, n in enumerate(names)}
    picked = chunk_paths(directories, *chunk) if chunk is not None else None
    if picked is not None:
        for spec in directories:
            for path in shards.shards(parse_directory(spec)[0]):
                for n in shard_cards(path, shards.header(path)):
                    if n not in row_of:
                        names.append(n)
                        row_of[n] = len(names)
    plan = []
    for d, spec in enumerate(directories):
        directory, weight = parse_directory(spec)
        plan.append((d, directory, weight, [p for p in shards.shards(directory) if picked is None or p in picked]))
    workers = min(default_workers() if workers is None else workers, sum(len(paths) for *_, paths in plan))
    read = functools.partial(_read_shard, seed=seed, decks=decks, max_games=max_games,
                             drop_damage_before=drop_damage_before, want_decks=game_decks is not None,
                             searched_policy=searched_policy, fast_policy=fast_policy)
    results = _shard_results(plan, read, max_games, decks, workers)
    luts: list[np.ndarray] = []
    lut_of: dict[str, int] = {}
    header: dict | None = None
    producers: list[str] = []
    commits: list[str] = []
    summary = []
    seeds = []
    # Each shard's positions, spilled to a file as they come and read back into one array at the end: held in memory
    # beside that array, they would be the blob twice (gen2 of the templates line: 35 GB, and a 106 GB peak).
    if blob_file:
        _remove_stale_spills()
        blob = tempfile.NamedTemporaryFile(prefix="blob-", suffix=".u8", dir=spill_dir(), delete=False)
    else:
        blob = tempfile.TemporaryFile(prefix="cantrip-blob-")
    blob_size = 0
    parts = {k: [] for k in (*_COLUMNS, "game", "hand", "visit_token", "visit_prob", "q_token", "q_value", "index_of",
                             "repeat", "game_key", "game_reanalysed")}
    formats: list[int] = []
    games = 0
    samples = 0
    for d, directory, weight, _ in plan:
        dropped_roots = 0
        dropped_damage = 0
        flagged = 0
        before = (games, samples)
        while (r := next(results)) is not None:
            if r.error is not None and r.header is None:
                raise r.error
            theirs = r.cards
            key = r.key
            if key not in lut_of:
                for n in theirs:
                    if n not in row_of:
                        names.append(n)
                        row_of[n] = len(names)
                lut_of[key] = len(luts)
                luts.append(np.array([0] + [row_of[n] for n in theirs], np.uint32))
            which = lut_of[key]
            lut = luts[which]
            shared = {k: r.header.get(k) for k in SHARED}
            if header is None:
                header = shared
            elif shared != header:
                raise SystemExit(f"{r.path}: header {shared} differs from {header}")
            for k, into in (("producer", producers), ("commit", commits)):
                if r.header.get(k) not in into:
                    into.append(r.header.get(k))
            seeds.append(r.header.get("seeds"))
            if r.error is not None:
                raise r.error
            dropped_roots += r.nonfinite_roots
            dropped_damage += r.dropped_damage
            flagged += r.flagged
            cols, hand, visit_token, visit_prob, part = r.cols, r.hand, r.visit_token, r.visit_prob, r.blob
            q_token, q_value = r.q_token, r.q_value
            if r.format not in formats:
                formats.append(r.format)
            per_game = r.game_samples
            local = np.repeat(np.arange(len(per_game), dtype=np.int64), per_game)
            if trivial < 1.0:
                # A single-option position is kept by its game's number in the whole load and its place in the game.
                place = np.arange(len(local), dtype=np.int64) - np.repeat(np.cumsum(per_game) - per_game, per_game)
                keep = np.ones(len(local), bool)
                for i in np.flatnonzero(cols["legal"] <= 1):
                    keep[i] = _keep(seed + 0x5EED, games + int(local[i]), int(place[i]), trivial)
                if not keep.all():
                    part = np.frombuffer(part, np.uint8)[_ragged(cols["pos_len"], keep)].tobytes()
                    hand = hand[_ragged(cols["hand_count"], keep)]
                    at = _ragged(cols["visit_count"], keep)
                    visit_token, visit_prob = visit_token[at], visit_prob[at]
                    at = _ragged(cols["q_count"], keep)
                    q_token, q_value = q_token[at], q_value[at]
                    cols = {k: v[keep] for k, v in cols.items()}
                    local = local[keep]
            n = len(local)
            blob.write(part)
            blob_size += len(part)
            for k, v in cols.items():
                parts[k].append(v)
            parts["game"].append(games + local)
            parts["hand"].append(np.where(hand < len(lut), lut[np.minimum(hand, len(lut) - 1)], 0).astype(np.uint32))
            parts["visit_token"].append(visit_token)
            parts["visit_prob"].append(visit_prob)
            parts["q_token"].append(q_token)
            parts["q_value"].append(q_value)
            parts["index_of"].append(np.repeat(np.array([which], np.uint16), n))
            parts["repeat"].append(np.repeat(np.array([max(weight, 1.0)], np.float32), n))
            parts["game_key"].append(r.game_key)
            parts["game_reanalysed"].append(r.game_reanalysed)
            if game_decks is not None:
                game_decks.extend(r.game_decks)
            games += len(per_game)
            samples += n
            log(f"  {r.path}: {games} games, {samples} samples")
        summary.append(dict(directory=directory, weight=weight, games=games - before[0], samples=samples - before[1],
                            nonfinite_roots=dropped_roots, dropped_damage=dropped_damage,
                            per_sample_searched=flagged))
    if header is None:
        raise SystemExit(f"no shards in {directories}")
    if header["schema"] != LAYOUT["schema"]:
        raise SystemExit(f"the shards are schema {header['schema']}, the encoding {LAYOUT['schema']}")
    if header["encoder_hash"] != encoder_hash():
        raise SystemExit(f"the shards' encoder hash {header['encoder_hash']} is not ai/encoding.json's {encoder_hash()}")
    header = dict(header, producer="; ".join(map(str, producers)), commit="; ".join(map(str, commits)),
                  format=max(formats, default=1))
    col = {k: _cat(parts[k], t) for k, t in _COLUMNS.items()}

    def offsets(counts):
        return np.concatenate([np.zeros(1, np.int64), np.cumsum(counts, dtype=np.int64)])

    mapped = {}
    if blob_file:
        blob.close()
        path = Path(blob.name)
        atexit.register(_remove, path)
        if blob_size:
            mapped["blob"] = path
            positions = np.memmap(path, np.uint8, "r", shape=(blob_size,))
        else:
            positions = np.zeros(0, np.uint8)
    else:
        positions = _joined(blob, blob_size)
    return Data(
        header=header,
        seeds=seeds,
        blob=positions,
        offset=offsets(col["pos_len"]),
        length=col["length"],
        legal=col["legal"],
        target=col["target"],
        outcome=col["outcome"],
        swing=col["swing"],
        dead=col["dead"],
        game=_cat(parts["game"], np.int32),
        seat=col["seat"],
        decision=col["decision"],
        turn=col["turn"],
        hand_offset=offsets(col["hand_count"]),
        hand=_cat(parts["hand"], np.uint32),
        games=games,
        visit_offset=offsets(col["visit_count"]),
        visit_token=_cat(parts["visit_token"], np.uint8),
        visit_prob=_cat(parts["visit_prob"], np.float32),
        root=col["root"],
        heuristic=col["heuristic"],
        policy=col["policy"],
        distill=col["distill"],
        learner=col["learner"],
        repeat=_cat(parts["repeat"], np.float32),
        directories=summary,
        names=names,
        index_of=_cat(parts["index_of"], np.uint16),
        luts=luts,
        searched=col["searched"],
        fast=col["fast"],
        q_offset=offsets(col["q_count"]),
        q_token=_cat(parts["q_token"], np.uint8),
        q_value=_cat(parts["q_value"], np.float32),
        reanalysed=col["reanalysed"],
        game_key=_cat(parts["game_key"], np.uint64),
        game_reanalysed=_cat(parts["game_reanalysed"], np.uint8),
        mapped=mapped,
    )


def split(data: Data, held_out: float, seed: int = 0) -> tuple[np.ndarray, np.ndarray]:
    """Sample indices for training and held out, held out by whole games. A reanalysed game goes where its source game
    goes (`source_key`): every game of a source that a reanalysed game names takes the side of that source's first
    game (its first self-played one, where that was loaded), so a copy never trains on a held-out game's positions.
    Without reanalysed games each game's side is its own draw, as it always was."""
    rng = np.random.default_rng(seed)
    out = rng.random(data.games) < held_out
    again = getattr(data, "game_reanalysed", None)
    if again is not None and len(again) == data.games and again.any():
        keys = data.game_key
        linked = np.flatnonzero(np.isin(keys, keys[again > 0]))
        # Each key's leader: its first game that is no reanalysed copy, else its first game.
        order = linked[np.lexsort((linked, again[linked], keys[linked]))]
        leader = {}
        for g in order:
            leader.setdefault(int(keys[g]), g)
        for g in linked:
            out[g] = out[leader[int(keys[g])]]
    held = out[data.game]
    return np.flatnonzero(~held), np.flatnonzero(held)


def search_targets(data: Data, rows: np.ndarray | None = None) -> np.ndarray:
    """Whether each sample (of `rows`, or every one) is a search target: a policy target (more than one legal option,
    not a fast search's, not dropped by `searched_policy`) whose visits are a search's root visits (`SEARCHED`)."""
    at = slice(None) if rows is None else rows
    return (data.legal[at] > 1) & (data.policy[at] > 0) & (data.searched[at] == SEARCHED)


def unmark(data: Data, rows: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """A batch's rows as `batches` dealt them, as sample indices, and whether each is a sample all of whose targets are
    read (True) or a policy target alone (False): a policy-only copy of a search target (`search_repeat`), which
    `batches` marks by adding `len(data)`, or a reanalysed game's sample (`Data.reanalysed`)."""
    n = len(data.length)
    fresh = rows < n
    real = np.where(fresh, rows, rows - n)
    again = getattr(data, "reanalysed", None)
    if again is not None and len(again):
        fresh = fresh & (again[real] == 0)
    return real, fresh


def batches(data: Data, indices: np.ndarray, batch: int, rng: np.random.Generator | None, trivial: float = 1.0,
            search_repeat: float = 1.0) -> list[np.ndarray]:
    """`indices` dealt into batches of samples of one length bin, in a random order (`rng`), or in order (`None`).
    Samples with a single legal option are kept at the rate `trivial`: they teach the value, not the policy.

    `search_repeat` (review 2026-11-13, item 1): each search target (`search_targets`) is dealt that many times an
    epoch in all (the fraction by chance), the copies beyond the first marked (`unmark`) so that training reads only
    their policy: a search target is about one sample in twenty and the value overfits whole games, so the policy's
    exposure to the search is raised without raising the value's. Shuffled in with the rest; dealt only with `rng`."""
    if rng is not None and trivial < 1.0:
        keep = (data.legal[indices] > 1) | (rng.random(len(indices)) < trivial)
        indices = indices[keep]
    if rng is not None and len(indices) and (data.repeat[indices] > 1).any():
        r = data.repeat[indices]
        times = np.floor(r).astype(np.int64) + (rng.random(len(indices)) < (r - np.floor(r)))
        indices = np.repeat(indices, times)
    # The samples' count from a column: `data` may be a view of a few columns (belief.py's `View`), with no `len`.
    n = len(data.length)
    copied = False
    if rng is not None and search_repeat > 1.0 and len(indices):
        targets = indices[search_targets(data, indices)]
        extra = search_repeat - 1.0
        times = int(math.floor(extra)) + (rng.random(len(targets)) < (extra - math.floor(extra)))
        indices = np.concatenate([indices, np.repeat(targets, times) + n])
        copied = True
    if rng is not None:
        indices = indices[rng.permutation(len(indices))]
    real = np.where(indices < n, indices, indices - n) if copied else indices
    bins = (data.length[real].astype(np.int64) + GRAIN - 1) // GRAIN
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
        # Summed, not assigned: a shard written before the search kept its root's options apart by index may name one
        # option twice (two options of an equal search token, both written at the first's index), and its mass is all
        # the target's.
        np.add.at(visits[i], data.visit_token[a:b], data.visit_prob[a:b])
    extra = {"heuristic": data.heuristic[rows], "distill": data.distill[rows], "searched": data.searched[rows],
             "fast": data.fast[rows]}
    if len(data.q_value):
        # The search's root value of each option it measured, NaN for the rest (`Data.q_value`).
        q = np.full(inputs["card"].shape, np.nan, np.float32)
        for i, r in enumerate(rows):
            a, b = data.q_offset[r], data.q_offset[r + 1]
            q[i, data.q_token[a:b]] = data.q_value[a:b]
        extra["q"] = q
    if getattr(data, "td", None) is not None:
        extra["td"] = data.td[rows]
    if getattr(data, "future", None) is not None:
        extra["future"] = data.future[rows]
    if getattr(data, "value_weight", None) is not None:
        extra["value_weight"] = data.value_weight[rows]
    if getattr(data, "fate_snap", None) is not None:
        from . import targets  # noqa: PLC0415 (targets imports this module)

        extra.update(targets.fate_labels(data, rows, inputs))
    return dict(
        **extra,
        target=data.target[rows].astype(np.int64),
        visits=visits,
        root=data.root[rows],
        outcome=data.outcome[rows],
        swing=data.swing[rows],
        dead=data.dead[rows],
        choice=(data.legal[rows] > 1) & (data.policy[rows] > 0),
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
