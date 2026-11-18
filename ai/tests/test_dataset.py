"""The dataset reads a self-played shard's visit distributions and root values, and weights directories.

    nix-shell ai/shell.nix --run "python -m unittest discover -s ai/tests"
"""

from __future__ import annotations

import json
import multiprocessing
import os
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


def position(length: int, first: int, options: int, cards: tuple = (), ask: int = 0) -> bytes:
    """A compact position of `length` empty tokens whose last `options` are options, all legal; token t's card row
    `cards[t]` (0 past them); the first token, the game, has the ask categorical `ask`."""
    out = bytearray([LAYOUT["schema"], 32, length, first, options, 0])
    out += bytes([0xFF] * ((options + 7) // 8))
    for t in range(length):
        kind = LAYOUT["kinds"].index("option") if t >= first else LAYOUT["kinds"].index("game")
        card = cards[t] if t < len(cards) else 0
        cats = bytearray(dataset.CATS)
        if t == 0:
            cats[dataset.ASK_SLOT] = ask
        out += bytes([kind, card & 0xFF, card >> 8]) + bytes(cats) + bytes([dataset.NO_POINTER] * dataset.PTRS) + bytes(dataset.NUMS // 8)
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
           cards: tuple = (), hand: tuple = (), ask: int = 0) -> bytes:
    out = struct.pack("<BIHHbfBHH", 0, decision, 0, 1, 1, 0.0, 0, chosen, len(visits))
    for o, v in visits:
        out += struct.pack("<Hf", o, v)
    out += struct.pack("<H", len(hand)) + b"".join(struct.pack("<I", c) for c in hand)
    p = position(first + options, first, options, cards, ask)
    return out + struct.pack("<I", len(p)) + p


def sample2(decision: int, chosen: int, visits: list[tuple[int, float]], searched: bool, q: list[float] | None = None,
            seat: int = 0, first: int = 4, options: int = 3) -> bytes:
    """A format 2 sample: the flags byte (`searched`) and a q beside each visit (NaN where `q` is None)."""
    q = q if q is not None else [float("nan")] * len(visits)
    out = struct.pack("<BIHHbfBHBH", seat, decision, 0, 1, 1, 0.0, 0, chosen, int(searched), len(visits))
    for (o, v), x in zip(visits, q):
        out += struct.pack("<Hff", o, v, x)
    out += struct.pack("<H", 0)
    p = position(first + options, first, options)
    return out + struct.pack("<I", len(p)) + p


def write_shard(path: Path, producer: str, games: list[tuple[dict, list[bytes]]], cards: list | None = ("Opt", "Shock"),
                fmt: int = 1):
    """A shard of `games`, its card index `cards` (none: an older shard's header, which names only its hash), in
    format `fmt` (its samples must be written in it: `sample` for 1, `sample2` for 2)."""
    names = list(cards or ("Opt", "Shock"))
    header = dict(format=fmt, schema=LAYOUT["schema"], encoder_hash=dataset.encoder_hash(), text_hash=dataset.index_hash(names),
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

    def test_only_searched_decisions_are_policy_targets_when_asked(self):
        # The self-played game searched decision 0 alone (its root value); decision 1 is the network's own answer.
        plain = dataset.load([self.selfplay, self.imitation], log=lambda *_: None)
        data = dataset.load([self.selfplay, self.imitation], log=lambda *_: None, searched_policy=True)
        np.testing.assert_array_equal(plain.policy, np.ones(len(plain), np.uint8))
        self.assertEqual(data.policy[:2].tolist(), [1, 0])
        np.testing.assert_array_equal(data.policy[2:], plain.policy[2:])  # imitation: unchanged
        np.testing.assert_array_equal(data.outcome, plain.outcome)  # value targets: unchanged

    def test_a_fast_search_is_a_policy_target_only_when_asked(self):
        root = Path(self.tmp.name) / "fast"
        root.mkdir()
        # Decisions 1 and 2 are fast searches; only 1's target is Gumbel's (2's a PUCT search's visit counts).
        meta = dict(seed=1, root_values=[[0, 0.25], [1, 0.5], [2, 0.0]], fast_decisions=[1, 2], gumbel_decisions=[0, 1])
        write_shard(root / "a.shard.zst", "selfplay:m@64",
                    [(meta, [sample(0, 1, [(0, 0.25), (1, 0.75)]), sample(1, 0, [(0, 0.5), (1, 0.5)]),
                             sample(2, 0, [(0, 1.0)])])])
        plain = dataset.load([root], log=lambda *_: None)
        fast = dataset.load([root], log=lambda *_: None, fast_policy=True)
        self.assertEqual(plain.policy.tolist(), [1, 0, 0])
        self.assertEqual(fast.policy.tolist(), [1, 1, 0])
        self.assertEqual(fast.fast.tolist(), plain.fast.tolist())  # still a fast search for the value's mix

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

    def test_older_shards_lose_their_combat_damage_assignments(self):
        root = Path(self.tmp.name) / "damage"
        root.mkdir()
        priority, damage = 1, dataset.DAMAGE_ASK
        self.assertEqual(dataset.LAYOUT["ask_kinds"][damage - 1], "AssignCombatDamage")
        games = [
            (dict(seed=1, root_values=[[0, 0.5], [2, -0.5]]),
             [sample(0, 0, [(0, 1.0)], ask=priority), sample(1, 1, [(1, 1.0)], ask=damage), sample(2, 2, [(2, 1.0)], ask=priority)]),
            (dict(seed=2), [sample(0, 0, [(0, 1.0)], ask=damage)]),  # a game left with no sample is still a game
            (dict(seed=3), [sample(0, 2, [(2, 1.0)], ask=priority)]),
        ]
        write_shard(root / "a.shard.zst", "selfplay:m@64", games)
        mtime = 1_800_000_000.0
        os.utime(root / "a.shard.zst", (mtime, mtime))
        quiet = dict(log=lambda *_: None)

        def arrays(data):
            return {k: v for k, v in data.__dict__.items() if isinstance(v, np.ndarray)}

        whole = dataset.load([root], **quiet)
        self.assertEqual((whole.games, len(whole)), (3, 5))
        self.assertEqual(whole.directories[0]["dropped_damage"], 0)
        # The cutoff before the shard's time, or at it, changes nothing.
        for cutoff in (mtime - 1, mtime):
            same = dataset.load([root], drop_damage_before=cutoff, **quiet)
            self.assertEqual(same.directories, whole.directories)
            for k, v in arrays(whole).items():
                np.testing.assert_array_equal(getattr(same, k), v, err_msg=k)
        # After it, the shard's damage assignments are gone, as if never recorded; its games and every other sample stay.
        data = dataset.load([root], drop_damage_before=mtime + 1, **quiet)
        self.assertEqual((data.games, len(data)), (3, 3))
        self.assertEqual(data.directories[0]["dropped_damage"], 2)
        self.assertEqual(data.directories[0]["samples"], 3)
        self.assertEqual(data.game.tolist(), [0, 0, 2])
        self.assertEqual(data.decision.tolist(), [0, 2, 0])
        self.assertEqual(data.target.tolist(), [4, 6, 6])
        np.testing.assert_allclose(data.root[:2], [0.5, -0.5])
        kept = [r for r in range(len(whole)) if dataset.ask_of(whole.blob[whole.offset[r] : whole.offset[r + 1]].tobytes()) != damage]
        self.assertEqual(kept, [0, 2, 4])
        self.assertEqual(data.blob.tobytes(), b"".join(whole.blob[whole.offset[r] : whole.offset[r + 1]].tobytes() for r in kept))
        inputs = dataset.decode(data.blob, data.offset, np.arange(len(data)))
        self.assertTrue((inputs["cats"][:, 0, dataset.ASK_SLOT] == priority).all())
        # Held out by game, the split never sees them either.
        train, held = dataset.split(data, 0.5, 3)
        self.assertEqual(sorted(np.concatenate([train, held]).tolist()), [0, 1, 2])

    def test_ask_of_reads_the_game_tokens_ask(self):
        p = position(5, 2, 3, ask=dataset.DAMAGE_ASK)
        self.assertEqual(dataset.ask_of(p), dataset.DAMAGE_ASK)
        inputs = dataset.decode(np.frombuffer(p, np.uint8), np.array([0, len(p)]), np.array([0]))
        self.assertEqual(int(inputs["cats"][0, 0, dataset.ASK_SLOT]), dataset.DAMAGE_ASK)
        self.assertEqual(dataset.ask_of(position(4, 1, 3)), 0)
        self.assertEqual(dataset.ask_of(bytes([LAYOUT["schema"], 32, 0, 0, 0, 0])), 0)

    def test_ask_of_agrees_with_decode_on_recorded_positions(self):
        """Real self-play positions with damage assignments (generation 1's, before the answer was factored by
        blocker): the fast read agrees with a full decode. Skipped where that corpus is absent."""
        directory = Path(os.environ.get("CANTRIP_DAMAGE_SHARDS", "../ai-data/templates/fp/gen1-damage-per-point/selfplay"))
        if not directory.is_dir() or not dataset.shards.shards(directory):
            self.skipTest(f"{directory} is absent")
        shards = dataset.shards
        raw = []
        for path in shards.shards(directory):
            _, gs = shards.read(path)
            for g in gs:
                raw.extend(s.position for s in g.samples)
                if len(raw) >= 4000:
                    break
            if len(raw) >= 4000:
                break
        blob = np.frombuffer(b"".join(raw), np.uint8)
        offset = np.concatenate([[0], np.cumsum([len(p) for p in raw])]).astype(np.int64)
        expected = []
        for at in range(0, len(raw), 500):
            inputs = dataset.decode(blob, offset, np.arange(at, min(at + 500, len(raw))))
            game = inputs["kind"] == dataset.GAME
            self.assertTrue(game[:, 0].all())
            self.assertTrue((game.sum(1) == 1).all())
            expected.append(inputs["cats"][:, :, dataset.ASK_SLOT][game])
        expected = np.concatenate(expected)
        fast = np.array([dataset.ask_of(p) for p in raw])
        np.testing.assert_array_equal(fast, expected)
        self.assertGreater(int((fast == dataset.DAMAGE_ASK).sum()), 0, "no damage assignment among the positions read")

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

    def test_a_blob_in_a_file_is_the_same_and_reaches_a_spawned_process_by_its_path(self):
        plain = dataset.load([self.selfplay, self.imitation], log=lambda *_: None)
        with tempfile.TemporaryDirectory() as spill:
            os.environ["CANTRIP_SPILL_DIR"] = spill
            try:
                data = dataset.load([self.selfplay, self.imitation], log=lambda *_: None, blob_file=True)
            finally:
                del os.environ["CANTRIP_SPILL_DIR"]
            self.assertEqual(plain.mapped, {})
            self.assertEqual(set(data.mapped), {"blob"})
            self.assertIsInstance(data.blob, np.memmap)
            for k, v in plain.__dict__.items():
                if isinstance(v, np.ndarray):
                    np.testing.assert_array_equal(getattr(data, k), v)
            dataset.share(data)
            self.assertNotIn("blob", data.blocks)  # `share` leaves the mapped blob where it is
            self.assertIsInstance(data.blob, np.memmap)
            # Pickled, the blob is its file's path: none of its bytes are in it.
            self.assertLess(len(pickle.dumps(data)), len(pickle.dumps(plain)) - plain.blob.nbytes)
            with multiprocessing.get_context("spawn").Pool(1) as pool:
                sums = pool.apply(_sums, (data,))
            self.assertEqual(sums, _sums(plain))
            data.blob._mmap.close()
            del data


class FormatTwoTest(unittest.TestCase):
    """Shard format 2's per-sample columns: `searched` (`SEARCHED` / `NOT_SEARCHED` by the sample's own flag), the
    options' root values (`q_offset`, `q_token`, `q_value`) and `learner`; and a window mixing it with format 1, whose
    samples take their decision's flag."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        root = Path(self.tmp.name)
        self.new = root / "new"
        self.old = root / "old"
        self.new.mkdir()
        self.old.mkdir()
        nan = float("nan")
        # Decision 0 searched: its first sub-step reached (visits and q), its second not (one-hot, not searched).
        # Decision 1, seat 2's, was an earlier champion's (`learners` names seat 1 alone).
        meta = dict(seed=1, root_values=[[0, 0.25]], learners=[1])
        write_shard(self.new / "a.shard.zst", "selfplay:m@64", [(meta, [
            sample2(0, 1, [(0, 0.25), (1, 0.75)], True, [-0.5, 0.125]),
            sample2(0, 2, [(2, 1.0)], False),
            sample2(1, 0, [(0, 1.0)], False, seat=1),
        ])], fmt=2)
        write_shard(self.old / "a.shard.zst", "selfplay:m@64",
                    [(dict(seed=2, root_values=[[0, nan]]), [sample(0, 1, [(0, 0.5), (1, 0.5)]), sample(1, 2, [(2, 1.0)])])])

    def tearDown(self):
        self.tmp.cleanup()

    def test_searched_q_and_learner(self):
        data = dataset.load([self.new], log=lambda *_: None)
        self.assertEqual(data.header["format"], 2)
        S, N = dataset.SEARCHED, dataset.NOT_SEARCHED
        self.assertEqual(data.searched.tolist(), [S, N, N])
        self.assertEqual(data.learner.tolist(), [1, 1, 0])
        self.assertEqual(data.visit_offset.tolist(), [0, 2, 3, 4])
        # The root values of the searched sub-step's options, by token (the step's options start at token 4).
        self.assertEqual(data.q_offset.tolist(), [0, 2, 2, 2])
        self.assertEqual(data.q_token.tolist(), [4, 5])
        np.testing.assert_allclose(data.q_value, [-0.5, 0.125])
        np.testing.assert_allclose(data.visit_prob, [0.25, 0.75, 1.0, 1.0])
        self.assertEqual(data.directories[0]["per_sample_searched"], 3)
        # An earlier champion's decision is no policy target; the learner's are, as before.
        self.assertEqual(data.policy.tolist(), [1, 1, 0])

    def test_a_window_mixes_the_formats(self):
        data = dataset.load([self.new, self.old], log=lambda *_: None)
        self.assertEqual(len(data), 5)
        self.assertEqual(data.header["format"], 2)
        # Format 1 says nothing per sample: decision 0 has a root value (a null one), so its sample is searched, by decision.
        S, N = dataset.SEARCHED, dataset.NOT_SEARCHED
        self.assertEqual(data.searched.tolist(), [S, N, N, S, N])
        self.assertEqual(data.learner.tolist(), [1, 1, 0, 1, 1])
        self.assertEqual(np.diff(data.q_offset).tolist(), [2, 0, 0, 0, 0])
        self.assertEqual([d["per_sample_searched"] for d in data.directories], [3, 0])

    def test_the_reader_takes_both_formats(self):
        import shards
        _, games = shards.read(next(iter(shards.shards(self.new))))
        g = next(games)
        self.assertEqual([s.searched for s in g.samples], [True, False, False])
        self.assertEqual(g.samples[0].option_q, [(0, -0.5), (1, 0.125)])
        self.assertEqual(g.samples[1].option_q, [])
        _, games = shards.read(next(iter(shards.shards(self.old))))
        self.assertEqual([s.searched for s in next(games).samples], [None, None])

    def test_a_scan_skips_format_2_samples(self):
        import shards
        write_shard(self.new / "b.shard.zst", "selfplay:m@64",
                    [(dict(seed=s), [sample2(0, 1, [(0, 0.5), (1, 0.5)], True, [0.1, 0.2])] * 3) for s in range(4)], fmt=2)
        _, games = shards.scan(self.new / "b.shard.zst")
        seeds = []
        for i, g in enumerate(games):
            seeds.append(g.meta["seed"])
            if i == 2:
                self.assertEqual(len(g.samples), 3)
                self.assertTrue(all(s.searched for s in g.samples))
        self.assertEqual(seeds, [0, 1, 2, 3])


def _corpus(root: Path) -> list[str]:
    """Directories of several shards each, of games among different decks with single-option positions, opponents'
    hands, visit distributions, root values, fast decisions and damage assignments; one of another card index, two
    shards older than the rest. Their specs, with weights."""
    rng = np.random.default_rng(11)
    deck_names = ["dimir", "izzet", "azorius"]
    specs = []
    for name, weight, n_shards, cards in (("a", 1.0, 3, ["Opt", "Shock"]), ("b", 0.5, 3, ["Opt", "Shock"]),
                                          ("c", 0.4, 2, ["Bolt", "Shock", "Opt"]), ("d", 2.0, 2, ["Opt", "Shock"])):
        (root / name).mkdir()
        for k in range(n_shards):
            games = []
            for g in range(int(rng.integers(6, 14))):
                samples = []
                for decision in range(int(rng.integers(1, 6))):
                    options = 1 if rng.random() < 0.4 else 3
                    visits = [(0, 1.0)] if options == 1 else [(o, 1 / 3) for o in range(3)] if rng.random() < 0.5 else []
                    ask = dataset.DAMAGE_ASK if rng.random() < 0.3 else 1
                    hand = tuple(int(c) for c in rng.integers(0, len(cards) + 2, int(rng.integers(0, 3))))
                    s = bytearray(sample(decision, 0, visits, options=options, cards=(1, 2), hand=hand, ask=ask))
                    if options == 1:  # one legal option: the mask's one bit
                        s[19 + 6 * len(visits) + 2 + 4 * len(hand) + 4 + 6] = 1
                    samples.append(bytes(s))
                decks = [deck_names[int(i)] for i in rng.integers(0, 3, 2)]
                meta = dict(seed=g, decks=decks, root_values=[[0, 0.5], [1, None]], fast_decisions=[2],
                            heuristic_values=[[1, 0.25]], distill_values=[[0, -0.5]])
                games.append((meta, samples))
            path = root / name / f"{k}.shard.zst"
            write_shard(path, f"p{name}", games, cards=cards)
            mtime = 1_700_000_000.0 if (name, k) in (("a", 2), ("b", 1)) else 1_800_000_000.0
            os.utime(path, (mtime, mtime))
        specs.append(f"{root / name}:{weight}")
    return specs


class ParallelLoadTest(unittest.TestCase):
    def test_parallel_reads_are_the_serial_data(self):
        with tempfile.TemporaryDirectory() as tmp:
            specs = _corpus(Path(tmp))
            cases = [
                dict(),
                dict(decks={"dimir", "izzet"}, drop_damage_before=1_750_000_000.0, seed=3),
                dict(decks={"dimir", "izzet"}, max_games=4, seed=1, base=["Shock"]),
                dict(max_games=7, trivial=0.3, drop_damage_before=1_750_000_000.0, seed=5),
                dict(chunk=(1, 2), trivial=0.5, decks={"dimir", "azorius"}, seed=2),
            ]
            for case in cases:
                with self.subTest(case=sorted(case)):
                    loaded = []
                    for workers in (0, 3):
                        decks, logs = [], []
                        data = dataset.load(specs, log=logs.append, game_decks=decks, workers=workers, **case)
                        loaded.append((data, decks, logs))
                    (serial, serial_decks, serial_logs), (parallel, parallel_decks, parallel_logs) = loaded
                    self.assertGreater(len(serial), 0)
                    arrays = [k for k, v in serial.__dict__.items() if isinstance(v, np.ndarray)]
                    self.assertIn("blob", arrays)
                    for k in arrays:
                        a, b = getattr(serial, k), getattr(parallel, k)
                        self.assertEqual(a.dtype, b.dtype, k)
                        np.testing.assert_array_equal(a, b, err_msg=k)
                    for k in ("header", "seeds", "games", "directories", "names"):
                        self.assertEqual(getattr(serial, k), getattr(parallel, k), k)
                    self.assertEqual(len(serial.luts), len(parallel.luts))
                    for a, b in zip(serial.luts, parallel.luts):
                        np.testing.assert_array_equal(a, b)
                    self.assertEqual(serial_decks, parallel_decks)
                    self.assertEqual(serial_logs, parallel_logs)
            # What the cases exercise: a weight's choice of games, the decks, the cap, the older shard's damage
            # assignments, the thinning of single-option positions.
            quiet = dict(log=lambda *_: None, workers=0)
            data = dataset.load(specs, decks={"dimir", "izzet"}, drop_damage_before=1_750_000_000.0, **quiet)
            self.assertLess(data.directories[1]["games"], dataset.load([specs[1]], **quiet).games)
            self.assertGreater(sum(d["dropped_damage"] for d in data.directories), 0)
            capped = dataset.load(specs, max_games=4, **quiet)
            self.assertTrue(all(d["games"] <= 4 for d in capped.directories))
            thinned = dataset.load(specs, trivial=0.3, **quiet)
            whole = dataset.load(specs, **quiet)
            self.assertLess(len(thinned), len(whole))
            self.assertEqual(thinned.games, whole.games)

    def test_a_shard_that_fails_fails_the_parallel_load_as_the_serial(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "a").mkdir()
            (root / "b").mkdir()
            write_shard(root / "a" / "0.shard.zst", "p", [(dict(seed=1), [sample(0, 0, [(0, 1.0)])])])
            # An older shard naming no card index, with none above it; then one that is no shard at all.
            write_shard(root / "b" / "0.shard.zst", "p", [(dict(seed=1), [sample(0, 0, [(0, 1.0)])])], cards=None)
            (root / "b" / "1.shard.zst").write_bytes(b"not a shard")
            raised = []
            for workers in (0, 3):
                with self.assertRaises(SystemExit) as caught:
                    dataset.load([root / "a", f"{root / 'b'}:0.5"], log=lambda *_: None, workers=workers)
                raised.append(str(caught.exception))
            self.assertEqual(raised[0], raised[1])
            self.assertIn("names no card index", raised[0])
            (root / "b" / "0.shard.zst").unlink()
            for workers in (0, 3):
                with self.assertRaises(Exception):
                    dataset.load([root / "a", f"{root / 'b'}:0.5"], log=lambda *_: None, workers=workers)

    def test_a_scan_skips_what_a_read_reads(self):
        with tempfile.TemporaryDirectory() as tmp:
            _corpus(Path(tmp))
            for path in sorted(Path(tmp).glob("*/*.shard.zst")):
                header, read = dataset.shards.read(path)
                every = list(read)
                again, games = dataset.shards.scan(path)
                self.assertEqual(header, again)
                n = 0
                for i, g in enumerate(games):
                    self.assertEqual(g.meta, every[i].meta)
                    self.assertEqual(g.count, len(every[i].samples))
                    if i % 2:
                        self.assertEqual(g.samples, every[i].samples)
                    n += 1
                self.assertEqual(n, len(every))


def _sums(data) -> dict:
    """What a worker reads of each array (module level, so a spawned process can be handed it)."""
    return {k: float(np.nansum(v.astype(np.float64))) for k, v in data.__dict__.items() if isinstance(v, np.ndarray)}


if __name__ == "__main__":
    unittest.main()
