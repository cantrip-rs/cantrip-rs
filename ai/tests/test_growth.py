"""The growth report reads a generation's record, self-play's report, the server's log and its training, and a bare
training directory given by --models, and puts their numbers in its tables."""

from __future__ import annotations

import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from cantrip_ai import growth  # noqa: E402


def held(ce, mse, top1=0.9, choices=180):
    return dict(samples=200, choices=choices, top1=top1, policy_ce=ce, value_mse=mse)


def write(path: Path, text: str):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(text.encode("utf-8"))


TRAIN_LOG = """device cuda (test)
precision bf16 autocast
model {'width': 256, 'layers': 8, 'heads': 4, 'ffn': 1024}: 7158278 parameters
step 200/400 epoch 0 lr 2.0e-04 train ce 0.5 top1 0.8 value 0.9 4000 samples/s, 0 skipped, peak 3.1 GiB
step 400/400 epoch 0 lr 1.0e-04 train ce 0.4 top1 0.8 value 0.8 5000 samples/s, 0 skipped, peak 3.4 GiB
step 600/400 epoch 0 lr 1.0e-04 train ce 0.4 top1 0.8 value 0.8 6000 samples/s, 0 skipped, peak 3.2 GiB
"""


class GrowthTest(unittest.TestCase):
    def test_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp) / "fp"
            gen = root / "gen1"
            record = dict(generation=1, started="x", finished="y", stages=dict(
                build=dict(done=True, seconds=1.0),
                selfplay=dict(done=True, seconds=10.0, games=5000, games_per_hour=111.0, searched_per_second=2.0,
                              capped=0, dropped=0, lanes=64),
                train=dict(done=True, seconds=20.0, steps=4067, seen=1041152, size="384x10", parameters=12345678,
                           seconds_training=7200.0, precision="bf16", init="gen0", window=[],
                           held_out=held(0.1234, 0.5678, 0.9512)),
                gate=dict(done=True, seconds=30.0, wins=1900, losses=1800, draws=0, games=3700, share=0.5132,
                          margin=0.016, verdict="H1", llr=2.95),
                promote=dict(done=True, seconds=0.1, accepted=True, champion="gen1", rule="sprt")))
            write(gen / "generation.json", json.dumps(record))
            write(gen / "selfplay" / "selfplay-report.json", json.dumps(dict(
                Games=5000, GamesPerHour=4321.5, SearchedPerSecond=98.76, EvaluationsPerSecond=5000.0, Runaways=3, Capped=1)))
            write(gen / "logs" / "server-selfplay.log",
                  "infer: 1 model(s) on cuda:0\n"
                  "infer: 10 requests, 100 rows (900 a second) in 5 batches (T32: 0 rows in 0), 50% of the padded rows real; "
                  "workers busy 90%, in the network 80%; a request 4.2 ms in the engine, a row group queued 1.3 ms; 32 clients\n"
                  "infer: 20 requests, 200 rows (16569 a second) in 9 batches (T32: 0 rows in 0), 70% of the padded rows real; "
                  "workers busy 108%, in the network 96%; a request 4.2 ms in the engine, a row group queued 1.3 ms; 32 clients\n")
            history = [dict(step=200, seconds=100.0, seen=1_000_000, held_out=held(0.30, 0.70)),
                       dict(step=400, seconds=200.0, seen=3_000_000, held_out=held(0.20, 0.60)),
                       dict(step=500, seconds=250.0, seen=3_500_000)]
            write(root / "models" / "gen1" / "train" / "metrics.json", json.dumps(dict(final=True, history=history)))
            write(root / "models" / "gen1" / "train" / "train.log", TRAIN_LOG)
            # A bare training directory (metrics.json directly in it), its log beside it; trained with a fixed
            # evaluation set (train.py --eval-data), which its convergence rows are read on.
            bare = Path(tmp) / "models" / "grow-256x8-gen0" / "train-unfiltered-59200"
            write(bare / "metrics.json", json.dumps(dict(final=False, history=[
                dict(step=200, seconds=50.0, seen=1_000_000, held_out=held(0.40, 0.80), fixed_eval=held(0.50, 0.90, choices=300)),
                dict(step=300, seconds=60.0, seen=2_000_000),
                dict(step=400, seconds=3600.0, seen=5_000_000, held_out=held(0.25, 0.65, 0.96),
                     fixed_eval=held(0.30, 0.70, choices=300))])))
            write(bare / "train.log", TRAIN_LOG)

            text = growth.report(root, [bare], [2e6, 5e6, 20e6])
            rows = {line.split(" | ")[0].lstrip("| "): line for line in text.split("## Convergence")[0].splitlines() if line.startswith("| ")}
            g = rows["gen1"]
            for expected in ("finished", "384x10", "12.3M", "bf16", "4067", "5000", "3.4", "2.00", "0.1234", "0.5678",
                             "0.9512", "4322", "98.8", "16569", "70%", "108%", "| 3 |", "51.3% +/- 1.6%, 3700 games, H1", "yes"):
                self.assertIn(expected, g)
            b = rows["grow-256x8-gen0/train-unfiltered-59200"]
            for expected in ("256x8", "7.16M", "bf16", "400", "5000", "3.4", "1.00", "0.2500", "0.6500", "0.9600"):
                self.assertIn(expected, b)

            policy = text.split("### Policy cross-entropy")[1].split("### Value MSE")[0]
            value = text.split("### Value MSE")[1]
            self.assertIn("| 2M | 5M |", policy, "a mark no run reaches (20M) is left out")
            self.assertNotIn("20M", policy)
            # gen1 at 2M: halfway between 0.30 (1M) and 0.20 (3M); 5M is past its last evaluation.
            self.assertIn("| gen1 | 384x10 | held out | 180 | 0.2500 | - | 0.2000 (3M) |", policy)
            # The bare run, on its fixed set: at 2M a quarter of the way from 0.50 to 0.30; at 5M its last.
            self.assertIn("| grow-256x8-gen0/train-unfiltered-59200 | 256x8 | fixed | 300 | 0.4500 | 0.3000 | 0.3000 (5M) |", policy)
            self.assertIn("| gen1 | 384x10 | held out | 180 | 0.6500 | - | 0.6000 (3M) |", value)
            self.assertIn("| grow-256x8-gen0/train-unfiltered-59200 | 256x8 | fixed | 300 | 0.8500 | 0.7000 | 0.7000 (5M) |", value)

    def test_missing_fields(self):
        """A hand-written selfplay record, no training yet, no logs: every number it lacks is a dash."""
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write(root / "gen1" / "generation.json", json.dumps(dict(generation=1, stages=dict(
                selfplay=dict(done=True, seconds=0, games=4999, samples=2397500, note="recorded by hand")))))
            (root / "gen2").mkdir()
            text = growth.report(root, [])
            self.assertIn("| gen1 | selfplay | - |", text)
            self.assertIn("| gen2 | - | - |", text)
            self.assertIn("No training run has a held-out evaluation yet.", text)

    def test_main_writes_the_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            root, out = Path(tmp) / "fp", Path(tmp) / "growth.md"
            (root / "gen3").mkdir(parents=True)
            with contextlib.redirect_stdout(io.StringIO()):
                growth.main(["--root", str(root), "--out", str(out), "--marks", "2M,5M"])
            self.assertIn("| gen3 |", out.read_text(encoding="utf-8"))
            self.assertNotIn(b"\r\n", out.read_bytes())


if __name__ == "__main__":
    unittest.main()
