"""The generation driver's plan (a dry run on a temp root), its window and its reading of a gate."""

from __future__ import annotations

import contextlib
import io
import json
import sys
import tempfile
import unittest
import unittest.mock
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from cantrip_ai import generation  # noqa: E402

GATE = """
SPRT verdict after 412 games: LLR 2.951 (232 to 180, 0 drawn).

neural@64 beat neural@64 240 to 188, 2 drawn (56.1 % +/- 4.7 %).
430 games in 3600.0s (0 games/s), 14.0 turns a game; whoever played first won 53.0 %.
SPRT (H0 50 %, H1 55 %, alpha = beta = 0.05, bounds +/-2.944): LLR 3.120 after 430 games; H1 accepted, neural@64 is the better at game 412 (LLR 2.951; games already in play finished after).
"""


def root_with(tmp: Path, gens=(1, 2), finished=True) -> Path:
    (tmp / "models" / "gen0").mkdir(parents=True)
    (tmp / "models" / "gen0" / "manifest.json").write_text("{}")
    for pairing in ("matchup", "rw-mirror"):
        (tmp / "gen0" / pairing).mkdir(parents=True)
        (tmp / "gen0" / pairing / "a.shard.zst").write_bytes(b"x")
    for n in gens:
        (tmp / f"gen{n}" / "selfplay").mkdir(parents=True)
        (tmp / f"gen{n}" / "selfplay" / "a.shard.zst").write_bytes(b"x")
        record = dict(generation=n, started="then", stages={})
        if finished:
            record["finished"] = "then"
        (tmp / f"gen{n}" / "generation.json").write_text(json.dumps(record))
    return tmp


class GenerationTest(unittest.TestCase):
    def test_window_weights_the_newest_most(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=range(1, 12))
            w = generation.window(root, 11, 8, 0.5, 0.2)
            self.assertEqual([(d.parent.name, x) for d, x in w[:2]], [("gen11", 1.0), ("gen10", 0.5)])
            self.assertEqual(len([d for d, _ in w if d.name == "selfplay"]), 8)
            self.assertEqual([d.name for d, x in w if x == 0.2], ["matchup", "rw-mirror"])

    def test_reads_a_gate(self):
        g = generation.parse_gate(GATE)
        self.assertEqual((g["wins"], g["losses"], g["draws"], g["games"]), (240, 188, 2, 430))
        self.assertAlmostEqual(g["share"], 0.561)
        self.assertEqual((g["verdict"], g["stopped_at"]), ("H1", 412))
        self.assertTrue(generation.accepted(g, "sprt"))
        self.assertTrue(generation.accepted(g, "ahead"))
        h0 = generation.parse_gate(GATE.replace("H1 accepted, neural@64 is the better", "H0 accepted, neural@64 is not the better by 55%"))
        self.assertFalse(generation.accepted(h0, "sprt"))

    def test_next_generation_resumes_an_interrupted_one(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1, 2))
            self.assertEqual(generation.next_generation(root), 3)
            (root / "gen3").mkdir()
            (root / "gen3" / "generation.json").write_text(json.dumps(dict(generation=3, stages=dict(build=dict(done=True)))))
            self.assertEqual(generation.next_generation(root), 3, "an interrupted generation is resumed, not skipped")
            (root / "gen3" / "generation.json").write_text(json.dumps(dict(generation=3, stages={}, finished="then")))
            self.assertEqual(generation.next_generation(root), 4)

    def test_a_finished_gate_in_the_log_is_recorded_and_promoted(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            champion = root / "champions" / "gen0"
            (champion / "train").mkdir(parents=True)
            (champion / "train" / "checkpoint.pt").write_bytes(b"x")
            (champion / "manifest.json").write_text("{}")
            (root / "champions" / "current").symlink_to("gen0")
            model = root / "models" / "gen2"
            (model / "train").mkdir(parents=True)
            (model / "train" / "checkpoint.pt").write_bytes(b"x")
            (model / "manifest.json").write_text("{}")
            stages = {n: dict(done=True, seconds=1.0) for n in ("build", "selfplay", "train", "export", "parity")}
            stages["selfplay"].update(games=4, simulations=8, lanes=1, games_per_hour=4.0, samples=40, visit_samples=8,
                                      capped=0, dropped=0, fallbacks=0, first_player=0.5, seeds="1..5")
            stages["train"].update(steps=10, seen=2560, init="gen0", window=[], value_mix=0.0, held_out=dict(value_mse=0.5))
            stages["export"].update(model="models/gen2")
            stages["parity"].update(summary="ok")
            rec = generation.Record(root / "gen2", 2, False)
            rec.data.update(stages=stages, champion="gen0")
            rec.save()
            (root / "gen2" / "logs").mkdir()
            (root / "gen2" / "logs" / "gate.log").write_text(
                "neural@64 against neural@64, 1200 games, 1 decks, each a mirror\n" + GATE +
                "\nneural@64/gen0: the network's 1 answers\nneural@64/gen2: the network's 1 answers\n")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--generation", "2", "--no-build", "--decks-dir", "", "--accept", "sprt"])
            record = json.loads((root / "gen2" / "generation.json").read_text())
            self.assertTrue(record["stages"]["gate"]["done"])
            self.assertEqual((record["stages"]["gate"]["wins"], record["stages"]["gate"]["losses"]), (240, 188))
            self.assertTrue(record["stages"]["promote"]["accepted"])
            self.assertIn("reusing the finished gate", out.getvalue())
            self.assertEqual((root / "champions" / "current").resolve().name, "gen2")
            self.assertTrue((root / "champions" / "gen2" / "CHAMPION.json").exists())

    def test_current_moves_from_champion_to_champion(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for name in ("gen0", "gen1"):
                (root / "champions" / name).mkdir(parents=True)
                (root / "champions" / name / "manifest.json").write_text(name)
            generation.set_current(root, "gen0")
            generation.set_current(root, "gen1")
            self.assertEqual(generation.champion(root).name, "gen1")
            self.assertEqual((root / "champions" / "current" / "manifest.json").read_text(), "gen1")
            self.assertEqual((root / "champions" / "gen0" / "manifest.json").read_text(), "gen0", "the old champion stays")

    @unittest.skipUnless(sys.platform == "win32", "a junction stands in for a symlink on Windows only")
    def test_current_is_a_junction_where_windows_makes_no_symlink(self):
        def refuse(*_, **__):
            raise OSError("A required privilege is not held by the client")

        with tempfile.TemporaryDirectory() as tmp, unittest.mock.patch.object(Path, "symlink_to", refuse):
            root = Path(tmp)
            for name in ("gen0", "gen1"):
                (root / "champions" / name).mkdir(parents=True)
                (root / "champions" / name / "manifest.json").write_text(name)
            generation.set_current(root, "gen0")
            self.assertEqual(generation.champion(root).name, "gen0")
            generation.set_current(root, "gen1")
            self.assertEqual(generation.champion(root).name, "gen1")
            self.assertEqual((root / "champions" / "gen0" / "manifest.json").read_text(), "gen0", "the old champion stays")

    def test_a_dry_run_plans_every_stage_and_writes_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            before = sorted(p for p in root.rglob("*"))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--games", "100", "--simulations", "32", "--decks-dir", ""])
            text = out.getvalue()
            self.assertEqual(sorted(p for p in root.rglob("*")), before, "a dry run writes nothing")
            self.assertIn("generation 2", text)
            self.assertRegex(text, r"cantrip-selfplay(\.exe)? --model \S+champions[/\\]gen0 --out \S+gen2[/\\]selfplay --games 100 --simulations 32")
            self.assertRegex(text, r"gen2[/\\]selfplay:1 \S+gen1[/\\]selfplay:0.5 \S+matchup:0.2 \S+rw-mirror:0.2")
            self.assertIn("--init " + str(root / "champions" / "gen0" / "train" / "checkpoint.pt"), text)
            self.assertIn("model_parity", text)
            self.assertRegex(text, r"duel --a neural@32 --b neural@32 --model \S+models[/\\]gen2 --model-b \S+champions[/\\]gen0 .*--mirrors .*--sprt")

    def test_a_dry_run_resumes_partial_selfplay_and_training(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            (root / "gen2" / "selfplay").mkdir(parents=True)
            (root / "gen2" / "selfplay" / "a.shard.zst").write_bytes(b"x")
            (root / "models" / "gen2" / "train").mkdir(parents=True)
            (root / "models" / "gen2" / "train" / "checkpoint.pt").write_bytes(b"x")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--generation", "2", "--dry-run", "--games", "100", "--simulations", "32", "--decks-dir", ""])
            text = out.getvalue()
            self.assertRegex(text, r"cantrip-selfplay(\.exe)? .*--shard-games 100.*--resume")
            self.assertRegex(text, r"cantrip_ai.train .*--resume")
            self.assertNotRegex(text, r"cantrip_ai.train .*--init ")

    def test_retrain_sets_a_rejected_attempt_aside(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            gen1 = root / "gen1"
            (gen1 / "logs").mkdir()
            for stage in ("selfplay", "train", "gate"):
                (gen1 / "logs" / f"{stage}.log").write_text(stage)
            (root / "models" / "gen1" / "train").mkdir(parents=True)
            (root / "models" / "gen1" / "train" / "metrics.json").write_text("{}")
            stages = {n: dict(done=True, seconds=60.0) for n in generation.STAGES}
            stages["selfplay"].update(games=4, simulations=8, lanes=1, games_per_hour=4.0, samples=40, visit_samples=8, capped=0,
                                      dropped=0, fallbacks=0, first_player=0.5, seeds="1..5")
            stages["train"].update(steps=10, seen=2560, init="gen0", window=[], value_mix=0.0, held_out=dict(value_mse=0.5))
            stages["export"].update(model="models/gen1")
            stages["parity"].update(summary="ok")
            stages["gate"].update(wins=6, losses=4, draws=0, games=10, share=0.6, margin=0.3, llr=0.1, sprt="no verdict",
                                  champion="gen0", simulations=8, decks="rw-aggro")
            stages["promote"].update(accepted=False, champion="gen0", rule="sprt")
            stages["retire"].update(deleted=0, keep=8)
            rec = generation.Record(gen1, 1, False)
            rec.data.update(stages=stages, champion="gen0", finished="then")
            rec.save()

            class Args:
                retrain, dry_run = "value overfit", False

            with contextlib.redirect_stdout(io.StringIO()):
                generation.retrain(rec, gen1, root / "models" / "gen1", Args)
            self.assertEqual(sorted(rec.data["stages"]), ["build", "selfplay"], "self-play is kept")
            self.assertEqual(rec.data["attempts"][0]["reason"], "value overfit")
            self.assertTrue((gen1 / "logs" / "attempt1" / "gate.log").exists())
            self.assertTrue((gen1 / "logs" / "attempt1" / "metrics.json").exists())
            self.assertTrue((gen1 / "logs" / "selfplay.log").exists())
            self.assertFalse((root / "models" / "gen1").exists())
            self.assertIn("| attempt 1 |", (gen1 / "README.md").read_text())
            with self.assertRaises(SystemExit):  # the attempt in progress is not rejected yet
                generation.retrain(rec, gen1, root / "models" / "gen1", Args)


if __name__ == "__main__":
    unittest.main()
