"""Training shards, read back: the files `cantrip-native-host --ai-record DIR` writes (crates/cantrip-ai/src/shard.rs
has the layout; docs/ai-player-plan.md, "Data").

A shard is concatenated zstd frames which, decompressed, are one little-endian stream: b"CTRPSHD1", a u32-prefixed
JSON header, then per game a u32-prefixed JSON meta, a u32 sample count and the samples. A sample's position is the
compact form of `cantrip_obs::encode::to_compact`, which `position()` turns into numpy arrays in the layout of
ai/encoding.json.

    python ai/shards.py DIR [--check]   # counts, header values, and (--check) every chosen option is legal
"""

from __future__ import annotations

import io
import json
import struct
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import BinaryIO, Iterator

import numpy as np
import zstandard

MAGIC = b"CTRPSHD1"
FORMAT = 1
EXTENSION = ".shard.zst"
LAYOUT = json.loads((Path(__file__).parent / "encoding.json").read_text())


@dataclass
class Sample:
    seat: int  # 0-based
    decision: int  # the answer's index on the game's line; a decision's sub-steps share it
    step: int  # the sub-step within the decision
    turn: int
    outcome: int  # +1 won, -1 lost, 0 draw or cap, for the sample's seat
    life_swing: float  # (own life - opponents') at the end of the turn, minus the same now
    dead_next_turn: bool
    chosen: int  # the option chosen, an index into the position's options
    visits: list[tuple[int, float]]  # the visit distribution over options (one-hot for imitation)
    opponent_hand: list[int]  # card index rows of the opponents' hands (the header's `cards`)
    position: bytes  # the compact encoded position: `position(sample.position)`


@dataclass
class Game:
    meta: dict
    samples: list[Sample] = field(default_factory=list)


class _Stream:
    def __init__(self, raw: BinaryIO):
        self.raw = raw

    def take(self, n: int) -> bytes:
        out = bytearray()
        while len(out) < n:
            chunk = self.raw.read(n - len(out))
            if not chunk:
                break
            out += chunk
        return bytes(out)

    def exact(self, fmt: str):
        size = struct.calcsize(fmt)
        data = self.take(size)
        if len(data) != size:
            raise ValueError("the shard ends inside a game")
        return struct.unpack(fmt, data)


def _sample(s: _Stream) -> Sample:
    seat, decision, step, turn, outcome, swing, dead, chosen, v = s.exact("<BIHHbfBHH")
    visits = [s.exact("<Hf") for _ in range(v)]
    (h,) = s.exact("<H")
    hand = list(s.exact(f"<{h}I")) if h else []
    (n,) = s.exact("<I")
    position = s.take(n)
    return Sample(seat, decision, step, turn, outcome, swing, bool(dead), chosen, visits, hand, position)


def read(path: str | Path) -> tuple[dict, Iterator[Game]]:
    """A shard's header, and its games, read one at a time."""
    f = open(path, "rb")
    raw = zstandard.ZstdDecompressor().stream_reader(f, read_across_frames=True)
    s = _Stream(io.BufferedReader(raw, 1 << 20))
    if s.take(8) != MAGIC:
        raise ValueError(f"{path} is not a shard")
    (n,) = s.exact("<I")
    header = json.loads(s.take(n))
    if header["format"] != FORMAT:
        raise ValueError(f"{path} is shard format {header['format']}, not {FORMAT}")

    def games() -> Iterator[Game]:
        try:
            while True:
                first = s.take(4)
                if not first:
                    return
                (n,) = struct.unpack("<I", first)
                meta = json.loads(s.take(n))
                (count,) = s.exact("<I")
                yield Game(meta, [_sample(s) for _ in range(count)])
        finally:
            f.close()

    return header, games()


def shards(directory: str | Path) -> list[Path]:
    """The finished shards in a directory (a `.part` is one still being written), sorted."""
    return sorted(p for p in Path(directory).iterdir() if p.name.endswith(EXTENSION))


def _f16(bits: np.ndarray) -> np.ndarray:
    return bits.astype(np.uint16).view(np.float16).astype(np.float32)


def position(data: bytes) -> dict[str, np.ndarray | int | bool]:
    """A compact position as arrays: `kind` [bucket] u8, `card` [bucket] u32, `cats` [bucket, CATS] u8,
    `ptrs` [bucket, PTRS] u8, `nums` [bucket, NUMS] f32, `legal` [bucket] bool, and `bucket`, `len`, `first_option`,
    `options`, `cap_hit`. The inverse of `to_compact`, as `from_compact` reads it."""
    cats_n, ptrs_n, nums_n = LAYOUT["cats"], LAYOUT["ptrs"], LAYOUT["nums"]
    schema, bucket, length, first, options, cap = data[0], data[1], data[2], data[3], data[4], data[5]
    if schema != LAYOUT["schema"]:
        raise ValueError(f"position schema {schema}, not {LAYOUT['schema']}")
    at = 6
    bits = np.frombuffer(data, np.uint8, (options + 7) // 8, at)
    at += (options + 7) // 8
    kind = np.zeros(bucket, np.uint8)
    card = np.full(bucket, LAYOUT["unknown_card"], np.uint32)
    cats = np.zeros((bucket, cats_n), np.uint8)
    ptrs = np.full((bucket, ptrs_n), LAYOUT["no_pointer"], np.uint8)
    nums = np.zeros((bucket, nums_n), np.float32)
    legal = np.zeros(bucket, bool)
    legal[first : first + options] = np.unpackbits(bits, bitorder="little")[:options].astype(bool)
    mask_bytes = nums_n // 8
    for t in range(length):
        kind[t] = data[at]
        card[t] = data[at + 1] | (data[at + 2] << 8)
        at += 3
        cats[t] = np.frombuffer(data, np.uint8, cats_n, at)
        at += cats_n
        ptrs[t] = np.frombuffer(data, np.uint8, ptrs_n, at)
        at += ptrs_n
        present = np.unpackbits(np.frombuffer(data, np.uint8, mask_bytes, at), bitorder="little").astype(bool)
        at += mask_bytes
        k = int(present.sum())
        nums[t, present] = _f16(np.frombuffer(data, "<u2", k, at))
        at += 2 * k
    if at != len(data):
        raise ValueError("the position has bytes left over")
    return dict(bucket=bucket, len=length, first_option=first, options=options, cap_hit=bool(cap),
                kind=kind, card=card, cats=cats, ptrs=ptrs, nums=nums, legal=legal)


def main(argv: list[str]) -> int:
    if not argv:
        print(__doc__)
        return 2
    check = "--check" in argv
    files = shards(argv[0])
    games = samples = bad = 0
    headers = set()
    decks: dict[str, int] = {}
    for path in files:
        header, gs = read(path)
        # The card index (`cards`) is thousands of names: shown by its hash and size.
        shown = {k: v for k, v in header.items() if k != "cards"}
        if header.get("cards"):
            shown["cards"] = f"{len(header['cards'])} names"
        headers.add(json.dumps(shown, sort_keys=True))
        for g in gs:
            games += 1
            samples += len(g.samples)
            for d in g.meta.get("decks", []):
                decks[d] = decks.get(d, 0) + 1
            if check:
                for s in g.samples:
                    p = position(s.position)
                    if not (s.chosen < p["options"] and p["legal"][p["first_option"] + s.chosen]):
                        bad += 1
    size = sum(p.stat().st_size for p in files)
    print(f"{len(files)} shards, {games} games, {samples} samples, {size} bytes"
          f" ({size / max(samples, 1):.0f} a sample, {samples / max(games, 1):.0f} samples a game)")
    for h in sorted(headers):
        print("header", h)
    print("decks", dict(sorted(decks.items(), key=lambda kv: -kv[1])))
    if check:
        print(f"{bad} samples whose chosen option is not legal")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
