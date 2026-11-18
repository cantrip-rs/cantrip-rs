"""The generation driver's plan (a dry run on a temp root), its window and its reading of a gate."""

from __future__ import annotations

import contextlib
import io
import json
import subprocess
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


# A gate by pair, as BotLab prints it (Tally.WritePaired) after the unpaired lines.
PAIRED_GATE = """
Paired SPRT verdict after 2950 pairs: LLR 2.951 (pairs 12 / 30 / 2850 / 40 / 18).

neural:decklist@64 beat neural:decklist@64 3060 to 2927, 13 drawn (51.1 % +/- 1.3 %).
6000 games in 5,196.3s (1 games/s), 16.5 turns a game; whoever played first won 52.2 %.

Pairs by neural:decklist@64's points in a pair (0 / 0.5 / 1 / 1.5 / 2): 12 / 30 / 2900 / 40 / 18; 3000 pairs, 0 incomplete; 96.7 % split 1-1.
Paired: neural:decklist@64 scored 50.6 % +/- 0.3 % by pair (95 %, pentanomial); the pairing's variance is 0.07 of the unpaired.
Paired SPRT (pentanomial GSPRT, H0 50.0 %, H1 52.0 %, alpha = 0.05, beta = 0.05, bounds -2.944/2.944): LLR 3.012 after 3000 pairs; H1 accepted, neural:decklist@64 is the better at pair 2950 (LLR 2.951; games already in play finished after).
"""

# A real BotLab duel's paired lines (the tactical bot against itself on three decks, true duplicate pairs, an
# interval stop from 5 pairs; BotLab's own output, 2026-11-13).
INTERVAL_GATE = """
Interval stop after 19 pairs: 34.2 % +/- 14.7 % (pairs 8 / 0 / 9 / 0 / 2).

tactical beat tactical 13 to 26 (33.3 % +/- 14.8 %).
39 games in 26.3s (1 games/s), 18.2 turns a game; whoever played first won 53.8 %.

Pairs by tactical's points in a pair (0 / 0.5 / 1 / 1.5 / 2): 8 / 0 / 9 / 0 / 2; 19 pairs, 1 incomplete; 47.4 % split 1-1.
Paired: tactical scored 34.2 % +/- 14.7 % by pair (95 %, pentanomial); the pairing's variance is 0.95 of the unpaired.
Interval stop (the paired 1.96-sigma interval excludes 50 %, from 5 pairs): stopped at pair 19, tactical behind (34.2 % +/- 14.7 %; games already in play finished after).
"""


def decks_in(tmp: Path) -> Path:
    d = tmp / "decks"
    d.mkdir()
    for name, players in (("aggro", 21), ("control", 19)):
        (d / f"{name}.txt").write_text(f"// {name} ({players} players, 5-0)\n60 Island\n")
    return d


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
        # gen2 of the templates line: 51.1% +/- 1.3% over 6,000, no verdict at H1 52%. Only `above-half` keeps it.
        gen2 = dict(g, share=0.5111, margin=0.013, verdict="none")
        self.assertEqual([generation.accepted(gen2, r) for r in ("sprt", "ahead", "above-half")], [False, False, True])
        self.assertFalse(generation.accepted(dict(gen2, wins=200, losses=200), "above-half"), "exactly a half is no win")

    def test_reads_a_gate_by_pair(self):
        g = generation.parse_gate(PAIRED_GATE)
        self.assertEqual((g["wins"], g["losses"], g["draws"]), (3060, 2927, 13), "the unpaired line reads as before")
        self.assertEqual((g["pentanomial"], g["pairs"], g["incomplete"]), ([12, 30, 2900, 40, 18], 3000, 0))
        self.assertAlmostEqual(g["split"], 2900 / 3000)
        self.assertAlmostEqual(g["paired_share"], 0.506)
        self.assertAlmostEqual(g["paired_margin"], 0.003)
        self.assertEqual((g["paired_llr"], g["paired_verdict"], g["paired_stopped_at"]), (3.012, "H1", 2950))
        self.assertEqual((g["verdict"], g["llr"]), ("none", None), "no unpaired SPRT was run")
        self.assertIsNone(g["interval_stop"])
        self.assertEqual([generation.accepted(g, r) for r in generation.RULES], [False, True, False, True, True])
        h0 = generation.parse_gate(PAIRED_GATE.replace("H1 accepted, neural:decklist@64 is the better", "H0 accepted, neural:decklist@64 is not the better by 52.0 %"))
        self.assertEqual(h0["paired_verdict"], "H0")
        self.assertFalse(generation.accepted(h0, "sprt-paired"))
        no_verdict = generation.parse_gate(PAIRED_GATE.replace("H1 accepted, neural:decklist@64 is the better at pair 2950 (LLR 2.951; games already in play finished after).", "no verdict within the games played."))
        self.assertEqual((no_verdict["paired_verdict"], no_verdict["paired_stopped_at"]), ("none", None))

    def test_above_half_reads_the_paired_share_and_always_promotes(self):
        g = generation.parse_gate(INTERVAL_GATE)
        self.assertEqual((g["pentanomial"], g["pairs"], g["incomplete"]), ([8, 0, 9, 0, 2], 19, 1))
        self.assertEqual((g["interval_stopped_at"], g["interval_way"]), (19, "behind"))
        self.assertFalse(generation.accepted(g, "above-half"))
        self.assertTrue(generation.accepted(g, "always"))
        # The pairs decide above-half where there are some, the games' wins and losses in an older log; each from the
        # counts, not a rounded share.
        self.assertFalse(generation.accepted(dict(g, share=0.52, wins=30, losses=10, pentanomial=[1, 0, 98, 0, 0]), "above-half"))
        self.assertTrue(generation.accepted(dict(g, share=0.49, wins=10, losses=30, pentanomial=[0, 0, 98, 0, 1]), "above-half"))
        self.assertTrue(generation.accepted(dict(g, share=0.52, wins=27, losses=26, pentanomial=None, paired_share=None), "above-half"))
        # 2999 split pairs and one won: the share prints as 50.0 % and is above a half; one lost, below; none, no win.
        split = dict(g, pentanomial=[0, 0, 2999, 0, 1], paired_share=0.500)
        self.assertTrue(generation.accepted(split, "above-half"))
        self.assertFalse(generation.accepted(dict(split, pentanomial=[1, 0, 2999, 0, 0]), "above-half"))
        self.assertFalse(generation.accepted(dict(split, pentanomial=[0, 1, 2998, 1, 0]), "above-half"))
        self.assertTrue(generation.accepted(dict(split, pentanomial=[0, 1, 2997, 2, 0]), "above-half"))
        # A gate that was never played (--accept always without --gate-measure) promotes under always alone.
        self.assertTrue(generation.accepted(dict(skipped="--accept always"), "always"))
        self.assertFalse(generation.accepted(dict(skipped="--accept always"), "sprt-paired"))

    def test_the_held_out_set_is_chosen_once_and_left_out_of_every_window(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1, 2))
            selfplay = root / "gen2" / "selfplay"
            (selfplay / "a.shard.zst").unlink()
            for lane in range(3):
                for k, games in enumerate((100, 40)):
                    (selfplay / f"selfplay-l{lane}-1-2-{k:05}.shard.zst").write_text(str(games))
            dest = root / "heldout"
            out = generation.designate_heldout(selfplay, dest, 250, 2, False, log=lambda *_: None,
                                               count=lambda p: int(p.read_text()))
            # Each lane's full first shard before any lane's second: 100 + 100 + 100 games.
            self.assertEqual((out["games"], out["shards"]), (300, 3))
            self.assertEqual(sorted(p.name for p in dest.glob("*.shard.zst")),
                             [f"selfplay-l{lane}-1-2-00000.shard.zst" for lane in range(3)])
            self.assertEqual(len(list(selfplay.glob("*.shard.zst"))), 3, "moved, not copied")
            record = json.loads((dest / generation.HELDOUT_RECORD).read_text())
            self.assertEqual((record["generation"], record["games"]), (2, 300))
            with self.assertRaises(SystemExit):  # chosen once
                generation.designate_heldout(selfplay, dest, 10, 2, False, log=lambda *_: None, count=lambda p: 1)
            # A held-out directory that is a window directory (chosen by hand) is never trained on.
            w = generation.window(root, 2, 8, 0.5, 0.2, exclude=[root / "gen1" / "selfplay"])
            self.assertEqual([d.parent.name for d, _ in w if d.name == "selfplay"], ["gen2"])
            self.assertEqual(len(generation.window(root, 2, 8, 0.5, 0.2, exclude=[dest])), 4)

    def test_the_ladder_is_due_every_n_generations_and_kept_under_the_root(self):
        self.assertEqual([generation.ladder_due(g, 2, 3) for g in (1, 2, 4, 6)],
                         [(False, False), (True, False), (True, False), (True, True)])
        self.assertEqual(generation.ladder_due(5, 0, 1), (False, False))
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            duel = generation.parse_gate(PAIRED_GATE)
            generation.ladder_record(root, [dict(duel, generation=2, champion="gen2", anchor="gen0", budget="@64")])
            generation.ladder_record(root, [dict(duel, generation=4, champion="gen3", anchor="tactical", budget="@250ms")])
            rows = generation.ladder_rows(root)
            self.assertEqual([(r["generation"], r["anchor"]) for r in rows], [(2, "gen0"), (4, "tactical")])
            table = (root / "ladder.md").read_text()
            self.assertIn("| 4 | gen3 | tactical | @250ms | 6000 | 50.6% +/- 0.3% | 12 / 30 / 2900 / 40 / 18 | 51.1% +/- 1.3% |", table)

    def test_a_dry_run_plans_the_paired_gate_and_the_measurements(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            decks = decks_in(Path(tmp))
            before = sorted(p for p in root.rglob("*"))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--games", "100", "--simulations", "32", "--decks-dir", str(decks),
                                 "--mode", "decklist", "--accept", "sprt-paired", "--gate-h1", "0.51", "--net-duel-games", "2000",
                                 "--heldout-games", "1000", "--ladder-every", "2", "--ladder-play-every", "1"])
            text = out.getvalue()
            self.assertEqual(sorted(p for p in root.rglob("*")), before, "a dry run writes nothing")
            self.assertRegex(text, r"would move whole shards of \S+gen2[/\\]selfplay holding at least 1000 games to \S+heldout")
            self.assertRegex(text, r"--data \S+gen2[/\\]selfplay:1 ")
            self.assertRegex(text, r"duel --a neural:decklist --b neural:decklist --model \S+models[/\\]gen2 --model-b \S+champions[/\\]gen0 "
                                   r"--decks \S+ --ai-prior \S+ --pairings \S+gen2[/\\]netduel-pairings.txt --games 2000 ")
            self.assertRegex(text, r"cantrip_ai.value_eval --data \S+heldout --every-game --policy --games 1000 .*--checkpoint "
                                   r"gen2=\S+ gen0=\S+ --out \S+heldout-eval.json")
            self.assertRegex(text, r"duel --a neural:decklist@32 --b neural:decklist@32 --model \S+models[/\\]gen2 --model-b \S+champions[/\\]gen0 "
                                   r".*--pairings \S+gen2[/\\]gate-pairings.txt --games 6000 .* --sprt-paired --sprt-h0 0.5 --sprt-h1 0.51 "
                                   r"--sprt-alpha 0.05 --sprt-beta 0.05")
            self.assertNotRegex(text, r"--mirrors")
            # The ladder (generation 2 of every 2nd, at the play budget too): gen0 is the champion in a dry run, so only
            # the tactical bot is an anchor.
            self.assertIn("gen0 is the champion: no ladder duel against itself", text)
            self.assertRegex(text, r"duel --a neural:decklist@32 --b tactical --model \S+champions[/\\]gen0 .*ladder-tactical-32-pairings.txt")
            self.assertRegex(text, r"duel --a neural:decklist@128 --b tactical --model \S+champions[/\\]gen0 ")

    def test_a_dry_run_of_above_half_with_its_early_stop_and_of_always(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            decks = decks_in(Path(tmp))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--decks-dir", str(decks), "--accept", "above-half",
                                 "--gate-early-stop", "--gate-stop-min-pairs", "200"])
            self.assertRegex(out.getvalue(), r"duel --a neural@64 .*--pairings .* --interval-stop --interval-z 2.58 --interval-min-pairs 200 "
                                             r"--interval-min-decisive 30")
            self.assertNotIn("value_eval", out.getvalue(), "no held-out set, no evaluate stage")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--decks-dir", str(decks), "--accept", "always"])
            self.assertIn("not played: --accept always promotes with no gate", out.getvalue())
            self.assertNotIn("duel --a", out.getvalue())
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--decks-dir", str(decks), "--accept", "always", "--gate-measure",
                                 "--gate-pairings", "mirrors"])
            self.assertRegex(out.getvalue(), r"duel --a neural@64 --b neural@64 .*--decks \S+ --ai-prior \S+ --games 6000 ")
            gate = next(line for line in out.getvalue().splitlines() if " duel --a " in line)
            self.assertNotRegex(gate, r"--pairings|--sprt|--interval-stop")

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
                "neural@64 against neural@64, 1200 games, rw-aggro vs ub-reanimator, shuffles from 20000001.\n" + GATE +
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
            # The gate's default rule is above-half (the project's rule): no SPRT, no early stop unless asked.
            gate = next(line for line in text.splitlines() if " duel --a neural@32 --b neural@32 " in line)
            self.assertRegex(gate, r"--model \S+models[/\\]gen2 --model-b \S+champions[/\\]gen0 .*--mirrors ")
            self.assertNotRegex(gate, r"--sprt|--interval-stop")
            self.assertLess(text.index("duel --a neural@32 --b neural@32"), text.index("[promote]"))

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
            self.assertNotRegex(text, r"cantrip-selfplay(\.exe)? .*--traces")
            self.assertRegex(text, r"cantrip_ai.train .*--resume")
            self.assertNotRegex(text, r"cantrip_ai.train .*--init ")

    def test_a_dry_run_with_selfplay_traces_asks_self_play_for_them(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--games", "100", "--simulations", "32", "--decks-dir", "",
                                 "--selfplay-traces"])
            self.assertRegex(out.getvalue(), r"cantrip-selfplay(\.exe)? .*--batch \S+ --traces")

    def test_a_dry_run_adds_extra_data_to_the_training_window(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--games", "100", "--simulations", "32", "--decks-dir", "",
                                 "--extra-data", "X:/mined/targets:8"])
            self.assertIn("extra: X:/mined/targets:8", out.getvalue())

    def test_a_dry_run_passes_search_settings_to_self_play_and_the_gate(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--games", "100", "--simulations", "32", "--decks-dir", "",
                                 "--full-simulations", "256", "--selfplay-search", "Worlds=1", "--selfplay-search",
                                 "Gumbel=true", "--selfplay-full-search", "Worlds=4", "--seat-settings", "+Worlds=1"])
            text = out.getvalue()
            self.assertRegex(text, r"cantrip-selfplay(\.exe)? .*--search Worlds=1 --search Gumbel=true --full-search Worlds=4")
            self.assertRegex(text, r"duel --a neural@32\+Worlds=1 --b neural@32\+Worlds=1 ")

    def test_a_dry_run_passes_the_governor_to_self_play(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--games", "100", "--simulations", "32", "--decks-dir", ""])
            text = out.getvalue()
            self.assertRegex(text, rf"cantrip-selfplay(\.exe)? .*--lanes {generation.AUTOSCALE_LANES} ")
            self.assertRegex(text, r"cantrip-selfplay(\.exe)? .*--memory-reserve 6.0 --commit-reserve 6.0 --memory-floor 2.0")
            self.assertNotRegex(text, r"--memory-budget|--no-autoscale|--scale-window")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--games", "100", "--simulations", "32", "--decks-dir", "",
                                 "--selfplay-memory-budget", "40", "--selfplay-memory-reserve", "8",
                                 "--selfplay-memory-commit-reserve", "10", "--selfplay-memory-floor", "3",
                                 "--no-selfplay-autoscale", "--lanes", "16", "--selfplay-scale-window", "30",
                                 "--selfplay-scale-min-gain", "0.3", "--selfplay-cpu-ceiling", "0.9"])
            text = out.getvalue()
            self.assertRegex(text, r"cantrip-selfplay(\.exe)? .*--lanes 16 ")
            self.assertRegex(text, r"cantrip-selfplay(\.exe)? .*--memory-budget 40.0 --memory-reserve 8.0 --commit-reserve 10.0 "
                                   r"--memory-floor 3.0 --no-autoscale --scale-window 30.0 --scale-min-gain 0.3 --cpu-ceiling 0.9")

    def test_a_dry_run_with_workers_plans_the_split_and_the_rest_unchanged(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            workers = Path(tmp) / "workers.json"
            workers.write_text(json.dumps(dict(key="k", workers=[dict(name="w1", host="10.0.0.2", user="worker", root="D:/compute")])))
            before = sorted(p for p in root.rglob("*"))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--dry-run", "--games", "100", "--simulations", "32", "--decks-dir", "",
                                 "--workers", str(workers)])
            text = out.getvalue()
            self.assertEqual(sorted(p for p in root.rglob("*")), before, "a dry run writes nothing")
            self.assertIn("distributed self-play: 1 worker(s) (w1) and this machine", text)
            self.assertRegex(text, r"worker@10.0.0.2:D:/compute/bundle/<hash> and play in D:/compute/gens/\S+/gen2/w1/<attempt>")
            self.assertRegex(text, r"cantrip-selfplay(\.exe)? .*--resume --grant \S+gen2[/\\]dist[/\\]<attempt>.grant --host ")
            self.assertRegex(text, r"cantrip_ai.train ")
            # The workers' server: the native server's settings by default (the python one's are not its own).
            self.assertIn("cantrip-infer --precision fp16 --pipelines 1", text)
            self.assertRegex(text, r"--lease \S+<attempt>.heartbeat --lease-seconds 900")
            workers.write_text(json.dumps(dict(workers=[dict(name="w1", host="h", user="u", root="D:/c",
                                                             server_args="--precision fp16 --shapes 32,64")])))
            with self.assertRaises(SystemExit) as refused, contextlib.redirect_stdout(io.StringIO()):
                generation.main(["--root", str(root), "--dry-run", "--games", "100", "--decks-dir", "", "--workers", str(workers)])
            self.assertIn("--shapes", str(refused.exception))

    def test_a_distributed_generation_is_not_finished_without_its_workers_unless_asked(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            (root / "gen2" / "dist").mkdir(parents=True)
            (root / "gen2" / "dist" / "ledger.json").write_text("{}")
            with self.assertRaises(SystemExit) as refused, contextlib.redirect_stdout(io.StringIO()):
                generation.main(["--root", str(root), "--generation", "2", "--no-build", "--games", "10", "--decks-dir", "",
                                 "--gpu-server", "off", "--stop-after", "selfplay"])
            self.assertIn("--abandon-workers", str(refused.exception))

    def test_stop_writes_the_stop_file_of_the_current_generations_self_play(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,), finished=False)
            with contextlib.redirect_stdout(io.StringIO()):
                path = generation.main(["--root", str(root), "--stop"])
            self.assertEqual(path, root / "gen1" / "selfplay" / "selfplay.stop")
            self.assertEqual(path.read_text(), "")
            with contextlib.redirect_stdout(io.StringIO()):
                generation.main(["--root", str(root), "--stop-now", "--generation", "1"])
            self.assertEqual(path.read_text().strip(), "now")

    def test_run_waits_out_a_graceful_stop_and_raises_stopped(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "x.log"
            stop = [sys.executable, "-c", "import sys; print('closing'); sys.exit(3)"]
            for stream in (False, True):
                with self.assertRaises(generation.Stopped) as caught, contextlib.redirect_stderr(io.StringIO()):
                    generation.run(stop, log, False, log=lambda *_: None, stream=stream, stopped_code=3)
                self.assertEqual(caught.exception.code, 3)
                self.assertIn("closing", caught.exception.text)
            with self.assertRaises(SystemExit) as failed:
                generation.run(stop, log, False, log=lambda *_: None)
            self.assertNotIsInstance(failed.exception, generation.Stopped)
            self.assertIn("failed (3)", str(failed.exception))

    def test_a_third_interrupt_kills_a_stopping_process(self):
        with tempfile.TemporaryDirectory() as tmp:
            log = Path(tmp) / "x.log"
            slow = [sys.executable, "-c", "import time; time.sleep(60)"]
            said = []
            calls = 0
            real = subprocess.Popen

            class Interrupted(real):
                def wait(self, timeout=None):
                    nonlocal calls
                    calls += 1
                    if calls <= 3:
                        raise KeyboardInterrupt
                    return super().wait(timeout)

            with unittest.mock.patch.object(generation.subprocess, "Popen", Interrupted):
                with self.assertRaises(KeyboardInterrupt):
                    generation.run(slow, log, False, log=said.append, stopped_code=3)
            self.assertEqual(len(said), 4, said)  # the command, then one line an interrupt
            self.assertIn("killing it", said[-1])

    def test_the_inference_server_has_a_process_group_of_its_own(self):
        with tempfile.TemporaryDirectory() as tmp:
            seen = {}

            class Recorded:
                def __init__(self, cmd, **kw):
                    seen.update(kw)
                    raise OSError("not started in a test")

            server = generation.InferenceServer([Path(tmp)], Path(tmp) / "server.log", "cpu", [], False, log=lambda *_: None)
            with unittest.mock.patch.object(generation.subprocess, "Popen", Recorded), self.assertRaises(OSError):
                server.__enter__()
            server.file.close()
            if sys.platform == "win32":
                self.assertEqual(seen.get("creationflags"), subprocess.CREATE_NEW_PROCESS_GROUP)
            else:
                self.assertTrue(seen.get("start_new_session"))

    def test_self_play_stopped_by_its_governor_is_left_to_resume(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            champion = root / "champions" / "gen0"
            (champion / "train").mkdir(parents=True)
            (champion / "manifest.json").write_text("{}")
            (root / "champions" / "current").symlink_to("gen0")

            def stopping(cmd, log_path, dry_run, **kw):
                if "cantrip-selfplay" not in str(cmd[0]):
                    return ""
                self.assertEqual(kw.get("stopped_code"), generation.SELFPLAY_STOPPED)
                out = Path(cmd[cmd.index("--out") + 1])
                out.mkdir(parents=True, exist_ok=True)
                (out / "selfplay-report.json").write_text(json.dumps(dict(
                    Games=12, Stopped=dict(Reason="memory under the floor of 2.0 GB", GamesStopped=3, ExitCode=3))))
                raise generation.Stopped(3, "")

            with unittest.mock.patch.object(generation, "run", stopping), contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaises(SystemExit) as caught:
                    generation.main(["--root", str(root), "--generation", "2", "--no-build", "--decks-dir", "",
                                     "--gpu-server", "off", "--stop-after", "selfplay"])
            why = str(caught.exception.code)
            self.assertIn("memory under the floor of 2.0 GB", why)
            self.assertIn("12 finished games are in closed shards", why)
            self.assertIn("run the driver again with the same arguments to resume", why)
            record = root / "gen2" / "generation.json"
            stages = json.loads(record.read_text()).get("stages", {}) if record.exists() else {}
            self.assertFalse(stages.get("selfplay", {}).get("done"), "self-play is not marked done")

    def test_a_gate_log_is_reused_only_for_the_same_rule_and_pairing(self):
        log = ("neural@64 against neural@64, 6000 games, 3 decks in the pairings of gate-pairings.txt, each pair one deal, "
               "shuffles from 5.\n" + PAIRED_GATE.replace("neural:decklist@64", "neural@64") + "\nneural@64/gen1: answers\n")
        metagame = "in the pairings of gate-pairings.txt, each pair one deal"
        paired = generation.gate_stop_lines("sprt-paired", False, 2.58, 100, 30)
        self.assertTrue(generation.gate_reusable(log, "gen1", "neural@64", paired, metagame))
        self.assertFalse(generation.gate_reusable(log, "gen2", "neural@64", paired, metagame), "another champion")
        self.assertFalse(generation.gate_reusable(log, "gen1", "neural@32", paired, metagame), "another seat")
        self.assertFalse(generation.gate_reusable(log, "gen1", "neural@64", paired, " decks, each a mirror"), "another pairing")
        for rule in ("sprt", "above-half", "always"):
            lines = generation.gate_stop_lines(rule, False, 2.58, 100, 30)
            self.assertFalse(generation.gate_reusable(log, "gen1", "neural@64", lines, metagame), rule)
        # above-half with its early stop: the interval line, at this run's z and minimums.
        stopped = log.replace("Paired SPRT (pentanomial", "Interval stop (the paired 2.58-sigma interval excludes 50 %, from 100 "
                              "pairs, 30 of them not split): stopped at pair 9.\nIgnored (pentanomial")
        early = generation.gate_stop_lines("above-half", True, 2.58, 100, 30)
        self.assertTrue(generation.gate_reusable(stopped, "gen1", "neural@64", early, metagame))
        self.assertFalse(generation.gate_reusable(stopped, "gen1", "neural@64",
                                                  generation.gate_stop_lines("above-half", True, 1.96, 100, 30), metagame))
        self.assertFalse(generation.gate_reusable(stopped, "gen1", "neural@64",
                                                  generation.gate_stop_lines("above-half", False, 2.58, 100, 30), metagame))

    def test_a_ladder_run_again_replaces_its_generations_rows(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            duel = generation.parse_gate(PAIRED_GATE)
            generation.ladder_record(root, [dict(duel, generation=2, champion="gen2", anchor="gen0", budget="@64")], 2)
            generation.ladder_record(root, [dict(duel, generation=4, champion="gen3", anchor="gen0", budget="@64"),
                                            dict(duel, generation=4, champion="gen3", anchor="tactical", budget="@64")], 4)
            generation.ladder_record(root, [dict(duel, generation=4, champion="gen4", anchor="gen0", budget="@64")], 4)
            rows = generation.ladder_rows(root)
            self.assertEqual([(r["generation"], r["champion"], r["anchor"]) for r in rows], [(2, "gen2", "gen0"), (4, "gen4", "gen0")])
            self.assertEqual((root / "ladder.md").read_text().count("| 4 |"), 1)

    def test_the_deck_pool_leaves_out_what_botlab_leaves_out(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            (d / "aggro.txt").write_text("# Aggro (12 players, 5-0)\n\n20 Mountain\n4 Shock\nLightning Bolt\nDeck\n5 Goblin Guide\n"
                                         "Sideboard\n15 Island\n")
            (d / "short.txt").write_text("# Short (40 players)\n20 Island\n4 Opt\nSideboard\n15 Island\n")
            (d / "run.txt").write_text("seed 3, 12 games\n")
            (d / "commander.txt").write_text("Commander\n1 Atraxa\nCompanion\n1 Lurrus\nDeck\n29 Plains\n")
            self.assertEqual(generation.deck_cards((d / "aggro.txt").read_text()), 30)
            self.assertEqual(generation.deck_cards((d / "commander.txt").read_text()), 29)
            self.assertEqual(generation.deck_pool(d), [("aggro", 12)])
            (d / "landfall.txt").write_text("# Mono-Green Landfall: a Standard meta deck, 11.6% of the metagame (MTGGoldfish).\n"
                                            "60 Forest\nSideboard\n15 Forest\n")
            self.assertEqual(generation.deck_pool(d), [("aggro", 12), ("landfall", 11.6)])

    def test_a_failed_measurement_is_recorded_and_the_generation_goes_on(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            champion = root / "champions" / "gen0"
            (champion / "train").mkdir(parents=True)
            (champion / "manifest.json").write_text("{}")
            (root / "champions" / "current").symlink_to("gen0")
            stages = {n: dict(done=True, seconds=1.0) for n in ("build", "selfplay", "train", "export", "parity")}
            stages["selfplay"].update(games=4, simulations=8, lanes=1, games_per_hour=4.0, samples=40, visit_samples=8,
                                      capped=0, dropped=0, fallbacks=0, first_player=0.5, seeds="1..5")
            stages["train"].update(steps=10, seen=2560, init="gen0", window=[], value_mix=0.0, held_out=dict(value_mse=0.5))
            stages["export"].update(model="models/gen2")
            stages["parity"].update(summary="ok")
            stages["gate"] = dict(done=True, seconds=1.0, wins=6, losses=4, draws=0, games=10, share=0.6, margin=0.3,
                                  champion="gen0", simulations=8, decks="rw-aggro", pentanomial=[0, 0, 3, 0, 2])
            stages["promote"] = dict(done=True, seconds=1.0, accepted=False, champion="gen0", rule="above-half")
            rec = generation.Record(root / "gen2", 2, False)
            rec.data.update(stages=stages, champion="gen0")
            rec.save()
            ran = []

            def failing(cmd, *a, **k):
                ran.append(" ".join(map(str, cmd)))
                raise SystemExit("failed (1): botlab duel")

            out = io.StringIO()
            with unittest.mock.patch.object(generation, "run", failing), contextlib.redirect_stdout(out):
                generation.main(["--root", str(root), "--generation", "2", "--no-build", "--decks-dir", "",
                                 "--net-duel-games", "10", "--ladder-every", "1", "--ladder-anchors", "tactical",
                                 "--gpu-server", "off", "--device", "cpu"])
            self.assertEqual(len(ran), 2, "the network duel and the ladder were tried")
            record = json.loads((root / "gen2" / "generation.json").read_text())
            for name in ("netduel", "ladder"):
                self.assertFalse(record["stages"][name]["done"])
                self.assertIn("botlab duel", record["stages"][name]["failed"])
            self.assertTrue(record["stages"]["retire"]["done"], "the stages after it ran")
            self.assertIn("finished", record)
            self.assertIn("failed, recorded and passed over", out.getvalue())
            self.assertRegex((root / "gen2" / "README.md").read_text(), r"\| netduel \|.*failed, and passed over")

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
            stages["heldout"].update(directory="heldout", games=1000, shards=10, source="gen1")
            stages["train"].update(steps=10, seen=2560, init="gen0", window=[], value_mix=0.0, held_out=dict(value_mse=0.5),
                                   fixed_eval=dict(policy_ce=0.084, top1=0.97, value_mse=0.6))
            stages["export"].update(model="models/gen1")
            stages["parity"].update(summary="ok")
            duel = generation.parse_gate(PAIRED_GATE)
            stages["netduel"].update(duel, champion="gen0", decks="3 decks")
            stages["evaluate"].update(directory="heldout", games=1000, positions=470000,
                                      networks=dict(gen1=dict(value_mse=0.6, ece=0.02, policy_ce=0.5, top1=0.89, choices=18000)))
            stages["gate"].update(wins=6, losses=4, draws=0, games=10, share=0.6, margin=0.3, llr=0.1, sprt="no verdict",
                                  champion="gen0", simulations=8, decks="rw-aggro")
            stages["promote"].update(accepted=False, champion="gen0", rule="sprt")
            stages["ladder"].update(rows=[dict(duel, champion="gen0", anchor="tactical", budget="@64")])
            stages["retire"].update(deleted=0, keep=8)
            rec = generation.Record(gen1, 1, False)
            rec.data.update(stages=stages, champion="gen0", finished="then")
            rec.save()
            readme = (gen1 / "README.md").read_text()
            self.assertIn("expected to rise as the policy improves", readme, "the fixed eval says what it measures")
            self.assertRegex(readme, r"\| netduel \|.*by pair 50\.6% \+/- 0\.3% over 3000 pairs \(0 / 0\.5 / 1 / 1\.5 / 2: 12 / 30 / 2900 / 40 / 18")
            self.assertRegex(readme, r"\| evaluate \|.*gen1: value MSE 0\.6000, ECE 0\.0200, policy CE 0\.5000, top-1 0\.890 on 18000 targets")
            self.assertRegex(readme, r"\| ladder \|.*gen0 against tactical at @64")

            class Args:
                retrain, dry_run = "value overfit", False

            with contextlib.redirect_stdout(io.StringIO()):
                generation.retrain(rec, gen1, root / "models" / "gen1", Args)
            self.assertEqual(sorted(rec.data["stages"]), ["build", "heldout", "selfplay"], "self-play and the held-out set are kept")
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
