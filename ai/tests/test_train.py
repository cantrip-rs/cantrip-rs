"""A chunked training run scores its fixed evaluation set (--eval-data) on the same positions at every evaluation,
through its chunks and a resumption, onto the card index its network was built with.

    nix-shell ai/shell.nix --run "python -m unittest discover -s ai/tests"
"""

from __future__ import annotations

import contextlib
import gc
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

import torch

TESTS = Path(__file__).resolve().parent
sys.path.insert(0, str(TESTS.parent))
sys.path.insert(0, str(TESTS))
from cantrip_ai import model, train  # noqa: E402
from test_dataset import sample, write_shard  # noqa: E402


def games(count: int, first_seed: int, cards: tuple) -> list:
    return [(dict(seed=first_seed + g), [sample(d, (g + d) % 3, [((g + d) % 3, 1.0)], cards=cards) for d in range(3)])
            for g in range(count)]


class TrainEvalDataTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(ignore_cleanup_errors=True)
        root = Path(self.tmp.name)
        self.data, self.eval = root / "train", root / "eval"
        self.data.mkdir()
        self.eval.mkdir()
        # Two shards, so two chunks; the training data names Opt and Shock.
        write_shard(self.data / "a.shard.zst", "p", games(12, 0, (1, 2)), cards=["Opt", "Shock"])
        write_shard(self.data / "b.shard.zst", "p", games(12, 100, (2, 1)), cards=["Opt", "Shock"])
        # The fixed set names a card the training data does not.
        write_shard(self.eval / "a.shard.zst", "p", games(7, 500, (1, 2, 1)), cards=["Lightning Bolt", "Opt"])
        self.root = root

    def tearDown(self):
        gc.collect()
        self.tmp.cleanup()

    def run_train(self, out: Path, *extra: str, eval_data: bool = True):
        argv = ["--data", str(self.data), "--out", str(out), "--chunks", "2", "--eval-every", "4", "--batch", "4",
                "--width", "16", "--layers", "1", "--heads", "1", "--ffn", "32", "--threads", "1", "--workers", "0",
                "--device", "cpu", "--progress", "off", "--warmup", "2", "--held-out", "0.2", "--checkpoint-every", "0",
                *extra]
        if eval_data:
            argv += ["--eval-data", str(self.eval), "--eval-data-samples", "15"]
        with contextlib.redirect_stdout(io.StringIO()):
            train.main(argv)
        gc.collect()

    def history(self, out: Path) -> list:
        return json.loads((out / "metrics.json").read_text())["history"]

    def test_fixed_eval_through_chunks_and_a_resumption(self):
        out = self.root / "out"
        self.run_train(out, "--steps", "12")
        first = self.history(out)
        self.assertEqual([r["step"] for r in first], [4, 8, 12])
        self.run_train(out, "--steps", "20", "--resume")
        history = self.history(out)
        self.assertEqual([r["step"] for r in history], [4, 8, 12, 16, 20])
        self.assertEqual(history[:2], first[:2], "a resumption keeps the records before it")
        log = (out / "train.log").read_text()
        self.assertIn("chunk 1 of 2", log)
        self.assertIn("chunk 2 of 2", log)
        self.assertIn("resuming", log)
        # Every evaluation scored the fixed set: the same 15 positions of its 21 each time, chunk after chunk.
        for r in history:
            self.assertEqual(r["fixed_eval"]["samples"], 15, r["step"])
            self.assertEqual(r["fixed_eval"]["choices"], history[0]["fixed_eval"]["choices"], r["step"])
        for step in (4, 8, 12, 16):
            self.assertIn(f"\nfixed eval at step {step}: ", log)
        self.assertIn("\nfinal fixed eval at step 20: ", log)

        # The checkpoint's index is the training data's names, then the fixed set's own; scored afresh on that index
        # (the set loaded first, onto the checkpoint's names), the network gives the numbers the run recorded.
        checkpoint = torch.load(out / "checkpoint.pt", map_location="cpu", weights_only=False)
        self.assertEqual(checkpoint["cards"], ["Opt", "Shock", "Lightning Bolt"])
        self.assertEqual(checkpoint["eval_data"]["evaluated"], 15)
        net = model.from_state(model.Config(**checkpoint["config"]), checkpoint["state"])
        net.load_state_dict(checkpoint["state"])
        data, rows = train.eval_set([str(self.eval)], 600, 15, 0, checkpoint["cards"])
        self.assertEqual(data.names, checkpoint["cards"])
        again = train.evaluate(net, data, train.dataset.batches(data, rows, 512, None), 0, False, torch.device("cpu"))
        recorded = history[-1]["fixed_eval"]
        for k in ("samples", "choices", "top1"):
            self.assertEqual(again[k], recorded[k], k)
        for k in ("policy_ce", "value_mse", "swing", "dead"):
            self.assertAlmostEqual(again[k], recorded[k], places=5, msg=k)

    def test_the_fixed_set_does_not_shape_the_network(self):
        """Its own card names are added to a fresh network's table as new cards: the run trains as it would without it."""
        with_set, without = self.root / "with", self.root / "without"
        self.run_train(with_set, "--steps", "8")
        self.run_train(without, "--steps", "8", eval_data=False)
        a, b = self.history(with_set), self.history(without)
        self.assertNotIn("fixed_eval", b[-1])
        for k in ("policy_ce", "value_mse", "top1"):
            self.assertAlmostEqual(a[-1]["held_out"][k], b[-1]["held_out"][k], places=6, msg=k)


if __name__ == "__main__":
    unittest.main()
