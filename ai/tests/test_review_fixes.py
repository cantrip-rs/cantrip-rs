"""The fixes of the review of the AI integration branch (2026-11-13): belief training's batching, reanalysed games as
policy targets held out with their source, and a visit target that names one option twice."""

from __future__ import annotations

import gc
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
import torch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from cantrip_ai import belief, dataset, train  # noqa: E402
from test_dataset import write_shard  # noqa: E402
from test_review_train import imitation_game, position_of, raw_sample, selfplay_game, token  # noqa: E402

SELF = 1


class BeliefBatches(unittest.TestCase):
    def test_belief_training_deals_its_positions(self):
        n = 37
        lengths = np.array([5 + (i * 7) % 60 for i in range(n)], np.int32)
        data = belief.Decisions(blob=np.zeros(0, np.uint8), offset=np.zeros(n + 1, np.int64), lut_of=np.zeros(n, np.uint16),
                                luts=[], hand_offset=np.zeros(n + 1, np.int64), hand=np.zeros(0, np.uint32),
                                turn=np.ones(n, np.int32), game=np.arange(n, dtype=np.int32), length=lengths, games=n,
                                matchup=["?"], pair=np.zeros(n, np.uint16))
        view = belief.batch_view(data)
        rows = np.arange(n)
        # As belief.run trains (shuffled) and evaluates (in order): every position once, each batch of one length bin.
        for rng in (np.random.default_rng(0), None):
            dealt = dataset.batches(view, rows, 8, rng)
            self.assertEqual(sorted(np.concatenate(dealt).tolist()), rows.tolist())
            for b in dealt:
                self.assertLessEqual(len(b), 8)
                self.assertEqual(len(set(((lengths[b] + dataset.GRAIN - 1) // dataset.GRAIN).tolist())), 1)


def reanalysed_game(seed: int) -> tuple[dict, list[bytes]]:
    """`selfplay_game(seed)`'s decision 1 (no full search answered it) searched again by `cantrip-reanalyse`."""
    p = position_of([token("card", 1, "battlefield", SELF, SELF)])
    return dict(seed=seed, root_values=[[1, 0.4]], reanalysed=True), [raw_sample(p, 1, chosen=1, visits=((0, 0.3), (1, 0.7)))]


class Reanalysed(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(ignore_cleanup_errors=True)
        root = Path(self.tmp.name)
        self.dirs = [root / "selfplay", root / "imitation", root / "reanalysed"]
        for d in self.dirs:
            d.mkdir()
        write_shard(self.dirs[0] / "a.shard.zst", "selfplay:m@64", [selfplay_game(s) for s in range(1, 41)])
        write_shard(self.dirs[1] / "a.shard.zst", "tactical:x", [imitation_game(100 + s) for s in range(10)])
        write_shard(self.dirs[2] / "a.shard.zst", "reanalyse:m@4096", [reanalysed_game(s) for s in range(1, 41)])

    def tearDown(self):
        gc.collect()
        self.tmp.cleanup()

    def load(self, dirs=None):
        return dataset.load(dirs or self.dirs, log=lambda *_: None, workers=0)

    def test_a_reanalysed_sample_is_a_policy_target_alone(self):
        data = self.load()
        again = data.reanalysed.astype(bool)
        self.assertEqual(int(again.sum()), 40)
        self.assertTrue((data.searched[again] == dataset.SEARCHED).all())
        self.assertTrue(dataset.search_targets(data)[again].all(), "a reanalysed sample is a search target")
        rows = np.arange(len(data))
        real, fresh = dataset.unmark(data, rows)
        np.testing.assert_array_equal(real, rows)
        np.testing.assert_array_equal(fresh, ~again, "its value, aux, belief and heuristic targets are skipped")
        # Dealt again (search_repeat), a reanalysed sample's copies are policy-only as well.
        dealt = np.concatenate(dataset.batches(data, rows, 64, np.random.default_rng(1), search_repeat=3.0))
        real, fresh = dataset.unmark(data, dealt)
        self.assertFalse(fresh[again[real]].any())
        # Training reads it through the copies' mask: the value loss is the other rows' alone.
        b = train.collate(data)(rows)
        np.testing.assert_array_equal(b["fresh"].numpy(), (~again).astype(np.float32))
        # Without reanalysed games the columns are zero and nothing is masked (the defaults as they were).
        plain = self.load(self.dirs[:2])
        self.assertFalse(plain.reanalysed.any())
        self.assertTrue(dataset.unmark(plain, np.arange(len(plain)))[1].all())
        self.assertNotIn("fresh", train.collate(plain)(np.arange(len(plain))))

    def test_a_reanalysed_game_is_held_out_with_its_source(self):
        data = self.load()
        self.assertEqual(data.games, 90)
        source = {int(k): g for g, k in enumerate(data.game_key) if not data.game_reanalysed[g]}
        copies = np.flatnonzero(data.game_reanalysed)
        self.assertEqual(len(copies), 40)
        self.assertTrue(all(int(data.game_key[g]) in source for g in copies), "a copy's key is its source's")
        for seed in range(12):
            train_rows, held_rows = dataset.split(data, 0.3, seed)
            held_games = set(data.game[held_rows].tolist())
            self.assertTrue(held_games and len(held_games) < data.games)
            for g in copies:
                self.assertEqual(g in held_games, source[int(data.game_key[g])] in held_games)
        # Without reanalysed games, each game is its own draw, as before.
        plain = self.load(self.dirs[:2])
        rng = np.random.default_rng(5)
        out = rng.random(plain.games) < 0.3
        np.testing.assert_array_equal(dataset.split(plain, 0.3, 5)[1], np.flatnonzero(out[plain.game]))


class SharedToken(unittest.TestCase):
    def test_an_option_named_twice_keeps_its_mass(self):
        tmp = tempfile.TemporaryDirectory(ignore_cleanup_errors=True)
        try:
            d = Path(tmp.name)
            p = position_of([token("card", 1, "battlefield", SELF, SELF)], options=3)
            # As the recorder wrote two options of an equal search token: both at the first's index.
            game = dict(seed=1, root_values=[[0, 0.1]]), [raw_sample(p, 0, visits=((0, 0.25), (0, 0.25), (2, 0.5)))]
            write_shard(d / "a.shard.zst", "selfplay:m@64", [game])
            data = dataset.load([d], log=lambda *_: None, workers=0)
            rows = np.arange(len(data))
            labels = dataset.labels(data, rows, dataset.decode(data.blob, data.offset, rows))
            first = data.blob[data.offset[0] + 3]
            np.testing.assert_allclose(labels["visits"][0, first : first + 3], [0.5, 0.0, 0.5])
            self.assertAlmostEqual(float(labels["visits"][0].sum()), 1.0, places=6)
        finally:
            gc.collect()
            tmp.cleanup()


if __name__ == "__main__":
    unittest.main()
