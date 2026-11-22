"""A whole generation, tiny, end to end (docs/ai-training.md, "Testing the driver"): the test to run before landing
any change to the driver, the miner, the distributed code or the supervisor.

    CANTRIP_AI_E2E=1 python -m unittest tests.test_e2e_generation      # from ai/; minutes, on the CPU

It records a few tactical-bot games, trains and exports a toy generation 0, and then runs two real generations with
the real binaries, nothing replaced: every stage, at `--tiny` scale, with the distributed path - self-play, the gate,
the miner's replays and judgements, the ladder - played through the worker manager's `local` machine (`--workers
local`: ledgers, grants, hosts' results files with their `#` header lines, merged and summarised), on a root that
starts from a scrubbed copy of a real line's shape (`fixtures/line`: a champions history with labelled generations
such as "5-mined", a distributed gate's results as its hosts wrote them).

- Generation 1: build, preflight, self-play (with its traces, brought home from the worker), the held-out set,
  training, export, parity, the gate, the promotion, the pair miner's two stages (targets written), the deep
  reanalysis of its self-play (`reanalyse.py`: the corpus written), evaluate, retire; under the supervisor
  (`loop.py`), so its preflight, its watching and its status file are run too.
- Generation 2: trains with generation 1's mined targets and its part of the deep corpus, promotes (`--accept
  always`, the gate played as a measurement), reanalyses its own self-play, scores the corpus's held-out slice and
  plays the ladder, whose turn is counted from the history's labelled rows.

It needs cargo, dotnet and PyTorch, uses no GPU, and is skipped unless `CANTRIP_AI_E2E` is set: the build's first
run takes many minutes (later ones a few), `CARGO_BUILD_JOBS` bounds its threads. The fixture's fast checks below it
run always.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

AI = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(AI))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import quiet  # noqa: E402,F401
from cantrip_ai import duels, generation, loop, mine, mining, notify, reanalyse, reports, status  # noqa: E402

REPO = AI.parent
FIXTURE = Path(__file__).resolve().parent / "fixtures" / "line"
EXE = ".exe" if os.name == "nt" else ""


class FixtureTest(unittest.TestCase):
    """What the real line's files look like, read by the code that reads them in a generation: fast, always run."""

    def test_a_history_with_labelled_generations_counts_each_generation_once(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "champions").mkdir()
            shutil.copy(FIXTURE / "champions-history.json", root / "champions" / "history.json")
            labels = [r["generation"] for r in json.loads((root / "champions" / "history.json").read_text())]
            self.assertIn("5-mined", labels)
            self.assertIn("6-v4", labels)
            # 0, 1, 3, 4, "5-mined", "6-v4", 7 were accepted: a labelled variant is its generation's promotion.
            self.assertEqual([generation.promotions(root, n) for n in (0, 2, 4, 5, 6, 7, 99)], [1, 2, 4, 5, 6, 7, 7])
            self.assertEqual(generation.ladder_due(generation.promotions(root, 7), 1, 0), (True, False))

    def test_a_distributed_gates_results_are_read_with_their_header_lines(self):
        hosts = [FIXTURE / "gate-host-a.results.tsv", FIXTURE / "gate-host-b.results.tsv"]
        for path in hosts:
            self.assertTrue(path.read_text().startswith("# {"), "as a host writes it: its header line first")
            header, done, counted = duels.read_results(path)
            self.assertEqual((header["format"], len(done), counted["failed"]), ("duel-results/1", 8, 0))
        # The miner's verify and the report by matchup read the hosts' own files as they read the merged one.
        merged = mine.read_results(FIXTURE / "gate-results.tsv")
        by_host = [r for path in hosts for r in mine.read_results(path)]
        self.assertEqual(len(by_host), 16)
        checked = mine.verify(merged, by_host)
        self.assertEqual((checked["games"], checked["same"], checked["differ"]), (16, 16, []))
        self.assertEqual(reports.matchups(hosts), reports.matchups([FIXTURE / "gate-results.tsv"]))
        with tempfile.TemporaryDirectory() as tmp:
            p = subprocess.run([sys.executable, "-m", "cantrip_ai.mine", "verify", "--gate", str(FIXTURE / "gate-results.tsv"),
                                "--replayed", *map(str, hosts), "--out", str(Path(tmp) / "verify.json")], cwd=AI,
                               capture_output=True, text=True, env={**os.environ, "PYTHONPATH": str(AI)})
            self.assertEqual(p.returncode, 0, p.stdout + p.stderr)
            self.assertEqual(json.loads((Path(tmp) / "verify.json").read_text())["same"], 16)


@unittest.skipUnless(os.environ.get("CANTRIP_AI_E2E"), "set CANTRIP_AI_E2E=1 to run a whole tiny generation (minutes; builds the engine)")
class EndToEndTest(unittest.TestCase):
    def sh(self, cmd, env=None, cwd=REPO, what=""):
        p = subprocess.run([str(c) for c in cmd], cwd=cwd, capture_output=True, text=True, encoding="utf-8", errors="replace",
                           env={**os.environ, **(env or {})}, stdin=subprocess.DEVNULL)
        self.assertEqual(p.returncode, 0, f"{what or cmd[0]} failed ({p.returncode}):\n{(p.stdout + p.stderr)[-4000:]}")
        return p.stdout + p.stderr

    def test_two_tiny_generations_every_stage_through_the_local_worker(self):
        keep = os.environ.get("CANTRIP_AI_E2E_KEEP")  # a directory to leave the root in, for a look afterwards
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
            root = Path(keep) if keep else Path(tmp) / "line"
            if root.exists():
                shutil.rmtree(root)
            root.mkdir(parents=True)
            home = Path(tmp) / "workers"
            home.mkdir()
            (home / "pool.json").write_text(json.dumps(dict(key="k", machines=[
                dict(name="here", local=True, os="windows" if os.name == "nt" else "linux", root=str(Path(tmp) / "compute"),
                     cores=4, ram_gb=16)])))
            env = {"CANTRIP_WORKERS_HOME": str(home), "CANTRIP_WORKERS_NO_DAEMON": "1", "DOTNET_ROLL_FORWARD": "Major",
                   notify.ENV_FILE: os.devnull, "PYTHONPATH": str(AI), "PYTHONUNBUFFERED": "1",
                   "CARGO_BUILD_JOBS": os.environ.get("CARGO_BUILD_JOBS", "4")}
            env_gone = [notify.ENV_HOOK, notify.ENV_WEBHOOK, "CANTRIP_WORKERS_CLI"]
            saved = {k: os.environ.pop(k) for k in env_gone if k in os.environ}
            self.addCleanup(os.environ.update, saved)
            decks = root / "decks"
            decks.mkdir()
            for name in ("rw-aggro", "ub-reanimator"):
                shutil.copy(REPO / "decks" / f"{name}.txt", decks / f"{name}.txt")
            # The real line's shape: a history whose generations carry labels.
            (root / "champions").mkdir()
            shutil.copy(FIXTURE / "champions-history.json", root / "champions" / "history.json")

            # Generation 0: the recording host and BotLab built, a few tactical games, a toy network.
            record_host = REPO / f"target/record/cards/cantrip-native-host{EXE}"
            self.sh(["cargo", "build", "--profile", "cards", "-p", "cantrip-host", "--bin", "cantrip-native-host", "--features",
                     "ai-record"], {**env, "CARGO_TARGET_DIR": str(REPO / "target" / "record")}, what="the recording host's build")
            self.sh([*generation.dotnet(), "build", "dotnet/tools/Cantrip.BotLab/Cantrip.BotLab.csproj", "-c", "Release", "-v", "q",
                     "--nologo"], env, what="BotLab's build")
            self.sh([*generation.dotnet(), generation.BOTLAB, "duel", "--a", "tactical", "--b", "tactical", "--deck", "rw-aggro",
                     "--deck", "ub-reanimator", "--mirrors", "--each-shuffle", "--games", "8", "--parallel", "4", "--ai-record",
                     root / "gen0" / "matchup", "--ai-record-mode", "decklist"], {**env, "CANTRIP_NATIVE_HOST": str(record_host)}, what="recording gen0's games")
            model = root / "models" / "gen0"
            self.sh([sys.executable, "-m", "cantrip_ai.train", "--data", root / "gen0" / "matchup", "--out", model / "train",
                     "--device", "cpu", "--steps", "6", "--width", "32", "--layers", "1", "--heads", "2", "--batch", "64",
                     "--workers", "0", "--held-out", "0.25", "--progress", "off"], env, cwd=AI, what="training gen0")
            self.sh([sys.executable, "-m", "cantrip_ai.export", "--checkpoint", model / "train" / "checkpoint.pt", "--out", model],
                    env, cwd=AI, what="exporting gen0")
            # The fixture's history says gen7 was the last champion; this root's champion is the toy gen0.
            (root / "champions" / "gen0").mkdir()
            shutil.copytree(model, root / "champions" / "gen0", dirs_exist_ok=True)
            generation.set_current(root, "gen0")

            # Decklist mode, as the real line's: the miner records its replays in it, and a window holds one mode.
            common = ["--tiny", "--mode", "decklist", "--device", "cpu", "--gpu-server", "off", "--decks-dir", decks, "--workers", "local",
                      "--games", "6", "--gate-games", "12", "--lanes", "2", "--no-selfplay-autoscale", "--selfplay-shard-games", "2",
                      "--gate-parallel", "2", "--worker-duel-parallel", "2", "--mine-lanes", "2", "--mine-local-lanes", "2",
                      "--mine-control", "2", "--ladder-every", "1", "--ladder-anchors", "tactical", "--ladder-games", "4",
                      "--window", "2", "--dist-poll", "2", "--train-args", "--workers 0 --batch 64 --held-out 0.4",
                      "--progress", "off",
                      # The deep reanalysis, at --tiny's toy scale: self-play's games (their traces kept), half of
                      # them held out, and the corpus in every later generation's training.
                      "--selfplay-traces", "--reanalyse-positions", "6", "--reanalyse-heldout", "0.5", "--corpus-train"]

            # Generation 1 under the supervisor: its arguments from the file, its preflight, the generation watched.
            (root / loop.ARGS).write_text("# the tiny line\n" + " ".join(f'"{x}"' for x in map(str, common))
                                          .replace("\\", "/") + "\n--heldout-games 2\n", encoding="utf-8")
            self.sh([sys.executable, "-m", "cantrip_ai.loop", "--root", root, "--max-generations", "1", "--poll", "2"], env,
                    cwd=AI, what=f"the supervisor's generation 1 (see {root / 'gen1-driver.log'}, {root / 'preflight' / 'preflight.log'})")
            one = json.loads((root / "gen1" / "generation.json").read_text())
            self.assertIn("finished", one, (root / "gen1-driver.log").read_text()[-3000:])
            ran = [n for n, s in one["stages"].items() if s.get("done")]
            self.assertEqual(ran, ["build", "selfplay", "heldout", "train", "export", "parity", "gate", "promote", "mine",
                                   "reanalyse", "evaluate", "mined", "retire"] if not one["stages"]["promote"]["accepted"] else
                             ["build", "selfplay", "heldout", "train", "export", "parity", "gate", "promote", "mine",
                              "reanalyse", "evaluate", "ladder", "mined", "retire"])
            self.assertFalse([n for n, s in one["stages"].items() if s.get("failed")], "no stage failed")
            # One engine, pinned, ran every stage.
            self.assertEqual({s.get("engine") for s in one["stages"].values()}, {one["engine"]["id"]})
            self.assertTrue((root / "builds" / one["engine"]["id"] / "release" / f"cantrip-selfplay{EXE}").exists())
            # The distributed path: ledgers, the hosts' own results files with their header lines, merged.
            for name in ("gate", "replay-first", "replay-second"):
                self.assertTrue((root / "gen1" / "dist" / name / "ledger.json").exists(), name)
            host_files = sorted((root / "gen1" / "dist" / "gate" / "results").glob("*.results.tsv"))
            self.assertTrue(host_files and all(f.read_text(encoding="utf-8").startswith("# {") for f in host_files))
            self.assertEqual(one["stages"]["gate"]["games"], 12)
            self.assertEqual(one["stages"]["gate"]["matchups"], reports.matchups(host_files))
            # The miner: every replay the gate's game, forks judged, targets written for the next generation.
            m = one["stages"]["mine"]
            self.assertEqual(m["same"], m["replayed"])
            self.assertGreater(m["replayed"], 0)
            paths = mining.Paths(root / "gen1")
            self.assertTrue(paths.report.exists())
            self.assertEqual(one["stages"]["evaluate"]["games"], 2)
            # The deep reanalysis: self-play's traces came home, its positions were searched by the champion the gate
            # left through the distributed judgements' ledger, and the corpus holds them, named by engine and judge.
            self.assertTrue(list((root / "gen1" / "selfplay" / "traces").glob("*.jsonl")), "the traces are with the shards")
            deep = one["stages"]["reanalyse"]
            self.assertEqual((deep["source"], deep["mismatched"]), ("selfplay", 0))
            self.assertGreater(deep["searched"], 0)
            self.assertEqual(deep["positions"], deep["searched"], "every position searched is a target")
            self.assertTrue((root / "gen1" / "dist" / "judge-deep-s1" / "ledger.json").exists())
            made = json.loads((root / reanalyse.CORPUS / "gen1" / reanalyse.RECORD).read_text())
            self.assertTrue(made["complete"])
            self.assertEqual(made["engine"]["id"], one["engine"]["id"])
            self.assertEqual(made["judge"], "gen1" if one["stages"]["promote"]["accepted"] else "gen0")
            self.assertEqual((made["simulations"], made["worlds"]), (32, 2))
            corpus_train = reanalyse.split_dirs(root / reanalyse.CORPUS / "gen1", "train")
            corpus_held = reanalyse.split_dirs(root / reanalyse.CORPUS / "gen1", "heldout")
            self.assertTrue(corpus_train and corpus_held, "three games, half held out: both sides have some")
            # Its held-out games are named, for every later training to leave out of the self-play that holds them.
            named = json.loads((root / reanalyse.CORPUS / "gen1" / reanalyse.KEYS).read_text())
            self.assertEqual(len(named), made["heldout_games"])
            self.assertGreater(len(named), 0)
            for d in corpus_train + corpus_held:
                self.assertEqual(json.loads((d / mining.EFFORT_FILE).read_text()), dict(simulations=32, worlds=2))
            # The status file's last word, the loop's, and what was told.
            said = status.read(root, 1)
            self.assertEqual((said["state"], status.judge(said)["state"]), ("finished", "finished"))
            self.assertGreater(said["progress"], 3)
            self.assertEqual(status.read_json(root / status.LOOP)["state"], "stopped")
            events = [json.loads(line)["event"] for line in (root / notify.LOG).read_text(encoding="utf-8").splitlines()]
            self.assertIn("verdict", events)
            self.assertEqual(events[-1], "finished")
            self.assertNotIn("stage-failed", events)

            # Generation 2: trains with generation 1's mined targets (when it kept any), promotes, plays the ladder.
            mined_dirs = [d for d in paths.targets.iterdir() if any(d.glob("*.shard.zst"))] if paths.targets.is_dir() else []
            out = self.sh([sys.executable, "-m", "cantrip_ai.generation", "--root", root, *common, "--accept", "always",
                           "--gate-measure", "--no-preflight"], env, cwd=AI, what="generation 2")
            two = json.loads((root / "gen2" / "generation.json").read_text())
            self.assertIn("finished", two, out[-3000:])
            self.assertEqual([n for n, s in two["stages"].items() if s.get("done")],
                             ["build", "selfplay", "train", "export", "parity", "gate", "promote", "reanalyse", "evaluate",
                              "ladder", "retire"])
            self.assertTrue(two["stages"]["promote"]["accepted"])
            self.assertEqual(Path(os.path.realpath(root / "champions" / "current")).name, "gen2")
            self.assertEqual(two["stages"]["ladder"]["rows"][0]["anchor"], "tactical")
            for d in mined_dirs:
                self.assertIn(f"mined: {d}", out)
            same = lambda path: os.path.normcase(os.path.realpath(path))  # noqa: E731
            window = [same(w["directory"]) for w in two["stages"]["train"]["window"]]
            self.assertIn(same(root / "gen1" / "selfplay"), window)
            for d in mined_dirs:
                self.assertIn(same(d), window, "generation 1's mined targets are trained on")
            # The deep corpus: generation 1's part is trained on and its held-out slice scored; generation 2's own
            # part (no gate was mined: its self-play again) is made beside it, and retention left generation 1's.
            for d in corpus_train:
                self.assertIn(same(d), window, "generation 1's deep targets are trained on")
            self.assertIn("left out of every directory: the deep corpus's held-out games", out)
            train_log = (root / "gen2" / "logs" / "train.log").read_text(encoding="utf-8", errors="replace")
            self.assertNotIn("Traceback", train_log)
            self.assertEqual(two["stages"]["train"]["corpus"], [f"{d}:4" for d in corpus_train], "by effort: 32 over --tiny's 8")
            scored = two["stages"]["evaluate"]["corpus"]
            self.assertEqual(sorted(scored["networks"]), sorted(["gen2", "gen1" if one["stages"]["promote"]["accepted"] else "gen0"]))
            self.assertGreater(scored["choices"], 0)
            self.assertEqual(two["stages"]["reanalyse"]["judge"], "gen2")
            self.assertEqual([g for g, _, _ in reanalyse.corpus_records(root)], [1, 2])
            # The history kept its labelled rows and gained this root's.
            history = json.loads((root / "champions" / "history.json").read_text())
            self.assertEqual([r["generation"] for r in history][-2:], [1, 2])
            self.assertIn("5-mined", [r["generation"] for r in history])


if __name__ == "__main__":
    unittest.main()
