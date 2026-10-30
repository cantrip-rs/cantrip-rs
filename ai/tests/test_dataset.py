"""The dataset reads a self-played shard's visit distributions and root values, and weights directories.

    nix-shell ai/shell.nix --run "python -m unittest discover -s ai/tests"
"""

from __future__ import annotations

import json
import multiprocessing
import pickle
import struct
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
import zstandard

AI = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(AI))
from cantrip_ai import dataset  # noqa: E402

LAYOUT = dataset.LAYOUT


def position(length: int, first: int, options: int, cards: tuple = ()) -> bytes:
    """A compact position of `length` empty tokens whose last `options` are options, all legal; token t's card row
    `cards[t]` (0 past them)."""
    out = bytearray([LAYOUT["schema"], 32, length, first, options, 0])
    out += bytes([0xFF] * ((options + 7) // 8))
    for t in range(length):
        kind = LAYOUT["kinds"].index("option") if t >= first else LAYOUT["kinds"].index("game")
        card = cards[t] if t < len(cards) else 0
        out += bytes([kind, card & 0xFF, card >> 8]) + bytes(dataset.CATS) + bytes([dataset.NO_POINTER] * dataset.PTRS) + bytes(dataset.NUMS // 8)
    return bytes(out)


def position_with_numbers(length: int, numbers: dict[tuple[int, int], int]) -> bytes:
    """A compact position with the given token numbers present, as their f16 bits."""
    out = bytearray([LAYOUT["schema"], 32, length, 0, 0, 0])
    for t in range(length):
        kind = LAYOUT["kinds"].index("game")
        mask = bytearray(dataset.NUMS // 8)
        values = []
        for slot in range(dataset.NUMS):
            bits = numbers.get((t, slot))
            if bits is not None:
                mask[slot // 8] |= 1 << (slot % 8)
                values.append(bits)
        out += bytes([kind, 0, 0]) + bytes(dataset.CATS) + bytes([dataset.NO_POINTER] * dataset.PTRS) + bytes(mask)
        for bits in values:
            out += struct.pack("<H", bits)
    return bytes(out)


def sample(decision: int, chosen: int, visits: list[tuple[int, float]], first: int = 4, options: int = 3,
           cards: tuple = (), hand: tuple = ()) -> bytes:
    out = struct.pack("<BIHHbfBHH", 0, decision, 0, 1, 1, 0.0, 0, chosen, len(visits))
    for o, v in visits:
        out += struct.pack("<Hf", o, v)
    out += struct.pack("<H", len(hand)) + b"".join(struct.pack("<I", c) for c in hand)
    p = position(first + options, first, options, cards)
    return out + struct.pack("<I", len(p)) + p


def write_shard(path: Path, producer: str, games: list[tuple[dict, list[bytes]]], cards: list | None = ("Opt", "Shock")):
    """A shard of `games`, its card index `cards` (none: an older shard's header, which names only its hash)."""
    names = list(cards or ("Opt", "Shock"))
    header = dict(format=1, schema=LAYOUT["schema"], encoder_hash=dataset.encoder_hash(), text_hash=dataset.index_hash(names),
                  commit="c", producer=producer, mode="full", seeds=[0])
    if cards is not None:
        header["cards"] = names
    h = json.dumps(header).encode()
    raw = bytearray(b"CTRPSHD1" + struct.pack("<I", len(h)) + h)
    for meta, samples in games:
        m = json.dumps(meta).encode()
        raw += struct.pack("<I", len(m)) + m + struct.pack("<I", len(samples))
        for s in samples:
            raw += s
    path.write_bytes(zstandard.ZstdCompressor().compress(bytes(raw)))


class DatasetTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self.selfplay = root / "selfplay"
        self.imitation = root / "imitation"
        self.selfplay.mkdir()
        self.imitation.mkdir()
        meta = dict(seed=1, root_values=[[0, 0.25]])
        write_shard(self.selfplay / "a.shard.zst", "selfplay:m@64",
                    [(meta, [sample(0, 1, [(0, 0.25), (1, 0.5), (2, 0.25)]), sample(1, 2, [(2, 1.0)])])])
        write_shard(self.imitation / "a.shard.zst", "tactical:x",
                    [(dict(seed=g), [sample(0, 0, [(0, 1.0)])]) for g in range(40)])

    def tearDown(self):
        self.tmp.cleanup()

    def test_visits_and_root_values(self):
        data = dataset.load([self.selfplay], log=lambda *_: None)
        self.assertEqual(len(data), 2)
        rows = np.arange(2)
        inputs = dataset.decode(data.blob, data.offset, rows)
        labels = dataset.labels(data, rows, inputs)
        np.testing.assert_allclose(labels["visits"][0, 4:7], [0.25, 0.5, 0.25])
        np.testing.assert_allclose(labels["visits"][1, 4:7], [0.0, 0.0, 1.0])
        np.testing.assert_allclose(labels["visits"].sum(-1), [1.0, 1.0])
        self.assertEqual(int(labels["target"][0]), 5)
        self.assertAlmostEqual(float(labels["root"][0]), 0.25)
        self.assertTrue(np.isnan(labels["root"][1]))

    def test_window_of_producers_and_weights(self):
        data = dataset.load([f"{self.selfplay}:3", f"{self.imitation}:0.5"], log=lambda *_: None, seed=7)
        self.assertIn("selfplay:m@64", data.header["producer"])
        self.assertIn("tactical:x", data.header["producer"])
        kept = data.directories[1]["games"]
        self.assertTrue(5 <= kept <= 35, kept)
        self.assertEqual(data.directories[0]["weight"], 3.0)
        # A weight above 1 deals each of its samples that many times an epoch.
        dealt = np.concatenate(dataset.batches(data, np.arange(len(data)), 64, np.random.default_rng(0)))
        self.assertEqual(int((dealt < 2).sum()), 6)
        self.assertEqual(int((dealt >= 2).sum()), kept)

    def test_parse_directory(self):
        self.assertEqual(dataset.parse_directory("a/b:0.5"), ("a/b", 0.5))
        self.assertEqual(dataset.parse_directory("a/b"), ("a/b", 1.0))
        self.assertEqual(dataset.parse_directory("C:/x"), ("C:/x", 1.0))

    def test_null_and_nonfinite_roots_are_absent(self):
        root = Path(self.tmp.name) / "roots"
        root.mkdir()
        meta = dict(seed=1, root_values=[[0, None], [1, 0.5], [2, "nan"]])
        write_shard(root / "a.shard.zst", "selfplay:m@64", [
            (meta, [sample(0, 0, [(0, 1.0)]), sample(1, 0, [(0, 1.0)]), sample(2, 0, [(0, 1.0)])]),
            (dict(seed=2, root_values=None), [sample(3, 0, [(0, 1.0)])]),
        ])
        data = dataset.load([root], log=lambda *_: None)
        self.assertEqual(len(data), 4)
        self.assertTrue(np.isnan(data.root[0]))
        self.assertAlmostEqual(float(data.root[1]), 0.5)
        self.assertTrue(np.isnan(data.root[2]))
        self.assertTrue(np.isnan(data.root[3]))
        self.assertEqual(data.directories[0]["nonfinite_roots"], 2)

    def test_only_the_games_among_the_decks(self):
        root = Path(self.tmp.name) / "decks"
        root.mkdir()
        write_shard(root / "a.shard.zst", "tactical:x", [
            (dict(seed=1, decks=["dimir", "izzet"]), [sample(0, 0, [(0, 1.0)])]),
            (dict(seed=2, decks=["dimir", "azorius"]), [sample(0, 0, [(0, 1.0)]), sample(1, 0, [(0, 1.0)])]),
            (dict(seed=3, decks=["izzet", "izzet"]), [sample(0, 0, [(0, 1.0)])]),
            (dict(seed=4), [sample(0, 0, [(0, 1.0)])]),
        ])
        data = dataset.load([root], log=lambda *_: None, decks={"dimir", "izzet"})
        self.assertEqual((data.games, len(data)), (2, 2))
        # A game passed over does not count towards the directory's cap.
        data = dataset.load([root], max_games=2, log=lambda *_: None, decks={"izzet", "dimir"})
        self.assertEqual(data.games, 2)
        self.assertEqual(dataset.load([root], log=lambda *_: None).games, 4)

    def test_card_indexes_are_mapped_onto_one(self):
        root = Path(self.tmp.name) / "indexes"
        (root / "a").mkdir(parents=True)
        (root / "b").mkdir()
        # a: Opt is row 1, Shock row 2. b: Bolt 1, Shock 2, Opt 3.
        write_shard(root / "a" / "a.shard.zst", "p", [(dict(seed=1), [sample(0, 0, [(0, 1.0)], cards=(1, 2), hand=(2,))])],
                    cards=["Opt", "Shock"])
        write_shard(root / "b" / "b.shard.zst", "p", [(dict(seed=2), [sample(0, 0, [(0, 1.0)], cards=(1, 2, 3), hand=(1,))])],
                    cards=["Bolt", "Shock", "Opt"])
        data = dataset.load([root / "a", root / "b"], log=lambda *_: None, base=["Shock"])
        # The base keeps its rows; each name new to it follows, as the shards first name it.
        self.assertEqual(data.names, ["Shock", "Opt", "Bolt"])
        rows = np.arange(2)
        inputs = dataset.decode(data.blob, data.offset, rows)
        card = dataset.cards(data, rows, inputs["card"])
        self.assertEqual(card[0, :3].tolist(), [2, 1, 0])
        self.assertEqual(card[1, :4].tolist(), [3, 1, 2, 0])
        self.assertEqual(data.hand.tolist(), [1, 3])
        # A position written out in the training index's rows (train.py's references) reads back as mapped.
        p = dataset.with_rows(data.blob[data.offset[1] : data.offset[2]].tobytes(), data.luts[data.index_of[1]])
        again = dataset.decode(np.frombuffer(p, np.uint8), np.array([0, len(p)]), np.array([0]))
        self.assertEqual(again["card"][0, :4].tolist(), [3, 1, 2, 0])

    def test_an_older_shard_takes_the_index_above_it(self):
        root = Path(self.tmp.name) / "older"
        (root / "gen1").mkdir(parents=True)
        write_shard(root / "gen1" / "a.shard.zst", "p", [(dict(seed=1), [sample(0, 0, [(0, 1.0)], cards=(2,))])], cards=None)
        with self.assertRaises(SystemExit):
            dataset.load([root / "gen1"], log=lambda *_: None)
        (root / "cards.json").write_text(json.dumps(dict(names=["Opt", "Shock"])))
        data = dataset.load([root / "gen1"], log=lambda *_: None)
        self.assertEqual(data.names, ["Opt", "Shock"])
        (root / "cards.json").write_text(json.dumps(dict(names=["Shock", "Opt"])))
        with self.assertRaises(SystemExit):
            dataset.load([root / "gen1"], log=lambda *_: None)

    def test_decode_sanitizes_nonfinite_numbers(self):
        p = position_with_numbers(1, {(0, 0): 0x7C00, (0, 1): 0xFC00, (0, 2): 0x7E00})
        blob = np.frombuffer(p, np.uint8)
        offset = np.array([0, len(p)], np.int64)
        rows = np.array([0])
        raw = dataset.decode(blob, offset, rows, tokens=32, sanitize=False)
        self.assertTrue(np.isposinf(raw["nums"][0, 0, 0]))
        self.assertTrue(np.isneginf(raw["nums"][0, 0, 1]))
        self.assertTrue(np.isnan(raw["nums"][0, 0, 2]))
        safe = dataset.decode(blob, offset, rows, tokens=32)
        self.assertEqual(float(safe["nums"][0, 0, 0]), dataset.F16_MAX)
        self.assertEqual(float(safe["nums"][0, 0, 1]), -dataset.F16_MAX)
        self.assertEqual(float(safe["nums"][0, 0, 2]), 0.0)
        self.assertTrue(np.isfinite(safe["nums"]).all())

    def test_shared_data_reaches_a_spawned_process_by_name(self):
        data = dataset.load([self.selfplay, self.imitation], log=lambda *_: None)
        before = {k: v.copy() for k, v in data.__dict__.items() if isinstance(v, np.ndarray)}
        copied = len(pickle.dumps(data))
        dataset.share(data)
        self.assertIn("blob", data.blocks)
        for k, v in before.items():
            np.testing.assert_array_equal(getattr(data, k), v)
        # Pickled, the arrays are block names: the blob's bytes are not in it.
        shared = pickle.dumps(data)
        self.assertLess(len(shared), copied - data.blob.nbytes)
        with multiprocessing.get_context("spawn").Pool(1) as pool:
            sums = pool.apply(_sums, (data,))
        self.assertEqual(sums, _sums(data))


def _sums(data) -> dict:
    """What a worker reads of each array (module level, so a spawned process can be handed it)."""
    return {k: float(np.nansum(v.astype(np.float64))) for k, v in data.__dict__.items() if isinstance(v, np.ndarray)}


if __name__ == "__main__":
    unittest.main()
