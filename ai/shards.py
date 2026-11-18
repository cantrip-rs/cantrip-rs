"""Training shards, read back: the files `cantrip-native-host --ai-record DIR` writes (crates/cantrip-ai/src/shard.rs
has the layout; docs/ai-player-plan.md, "Data").

A shard is concatenated zstd frames which, decompressed, are one little-endian stream: b"CTRPSHD1", a u32-prefixed
JSON header, then per game a u32-prefixed JSON meta, a u32 sample count and the samples. A sample's position is the
compact form of `cantrip_obs::encode::to_compact`, which `position()` turns into numpy arrays in the layout of
ai/encoding.json.

Two formats are read (the header's `format`): format 2 gives each sample a flags byte (bit 0: `searched`, its visits
are a search's root visits) and beside each visit the option's root value for the seat (NaN where unknown), read as
`option_q`, the `(option, value)` pairs whose value is known; format 1, every shard written before 2026-11-13, has
neither, and its samples read with `searched` None (not said) and `option_q` empty. These are what
`cantrip_ai.dataset` reads as `Sample.searched` and `Sample.option_q`.

    python ai/shards.py DIR [--check]   # counts, header values, and (--check) every chosen option is legal
"""

from __future__ import annotations

import json
import struct
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import BinaryIO, Iterator

import numpy as np
import zstandard

MAGIC = b"CTRPSHD1"
FORMAT = 2  # what the recorder writes now
FORMATS = (1, 2)  # what is read
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
    searched: bool | None = None  # its visits are a search's root visits (format 2); None in a format 1 shard
    # (option, value): each visited option's root value for the seat where it is known (format 2; NaN ones left out)
    option_q: list[tuple[int, float]] = field(default_factory=list)


@dataclass
class Game:
    meta: dict
    samples: list[Sample] = field(default_factory=list)


CHUNK = 1 << 24  # how much of the decompressed stream is read at once
_HEAD = struct.Struct("<BIHHbfBHH")  # a format 1 sample's fixed fields, its visits' count last
_VISIT = struct.Struct("<Hf")
_HEAD2 = struct.Struct("<BIHHbfBHBH")  # format 2: the flags byte before the visits' count
_VISIT2 = struct.Struct("<Hff")  # format 2: option, share, q
_U16 = struct.Struct("<H")
_U32 = struct.Struct("<I")
_SHORT = "the shard ends inside a game"


class _Stream:
    """The decompressed stream, read `chunk` bytes at a time and parsed in place (`struct.unpack_from`), so a sample costs a
    few calls rather than a read per field."""

    def __init__(self, raw: BinaryIO, chunk: int = CHUNK):
        self.raw = raw
        self.chunk = chunk
        self.buf = b""
        self.at = 0

    def have(self, n: int) -> bool:
        """Whether `n` bytes are buffered from `at`, reading more as needed: false only at the stream's end."""
        if self.at + n <= len(self.buf):
            return True
        parts = [self.buf[self.at :]]
        size = len(parts[0])
        while size < n:
            chunk = self.raw.read(max(self.chunk, n - size))
            if not chunk:
                break
            parts.append(chunk)
            size += len(chunk)
        self.buf = b"".join(parts)
        self.at = 0
        return size >= n

    def take(self, n: int) -> bytes:
        """`n` bytes, or as many as the stream has left."""
        self.have(n)
        out = self.buf[self.at : self.at + n]
        self.at += len(out)
        return out

    def exact(self, st: struct.Struct) -> tuple:
        if not self.have(st.size):
            raise ValueError(_SHORT)
        out = st.unpack_from(self.buf, self.at)
        self.at += st.size
        return out


class _Layout:
    """A format's sample layout: its fixed fields and a visit's size."""

    def __init__(self, fmt: int):
        self.format = fmt
        self.head = _HEAD2 if fmt >= 2 else _HEAD
        self.vsize = (_VISIT2 if fmt >= 2 else _VISIT).size

    def fields(self, head: tuple) -> tuple:
        """seat, decision, step, turn, outcome, swing, dead, chosen, searched, v"""
        if self.format >= 2:
            seat, decision, step, turn, outcome, swing, dead, chosen, flags, v = head
            return seat, decision, step, turn, outcome, swing, dead, chosen, bool(flags & 1), v
        seat, decision, step, turn, outcome, swing, dead, chosen, v = head
        return seat, decision, step, turn, outcome, swing, dead, chosen, None, v

    def visits(self, raw: bytes) -> tuple[list, list]:
        """The visits, and the `(option, q)` of those whose q value is known (none for format 1)."""
        if self.format < 2:
            return list(_VISIT.iter_unpack(raw)), []
        triples = list(_VISIT2.iter_unpack(raw))
        return [(t[0], t[1]) for t in triples], [(t[0], t[2]) for t in triples if t[2] == t[2]]


_LAYOUTS = {f: _Layout(f) for f in FORMATS}


def _sample(s: _Stream, lay: _Layout = _LAYOUTS[1]) -> Sample:
    seat, decision, step, turn, outcome, swing, dead, chosen, searched, v = lay.fields(s.exact(lay.head))
    if v:
        if not s.have(lay.vsize * v):
            raise ValueError(_SHORT)
        visits, option_q = lay.visits(s.buf[s.at : s.at + lay.vsize * v])
        s.at += lay.vsize * v
    else:
        visits, option_q = [], []
    (h,) = s.exact(_U16)
    if h:
        if not s.have(4 * h):
            raise ValueError(_SHORT)
        hand = list(struct.unpack_from(f"<{h}I", s.buf, s.at))
        s.at += 4 * h
    else:
        hand = []
    (n,) = s.exact(_U32)
    position = s.take(n)
    return Sample(seat, decision, step, turn, outcome, swing, bool(dead), chosen, visits, hand, position, searched,
                  option_q)


def _samples(s: _Stream, count: int, lay: _Layout = _LAYOUTS[1]) -> list[Sample]:
    """`count` samples: `_sample`, with the samples wholly in the buffer read without a call a field."""
    out = []
    head, u16, u32 = lay.head.unpack_from, _U16.unpack_from, _U32.unpack_from
    hsize, vsize, fields, read_visits = lay.head.size, lay.vsize, lay.fields, lay.visits
    buf, at, end = s.buf, s.at, len(s.buf)
    for _ in range(count):
        q = at + hsize
        if q <= end:
            seat, decision, step, turn, outcome, swing, dead, chosen, searched, v = fields(head(buf, at))
            r = q + vsize * v + 2
            if r <= end:
                h = u16(buf, r - 2)[0]
                t = r + 4 * h + 4
                if t <= end:
                    n = u32(buf, t - 4)[0]
                    if t + n <= end:
                        visits, qs = read_visits(buf[q : q + vsize * v]) if v else ([], [])
                        hand = list(struct.unpack_from(f"<{h}I", buf, r)) if h else []
                        out.append(Sample(seat, decision, step, turn, outcome, swing, bool(dead), chosen, visits, hand,
                                          buf[t : t + n], searched, qs))
                        at = t + n
                        continue
        s.at = at
        out.append(_sample(s, lay))
        buf, at, end = s.buf, s.at, len(s.buf)
    s.at = at
    return out


def _skip(s: _Stream, lay: _Layout = _LAYOUTS[1]):
    """Past a sample unread, as `_sample` would go (and failing where it would)."""
    hsize = lay.head.size
    if not s.have(hsize):
        raise ValueError(_SHORT)
    (v,) = _U16.unpack_from(s.buf, s.at + hsize - 2)
    s.at += hsize
    if not s.have(lay.vsize * v + 2):
        raise ValueError(_SHORT)
    (h,) = _U16.unpack_from(s.buf, s.at + lay.vsize * v)
    s.at += lay.vsize * v + 2
    if not s.have(4 * h + 4):
        raise ValueError(_SHORT)
    (n,) = _U32.unpack_from(s.buf, s.at + 4 * h)
    s.at += 4 * h + 4
    s.have(n)
    s.at = min(s.at + n, len(s.buf))


def _skip_many(s: _Stream, count: int, lay: _Layout = _LAYOUTS[1]):
    """Past `count` samples unread: `_skip`, with the samples wholly in the buffer walked without a call each."""
    buf, at, end = s.buf, s.at, len(s.buf)
    u16, u32 = _U16.unpack_from, _U32.unpack_from
    hsize, vsize = lay.head.size, lay.vsize
    for _ in range(count):
        q = at + hsize
        if q <= end:
            q += vsize * u16(buf, q - 2)[0] + 2
            if q <= end:
                q += 4 * u16(buf, q - 2)[0] + 4
                if q <= end:
                    q += u32(buf, q - 4)[0]
                    if q <= end:
                        at = q
                        continue
        s.at = at
        _skip(s, lay)
        buf, at, end = s.buf, s.at, len(s.buf)
    s.at = at


class LazyGame:
    """A game read by `scan`: its meta, its sample count, and its samples, read when first asked for; samples never
    asked for before the next game are skipped, which costs a fraction of reading them."""

    __slots__ = ("meta", "count", "_stream", "_samples", "_done", "_layout")

    def __init__(self, meta: dict, count: int, stream: _Stream, layout: _Layout = _LAYOUTS[1]):
        self.meta, self.count, self._stream, self._samples, self._done = meta, count, stream, None, False
        self._layout = layout

    @property
    def samples(self) -> list[Sample]:
        if self._samples is None:
            if self._done:
                raise RuntimeError("a game's samples are asked for after the next game was read")
            self._samples = _samples(self._stream, self.count, self._layout)
        return self._samples

    def _finish(self):
        if self._samples is None and not self._done:
            _skip_many(self._stream, self.count, self._layout)
        self._done = True


def _open(path: str | Path, f: BinaryIO, chunk: int = CHUNK) -> tuple[_Stream, dict]:
    s = _Stream(zstandard.ZstdDecompressor().stream_reader(f, read_across_frames=True), chunk)
    if s.take(8) != MAGIC:
        raise ValueError(f"{path} is not a shard")
    (n,) = s.exact(_U32)
    header = json.loads(s.take(n))
    if header["format"] not in FORMATS:
        raise ValueError(f"{path} is shard format {header['format']}, not one of {FORMATS}")
    return s, header


def header(path: str | Path) -> dict:
    """A shard's header alone."""
    with open(path, "rb") as f:
        return _open(path, f, 1 << 16)[1]


def scan(path: str | Path) -> tuple[dict, Iterator[LazyGame]]:
    """A shard's header, and its games, read one at a time, each game's samples only if asked for (`LazyGame`)."""
    f = open(path, "rb")
    try:
        s, header = _open(path, f)
    except BaseException:
        f.close()
        raise

    layout = _LAYOUTS[header["format"]]

    def games() -> Iterator[LazyGame]:
        try:
            while True:
                first = s.take(4)
                if not first:
                    return
                (n,) = struct.unpack("<I", first)
                meta = json.loads(s.take(n))
                (count,) = s.exact(_U32)
                game = LazyGame(meta, count, s, layout)
                yield game
                game._finish()
        finally:
            f.close()

    return header, _Games(games(), f)


class _Games:
    """`scan`'s games: an iterator whose `close` closes the file too, though not a game has been read."""

    def __init__(self, games: Iterator[LazyGame], f: BinaryIO):
        self._games, self._f = games, f

    def __iter__(self):
        return self

    def __next__(self) -> LazyGame:
        return next(self._games)

    def close(self):
        self._games.close()
        self._f.close()


def read(path: str | Path) -> tuple[dict, Iterator[Game]]:
    """A shard's header, and its games, read one at a time."""
    header, games = scan(path)
    return header, (Game(g.meta, g.samples) for g in games)


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
    # Schema 1's positions are laid out as this encoding's, with nothing of what schema 4 added (`model.SCHEMA1_HASH`;
    # `dataset.UPGRADABLE` takes their shards).
    if schema != LAYOUT["schema"] and not (schema == 1 and LAYOUT["schema"] == 4):
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
