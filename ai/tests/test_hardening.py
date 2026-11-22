"""The training loop's guards (docs/ai-training.md, "Watching a run" on): the status file and its reading, the
notifications, data-producing stages that fail loudly, the parity fallback, the loader's memory, the pinned engine, a
ledger that holds nothing, and the supervisor. Nothing here plays a game, serves a network or waits on time."""

from __future__ import annotations

import contextlib
import http.server
import io
import json
import os
import subprocess
import sys
import tempfile
import threading
import unittest
import unittest.mock
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import quiet  # noqa: E402,F401
from cantrip_ai import distributed, engine, generation, loop, notify, status  # noqa: E402
from test_generation import root_with  # noqa: E402


def promoted_root(tmp: Path, gen: int = 2, **extra_stages) -> Path:
    """A root whose generation `gen` has run to its promotion (accepted), a gate recorded with it."""
    root = root_with(tmp, gens=tuple(range(1, gen)))
    champion = root / "champions" / "gen0"
    (champion / "train").mkdir(parents=True)
    (champion / "manifest.json").write_text("{}")
    (root / "champions" / "current").symlink_to("gen0")
    stages = {n: dict(done=True, seconds=1.0) for n in ("build", "selfplay", "train", "export", "parity")}
    stages["selfplay"].update(games=4, simulations=8, lanes=1, games_per_hour=4.0, samples=40, visit_samples=8,
                              capped=0, dropped=0, fallbacks=0, first_player=0.5, seeds="1..5")
    stages["train"].update(steps=10, seen=2560, init="gen0", window=[], value_mix=0.0, held_out=dict(value_mse=0.5))
    stages["export"].update(model=f"models/gen{gen}")
    stages["parity"].update(summary="ok")
    stages["gate"] = dict(done=True, seconds=1.0, wins=6, losses=4, draws=0, games=10, share=0.6, margin=0.3,
                          champion="gen0", simulations=8, decks="rw-aggro", pentanomial=[0, 0, 3, 0, 2])
    stages["promote"] = dict(done=True, seconds=1.0, accepted=True, champion="gen0", rule="above-half")
    stages.update(extra_stages)
    generation.history(root, dict(generation=gen, accepted=True))
    rec = generation.Record(root / f"gen{gen}", gen, False)
    rec.data.update(stages=stages, champion="gen0")
    rec.save()
    return root


SELFPLAY = dict(done=True, seconds=1.0, games=4, simulations=8, lanes=1, games_per_hour=4.0, samples=40, visit_samples=8,
                capped=0, dropped=0, fallbacks=0, first_player=0.5, seeds="1..5")


def started(root: Path, gen: int) -> generation.Record:
    """Generation `gen` of `root` with its build and its self-play done."""
    rec = generation.Record(root / f"gen{gen}", gen, False)
    rec.data["stages"] = dict(build=dict(done=True, seconds=1.0), selfplay=dict(SELFPLAY))
    rec.data["champion"] = "gen0"
    rec.save()
    return rec


def notifications(root: Path) -> list[dict]:
    path = root / notify.LOG
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()] if path.exists() else []


def quietly(argv: list[str], **patches):
    """The driver run with `run` (and whatever else) replaced, its output kept."""
    out = io.StringIO()
    with contextlib.ExitStack() as stack:
        for name, value in patches.items():
            stack.enter_context(unittest.mock.patch.object(generation, name, value))
        stack.enter_context(contextlib.redirect_stdout(out))
        try:
            generation.main(argv)
        finally:
            quietly.out = out.getvalue()
    return out.getvalue()


BASE = ["--no-build", "--decks-dir", "", "--gpu-server", "off", "--device", "cpu"]


class FailLoudlyTest(unittest.TestCase):
    def test_a_failed_data_stage_stops_the_generation_and_says_what_is_lost(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            with self.assertRaises(SystemExit) as caught:
                quietly(["--root", str(root), "--generation", "2", *BASE], run=lambda *a, **k: "")
            why = str(caught.exception.code)
            self.assertIn("stage mine failed, and its output is training data", why)
            self.assertIn("WITHOUT gen2's mined targets", why)
            self.assertIn("no gate results to mine", why)
            record = json.loads((root / "gen2" / "generation.json").read_text())
            self.assertNotIn("finished", record)
            self.assertNotIn("retire", record["stages"], "nothing after the failed stage ran")
            # The status file has the driver's last word, and the failure was told.
            said = status.read(root, 2)
            self.assertEqual((said["state"], said["stage"]), ("failed", "mine"))
            self.assertEqual(status.judge(said)["state"], "failed")
            told = notifications(root)
            self.assertEqual([n["event"] for n in told], ["stage-failed"])
            self.assertIn("gen2 FAILED at mine", told[0]["message"])

    def test_an_allowed_data_failure_is_flagged_told_and_not_trained_past_unnoticed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            quietly(["--root", str(root), "--generation", "2", *BASE, "--allow-data-failure"], run=lambda *a, **k: "")
            record = json.loads((root / "gen2" / "generation.json").read_text())
            self.assertIn("finished", record)
            self.assertIn("no gate results", record["stages"]["mine"]["failed"])
            readme = (root / "gen2" / "README.md").read_text()
            self.assertIn("> **WARNING", readme)
            self.assertRegex(readme, r"stage mine failed and was passed over, so the next generations train WITHOUT")
            events = [n["event"] for n in notifications(root)]
            self.assertIn("data-failed", events)
            self.assertIn("warning", events)
            self.assertEqual(events[-1], "finished")
            self.assertIn("WARNING", notifications(root)[-1]["message"])
            # The next generation's training refuses to go without them, unless that too is allowed.
            started(root, 3)
            with self.assertRaises(SystemExit) as caught:
                quietly(["--root", str(root), "--generation", "3", *BASE], run=lambda *a, **k: "")
            self.assertIn("training would go without gen2's mined targets (its mine and mined failed)", str(caught.exception.code))

    def test_a_failed_measurement_is_in_the_warning_block_and_told(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))

            def failing(cmd, *a, **k):
                raise SystemExit("failed (1): botlab duel")

            quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine", "--ladder-every", "1",
                     "--ladder-anchors", "tactical"], run=failing)
            readme = (root / "gen2" / "README.md").read_text()
            self.assertRegex(readme, r"> - \*\*stage ladder failed and was passed over: failed \(1\): botlab duel")
            events = [n["event"] for n in notifications(root)]
            self.assertEqual(events, ["measurement-failed", "warning", "finished"])


class ServingTest(unittest.TestCase):
    def test_a_serving_option_replaces_only_the_precision_flags(self):
        configured = "--precision fp16 --batch 48 --fp16-graph residual --inputs-tf32 --pipelines 2"
        self.assertEqual(generation.with_precision(configured, "--precision fp32"), "--batch 48 --pipelines 2 --precision fp32")
        options = generation.serving_options(configured, generation.NATIVE_FALLBACKS)
        self.assertEqual(options[0], configured)
        self.assertEqual(options[1], "--batch 48 --pipelines 2 --precision fp16 --fp16-graph residual --inputs-tf32")
        self.assertEqual(options[-1], "--batch 48 --pipelines 2 --precision fp32")
        self.assertEqual(generation.serving_options("--precision fp32", ["--precision fp32"]), ["--precision fp32"])

    def run_parity(self, root: Path, failing: dict, extra=()):
        """Generation 2's parity stage with a server that refuses to start for the models of `failing` (an option's
        distinguishing word to the model names it fails): the servers started, as (stage, models, arguments)."""
        started = []

        class Server:
            def __init__(self, models, log_path, device, extra, dry_run, log=print, **kw):
                self.models, self.extra, self.name = models, extra, log_path.stem
                self.address = "127.0.0.1:1"

            def __enter__(self):
                started.append((self.name, [m.name for m in self.models], " ".join(self.extra)))
                for word, names in failing.items():
                    bad = [m.name for m in self.models if m.name in names]
                    if (word in self.extra or (word == "default" and "--fp16-graph" not in self.extra
                                                and "fp32" not in self.extra)) and bad:
                        raise SystemExit(f"the inference server did not start\ninfer: {bad[0]} references FAIL at fp16: value: 0.0126")
                return self

            def __exit__(self, *_):
                pass

        record = json.loads((root / "gen2" / "generation.json").read_text())
        for name in ("parity", "gate", "promote"):
            record["stages"].pop(name)
        (root / "gen2" / "generation.json").write_text(json.dumps(record))
        (root / "models" / "gen2").mkdir(parents=True, exist_ok=True)
        out = quietly(["--root", str(root), "--generation", "2", "--no-build", "--decks-dir", "", "--gpu-server", "on",
                       "--device", "cpu", "--server-impl", "native", "--server-args", "--precision fp16 --batch 48",
                       "--stop-after", "parity", *extra],
                      run=lambda *a, **k: "remote batch 64: Parity { max: 0.006 }", InferenceServer=Server)
        return started, json.loads((root / "gen2" / "generation.json").read_text()), out

    def test_parity_falls_back_to_the_first_option_every_model_passes(self):
        # gen14 of the templates line: the candidate passes `residual` with TF32 inputs and the champion fails it; the
        # option chosen is the first that passes both, held together by one server.
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            started, record, out = self.run_parity(root, {"default": ("gen2",), "residual": ("gen0",), "attention": ("gen0",)})
            self.assertEqual([s[1] for s in started], [["gen2", "gen0"]] * 4, "every check holds both models")
            pick = record["serving"]["candidate"]
            self.assertEqual(pick["args"], "--batch 48 --precision fp16 --fp16-graph products")
            self.assertEqual((pick["index"], pick["option"]), (3, "--precision fp16 --fp16-graph products"))
            self.assertEqual([t["args"] for t in pick["tried"]],
                             ["--precision fp16 --batch 48", "--batch 48 --precision fp16 --fp16-graph residual --inputs-tf32",
                              "--batch 48 --precision fp16 --fp16-graph attention --inputs-tf32"])
            self.assertIn("references FAIL", pick["tried"][0]["failed"])
            self.assertIn("served with `--batch 48 --precision fp16 --fp16-graph products` (fallback 3", record["stages"]["parity"]["summary"])
            self.assertIn("parity (candidate): serving gen2, gen0 with", (root / "gen2" / "README.md").read_text())
            self.assertEqual([n["event"] for n in notifications(root)], ["warning"])
            # Every later server of the generation serves with it: the gate's here, and a duel's hosts'.
            plan = quietly(["--root", str(root), "--generation", "2", "--no-build", "--decks-dir", "", "--gpu-server", "on",
                            "--device", "cpu", "--server-impl", "native", "--server-args", "--precision fp16 --batch 48",
                            "--dry-run", "--no-mine"])
            serving = [line for line in plan.splitlines() if "cantrip-infer" in line and " serve " in line]
            self.assertTrue(serving and all("--fp16-graph products" in line and "--batch 48" in line for line in serving), serving)

    def test_parity_with_the_configured_flags_changes_nothing_and_none_passing_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            started, record, _ = self.run_parity(root, {})
            self.assertEqual(len(started), 1)
            self.assertEqual((record["serving"]["candidate"]["index"], record["serving"]["candidate"]["option"]), (0, None))
            self.assertEqual(notifications(root), [])
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            with self.assertRaises(SystemExit) as caught:
                self.run_parity(root, {"default": ("gen2",)}, extra=["--no-parity-fallback"])
            self.assertIn("no serving option passes for gen2, gen0", str(caught.exception.code))
            self.assertIn("The tolerance is not loosened", str(caught.exception.code))


class LoaderMemoryTest(unittest.TestCase):
    def test_training_out_of_loader_memory_is_run_again_with_fewer_workers(self):
        import torch  # noqa: PLC0415

        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            champion = root / "champions" / "gen0"
            (champion / "train").mkdir(parents=True)
            (champion / "manifest.json").write_text("{}")
            (root / "champions" / "current").symlink_to("gen0")
            started(root, 2)
            calls = []

            def run(cmd, log_path, dry_run, **kw):
                cmd = [str(c) for c in cmd]
                calls.append((cmd, kw))
                out = Path(cmd[cmd.index("--out") + 1])
                out.mkdir(parents=True, exist_ok=True)
                if len(calls) < 3:
                    log_path.parent.mkdir(parents=True, exist_ok=True)
                    log_path.write_text("Traceback ...\nMemoryError\n")
                    if len(calls) == 2:
                        (out / "checkpoint.pt").write_bytes(b"partial")
                    raise SystemExit(f"failed (1): train\n--- {log_path} ---\nMemoryError")
                (out / "metrics.json").write_text(json.dumps(dict(history=[dict(step=4, seen=256)])))
                torch.save(dict(config=dict(width=8, layers=1), state={}, args={}, data=dict(directories=[], samples=0)),
                           out / "checkpoint.pt")
                return ""

            quietly(["--root", str(root), "--generation", "2", *BASE, "--stop-after", "train", "--train-args", "--workers 4"],
                    run=run)
            self.assertEqual(len(calls), 3)
            self.assertIs(calls[0][1]["fatal"], generation.LOADER_OOM, "a loader that hangs on it is killed")
            workers = [c[len(c) - 1 - c[::-1].index("--workers") + 1] for c, _ in calls]
            self.assertEqual(workers, ["4", "2", "1"])
            self.assertNotIn("--resume", calls[1][0], "nothing to resume yet")
            self.assertIn("--resume", calls[2][0], "from the checkpoint the failed run left")
            record = json.loads((root / "gen2" / "generation.json").read_text())
            self.assertEqual([r["workers_now"] for r in record["stages"]["train"]["loader_recovered"]], [2, 1])
            self.assertEqual([n["event"] for n in notifications(root)], ["recovered", "recovered"])

    def test_any_other_training_failure_is_not_retried_and_the_pattern_is_the_loaders(self):
        self.assertTrue(generation.LOADER_OOM.search("RuntimeError: DataLoader worker (pid 4120) exited unexpectedly"))
        self.assertTrue(generation.LOADER_OOM.search("OSError: [WinError 1455] The paging file is too small"))
        self.assertFalse(generation.LOADER_OOM.search("torch.OutOfMemoryError: CUDA out of memory. Tried to allocate 2 GiB"))
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))
            (root / "champions" / "gen0" / "train").mkdir(parents=True)
            (root / "champions" / "current").symlink_to("gen0")
            started(root, 2)
            calls = []

            def run(cmd, *a, **k):
                calls.append(cmd)
                raise SystemExit("failed (1): train\nValueError: bad shard")

            with self.assertRaises(SystemExit):
                quietly(["--root", str(root), "--generation", "2", *BASE, "--stop-after", "train"], run=run)
            self.assertEqual(len(calls), 1)

    def test_a_process_that_hangs_after_a_fatal_line_is_killed(self):
        with tempfile.TemporaryDirectory() as tmp:
            log_path = Path(tmp) / "train.log"
            code = "import sys, time; print('MemoryError', flush=True); time.sleep(120)"
            with self.assertRaises(SystemExit) as caught, contextlib.redirect_stdout(io.StringIO()):
                with unittest.mock.patch.object(generation, "fatal_watch",
                                                lambda p, path, pattern, grace, log=print: _real_watch(p, path, pattern, 0.2, log, 0.1)):
                    generation.run([sys.executable, "-c", code], log_path, False, fatal=generation.LOADER_OOM)
            self.assertIn("MemoryError", str(caught.exception.code))


_real_watch = generation.fatal_watch


class StatusTest(unittest.TestCase):
    def test_the_heartbeat_counts_progress_from_the_ledgers_the_shards_and_the_logs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for n, seconds in ((1, 100.0), (2, 300.0), (3, 200.0)):
                (root / f"gen{n}").mkdir()
                (root / f"gen{n}" / "generation.json").write_text(json.dumps(dict(stages=dict(gate=dict(done=True, seconds=seconds)))))
            self.assertEqual(status.expected(root, 4, "gate"), 200.0)
            self.assertIsNone(status.expected(root, 4, "train"))
            clock = [1000.0]
            beat = status.Heartbeat(root, 4, clock=lambda: clock[0])
            beat.gen_dir.mkdir()
            beat.beat()
            beat.begin("gate")
            first = status.read(root, 4)
            self.assertEqual((first["stage"], first["expected_seconds"], first["pid"]), ("gate", 200.0, os.getpid()))
            clock[0] += 30
            beat.beat()
            self.assertEqual(status.read(root, 4)["progress"], first["progress"], "nothing moved")
            ledger = root / "gen4" / "dist" / "gate" / "ledger.json"
            ledger.parent.mkdir(parents=True)
            ledger.write_text(json.dumps(dict(shards={"a": dict(indices=[[0, 4], [6, 8]])}, attempts={"x": {}})))
            beat.beat()
            now = status.read(root, 4)
            self.assertEqual(now["progress_detail"]["ledger_games"], 7)
            self.assertEqual((now["progress"], now["progress_at"]), (first["progress"] + 1, 1030.0))
            (root / "gen4" / "logs").mkdir()
            (root / "gen4" / "logs" / "server-gate.log").write_text("the server speaks whether a game moves or not")
            clock[0] += 30
            beat.beat()
            self.assertEqual(status.read(root, 4)["progress_at"], 1030.0, "a server's log is no progress")
            (root / "gen4" / "logs" / "gate.log").write_text("a game")
            beat.beat()
            self.assertEqual(status.read(root, 4)["progress_at"], 1060.0)

    def test_running_stalled_dead_failed_finished(self):
        import socket  # noqa: PLC0415

        s = dict(generation=4, state="running", stage="gate", pid=1, host=socket.gethostname(), stage_started=0.0,
                 expected_seconds=4000.0, progress_at=0.0, updated=0.0)
        alive, gone = (lambda pid: True), (lambda pid: False)
        self.assertEqual(status.judge(dict(s, updated=590, progress_at=500), 600, alive=alive)["state"], "running")
        # No progress for a quarter of the stage's usual time (and at least the floor): stalled.
        stalled = status.judge(dict(s, updated=1590, progress_at=500), 1600, alive=alive)
        self.assertEqual(stalled["state"], "stalled")
        self.assertIn("no progress for 18 min", stalled["why"])
        self.assertEqual(status.judge(dict(s, updated=1590, progress_at=500), 1600, stall_factor=0.5, alive=alive)["state"], "running")
        # A driver that stopped writing its own file hangs, whatever its stage does.
        self.assertEqual(status.judge(dict(s, updated=100, progress_at=100), 600, alive=alive)["state"], "stalled")
        # Its process gone with no last word: dead, however lately it wrote.
        dead = status.judge(dict(s, updated=599, progress_at=599), 600, alive=gone)
        self.assertEqual(dead["state"], "dead")
        self.assertIn("left no final word", dead["why"])
        self.assertEqual(status.judge(dict(s, state="failed", error="boom\nmore", transient=None), 600, alive=gone)["state"], "failed")
        self.assertEqual(status.judge(dict(s, state="finished"), 600, alive=gone)["state"], "finished")
        self.assertEqual(status.judge(dict(s, state="stopped"), 600, alive=gone)["state"], "stopped")
        self.assertEqual(status.judge(None)["state"], "none")
        late = status.judge(dict(s, stage_started=0, updated=8990, progress_at=8990), 9000, alive=alive)
        self.assertIn("2.2 times its expected", late["overdue"])
        # With nothing to go by, an hour.
        self.assertEqual(status.judge(dict(s, expected_seconds=None, updated=3000, progress_at=0), 3001, alive=alive)["state"], "running")
        self.assertEqual(status.judge(dict(s, expected_seconds=None, updated=3690, progress_at=0), 3700, alive=alive)["state"], "stalled")

    def test_a_transient_failure_is_one_the_driver_names(self):
        self.assertEqual(status.transient("self-play stopped by its governor (memory under the floor of 2.0 GB (1.1 GB available"), "selfplay-memory")
        self.assertIsNone(status.transient("self-play stopped by its governor (stop file D:/x/selfplay.stop): 12 finished"))
        self.assertTrue(status.asked_to_stop("self-play stopped by its governor (stop file D:/x/selfplay.stop): 12 finished"))
        self.assertTrue(status.asked_to_stop("gate stopped on every host: 12 of 40 games are in"))
        self.assertIsNone(status.transient("failed (101): cargo test model_parity"))
        self.assertIsNone(status.transient(
            "parity (candidate): no serving option passes for gen2, gen0; tried `--precision fp16` (the inference "
            "server did not start ... references FAIL)"), "a parity failure quotes the server; it is not transient")

    def test_unreadable_and_long_silent_is_dead(self):
        with unittest.mock.patch.object(distributed, "process_start", lambda pid: None), \
                unittest.mock.patch.object(distributed, "process_unreadable", lambda pid: True):
            self.assertTrue(status.alive(dict(pid=4, pid_start=1, updated=1000.0), now=1000.0 + status.SILENT - 1))
            self.assertFalse(status.alive(dict(pid=4, pid_start=1, updated=1000.0), now=1000.0 + status.SILENT + 1),
                             "nobody can ask it, and its file went quiet long ago: another process took the number")
            self.assertTrue(status.alive(dict(pid=4, pid_start=1)), "a file with no heartbeat says nothing of silence")
        # A parity failure at a later server's start (the gate's) quotes the server and is not a server that did not start.
        self.assertIsNone(status.transient("the inference server did not start\n--- server-gate.log ---\n"
                                           "infer: gen13 references FAIL at fp16: Check { value: 0.0196 }\n"
                                           "D:/models/gen13: the served model is not its references"))
        self.assertEqual(status.transient("the inference server did not start\nCUDA error: out of memory"), "server-start")

    def test_alive_is_the_same_process_not_the_same_pid(self):
        me = dict(pid=os.getpid(), pid_start=distributed.process_start(os.getpid()))
        self.assertTrue(status.alive(me))
        self.assertFalse(status.alive(dict(me, pid_start=me["pid_start"] + 1)), "the pid was reused by another process")
        self.assertTrue(status.alive(dict(pid=os.getpid())), "a file from before the start time was recorded")
        p = subprocess.Popen([sys.executable, "-c", "pass"])
        p.wait()
        self.assertFalse(status.alive(dict(pid=p.pid, pid_start=1)))
        self.assertFalse(status.alive(None))
        # A process that exists and may not be asked about (another user's) is alive: calling it dead would start a
        # second driver beside it.
        with unittest.mock.patch.object(distributed, "process_start", lambda pid: None), \
                unittest.mock.patch.object(distributed, "process_unreadable", lambda pid: True):
            self.assertTrue(status.alive(dict(pid=4, pid_start=123)))
        with unittest.mock.patch.object(distributed, "process_start", lambda pid: None), \
                unittest.mock.patch.object(distributed, "process_unreadable", lambda pid: False):
            self.assertFalse(status.alive(dict(pid=4, pid_start=123)))
        self.assertFalse(distributed.process_unreadable(os.getpid()), "this process can be asked")

    def test_a_log_still_open_counts_as_progress(self):
        # On Windows a directory entry keeps the size an open file had when it was last closed: the size is asked of
        # the file itself, or a stage whose only evidence is its log reads as stalled.
        with tempfile.TemporaryDirectory() as tmp:
            gen = Path(tmp) / "gen1"
            (gen / "logs").mkdir(parents=True)
            with (gen / "logs" / "mine-judge.log").open("w") as f:
                f.write("a line\n")
                f.flush()
                before = status.measure(gen)["log_bytes"]
                f.write("another line, the file still open\n" * 100)
                f.flush()
                self.assertGreater(status.measure(gen)["log_bytes"], before)

    def test_expected_goes_by_whole_runs_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            records = {1: dict(seconds=1000.0), 2: dict(seconds=50.0, runs=2), 3: dict(seconds=40.0, resumed=12),
                       4: dict(seconds=30.0, init="resumed"), 5: dict(seconds=1200.0, runs=1, resumed=0)}
            for n, stage in records.items():
                (root / f"gen{n}").mkdir()
                (root / f"gen{n}" / "generation.json").write_text(json.dumps(dict(stages=dict(selfplay=dict(done=True, **stage)))))
            self.assertEqual(status.expected(root, 6, "selfplay"), 1100.0, "the resumed runs' last parts are left out")

    def test_a_status_that_cannot_be_written_never_fails_the_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            beat = status.Heartbeat(root, 2)
            failing = unittest.mock.patch.object(status.os, "replace", side_effect=PermissionError("in use"))
            with failing:
                beat.start()
                beat.begin("selfplay")
                self.assertFalse(status.write_atomic(root / "x.json", dict(a=1), seconds=0.0))
            self.assertFalse(list(root.rglob("*.tmp")), "nothing is left behind")
            # The last word is tried for longer: a reader holding the file for a moment does not lose it.
            real, calls = os.replace, []

            def flaky(a, b):
                calls.append(a)
                if len(calls) < 4:
                    raise PermissionError("in use")
                return real(a, b)

            with unittest.mock.patch.object(status.os, "replace", flaky):
                beat.close("failed", "boom")
            self.assertEqual(status.read(root, 2)["state"], "failed")
            made = loop.Loop(root, SimpleNamespace(poll=0.01), [])
            made.log = lambda text: None
            with failing:
                made.state("running", "gen2")  # no exception
            self.assertFalse(status.write_atomic(root / "y.json", dict(a=object()), seconds=0.0) is None)

    def test_the_command_reports_a_dead_driver_with_its_logs_tail(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            p = subprocess.Popen([sys.executable, "-c", "pass"])
            p.wait()
            beat = status.Heartbeat(root, 3)
            beat.gen_dir.mkdir()
            beat.data["pid"] = p.pid
            beat.begin("train")
            (root / "gen3" / "logs").mkdir()
            (root / "gen3" / "logs" / "train.log").write_text("step 1\nstep 2\n")
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                code = status.main(["--root", str(root)])
            self.assertEqual(code, status.EXIT["dead"])
            self.assertIn("gen3: DEAD", out.getvalue())
            self.assertIn("step 2", out.getvalue())


class NotifyTest(unittest.TestCase):
    def test_a_hook_gets_the_message_and_the_payload_and_a_webhook_a_discord_body(self):
        got = []

        class Handler(http.server.BaseHTTPRequestHandler):
            def do_POST(self):
                got.append(json.loads(self.rfile.read(int(self.headers["Content-Length"]))))
                self.send_response(204)
                self.end_headers()

            def log_message(self, *a):
                pass

        server = http.server.HTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=server.serve_forever, daemon=True).start()
        try:
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                hooked = root / "hook.json"
                script = root / "hook.py"
                script.write_text("import json, os, sys\n"
                                  f"json.dump(dict(message=os.environ['CANTRIP_AI_MESSAGE'], event=os.environ['CANTRIP_AI_EVENT'], "
                                  f"payload=json.load(sys.stdin)), open({str(hooked)!r}, 'w'))\n")
                env = {notify.ENV_HOOK: f'"{sys.executable}" "{script}"',
                       notify.ENV_WEBHOOK: f"http://127.0.0.1:{server.server_address[1]}/hook"}
                with unittest.mock.patch.dict(os.environ, env):
                    sent = notify.send("stage-failed", "gen14 FAILED at parity: drift 0.0126", root=root, generation=14,
                                       stage="parity", error="value 0.0126 > 0.01")
                self.assertEqual(sent, dict(hook="ok", webhook="ok (204)"))
                hook = json.loads(hooked.read_text())
                self.assertEqual((hook["event"], hook["message"]), ("stage-failed", "gen14 FAILED at parity: drift 0.0126"))
                self.assertEqual(hook["payload"]["details"], dict(error="value 0.0126 > 0.01"))
                self.assertEqual((got[0]["generation"], got[0]["stage"]), (14, "parity"))
                self.assertRegex(got[0]["content"], r"^\*\*\[stage-failed\]\*\* \S+ gen14 parity: gen14 FAILED at parity")
                self.assertEqual(notifications(root)[0]["sent"], sent)
        finally:
            server.shutdown()
            server.server_close()

    def test_nothing_configured_only_logs_and_a_broken_channel_never_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual(notify.send("finished", "done", root=tmp, generation=1), {})
            self.assertEqual(len(notifications(Path(tmp))), 1)
            with unittest.mock.patch.dict(os.environ, {notify.ENV_WEBHOOK: "http://127.0.0.1:9/none"}):
                sent = notify.send("finished", "done", root=tmp)
            self.assertTrue(sent["webhook"].startswith("failed:"))
            config = Path(tmp) / "notify.json"
            config.write_text(json.dumps(dict(hook="exit 0", events=["dead"])))
            with unittest.mock.patch.dict(os.environ, {notify.ENV_FILE: str(config)}):
                self.assertEqual(notify.send("finished", "not one of its events", root=tmp), {})
                self.assertEqual(notify.send("dead", "one of them", root=tmp), dict(hook="ok"))


class EngineTest(unittest.TestCase):
    def repo(self, tmp: Path) -> Path:
        repo = tmp / "repo"
        (repo / "crates").mkdir(parents=True)
        (repo / "crates" / "a.rs").write_text("fn a() {}\n")
        (repo / "docs.md").write_text("words\n")
        for words in (["init", "-q"], ["add", "."], ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "one"]):
            subprocess.run(["git", *words], cwd=repo, check=True, capture_output=True)
        return repo

    def test_the_identity_is_the_engines_source_and_nothing_elses(self):
        real = quiet.engine.identify
        from importlib import reload  # noqa: PLC0415,F401

        identify = _real_identify
        with tempfile.TemporaryDirectory() as tmp:
            repo = self.repo(Path(tmp))
            one = identify(repo)
            self.assertEqual((len(one["id"]), one["dirty"]), (16, False))
            (repo / "docs.md").write_text("other words\n")
            self.assertEqual(identify(repo), one, "a file no binary is built from changes nothing")
            (repo / "crates" / "a.rs").write_text("fn a() { 1; }\n")
            changed = identify(repo)
            self.assertNotEqual(changed["id"], one["id"])
            self.assertTrue(changed["dirty"])
            (repo / "crates" / "new.rs").write_text("fn b() {}\n")
            self.assertNotEqual(identify(repo)["id"], changed["id"], "an untracked source file is part of it")
            self.assertIsNone(identify(Path(tmp))["id"], "no checkout: nothing to pin")
        self.assertIs(quiet.engine.identify, real)

    def test_a_pinned_build_is_kept_and_a_tree_that_moved_on_is_never_used(self):
        with tempfile.TemporaryDirectory() as tmp:
            root, repo = Path(tmp) / "root", Path(tmp) / "repo"
            release = repo / "target" / "release"
            release.mkdir(parents=True)
            exe = engine.EXE
            (release / f"cantrip-selfplay{exe}").write_bytes(b"one")
            (release / "onnxruntime.dll").write_bytes(b"lib")
            botlab = repo / "dotnet/tools/Cantrip.BotLab/bin/Release/net8.0"
            botlab.mkdir(parents=True)
            (botlab / "cantrip-botlab.dll").write_bytes(b"dll")
            (botlab / "other.dll").write_bytes(b"x")
            one, two = dict(id="aaaa", commit="c1", dirty=False), dict(id="bbbb", commit="c2", dirty=False)
            e = engine.Engine(root, repo, one, one)
            self.assertEqual(e.path("selfplay"), release / f"cantrip-selfplay{exe}", "not pinned yet: the tree's, the same source")
            with unittest.mock.patch.dict(os.environ):
                # Published for a kind of worker by some earlier build of another source: no part of this engine.
                stale = repo / "dotnet/tools/Cantrip.BotLab/bin/publish/linux-x64"
                stale.mkdir(parents=True)
                (stale / "cantrip-botlab").write_bytes(b"old")
                (release / f"cantrip-infer{exe}").write_bytes(b"a build of some other day")
                self.assertEqual(e.adopt(set()), [], "nothing this run built: nothing kept")
                self.assertEqual(sorted(e.adopt({"selfplay", "botlab"})), ["botlab/cantrip-botlab.dll", f"release/cantrip-selfplay{exe}"])
                self.assertFalse((root / "builds" / "aaaa" / "botlab-publish").exists())
                self.assertFalse((root / "builds" / "aaaa" / "release" / f"cantrip-infer{exe}").exists())
                self.assertEqual(e.adopt({"selfplay", "botlab"}), [], "kept already, and the same")
                # The same source built again and the file differs: the kept one is replaced.
                (release / f"cantrip-selfplay{exe}").write_bytes(b"one, rebuilt")
                self.assertEqual(e.adopt({"selfplay"}), [f"release/cantrip-selfplay{exe}"])
                self.assertEqual((root / "builds" / "aaaa" / "release" / f"cantrip-selfplay{exe}").read_bytes(), b"one, rebuilt")
                (release / f"cantrip-selfplay{exe}").write_bytes(b"one")
                e.adopt({"selfplay"})
                kept = root / "builds" / "aaaa" / "release" / f"cantrip-selfplay{exe}"
                self.assertEqual(e.path("selfplay"), kept)
                self.assertTrue((kept.parent / "onnxruntime.dll").exists())
                self.assertTrue((root / "builds" / "aaaa" / "botlab" / "other.dll").exists())
                self.assertEqual(e.path("botlab"), root / "builds" / "aaaa" / "botlab" / "cantrip-botlab.dll")
                self.assertEqual(os.environ["CANTRIP_ROOT"], str(repo))
            # The tree is rebuilt from another source: the generation still runs what it pinned.
            (release / f"cantrip-selfplay{exe}").write_bytes(b"two")
            moved = engine.Engine(root, repo, one, two)
            self.assertEqual(moved.adopt({"selfplay", "explore"}), [])
            # Once a stage of the generation has begun, what is pinned stays: the same source built again is said,
            # not taken (the generation's ledgers name the binaries it began with).
            said = []
            frozen = engine.Engine(root, repo, one, one, log=said.append)
            self.assertEqual(frozen.adopt({"selfplay"}, frozen=True), [])
            self.assertEqual(frozen.path("selfplay").read_bytes(), b"one")
            self.assertIn("the generation keeps the pinned one", said[0])
            self.assertTrue(frozen.kept())
            # BotLab's directory is replaced whole, the old one renamed aside first and removed after.
            (botlab / "cantrip-botlab.dll").write_bytes(b"dll, rebuilt")
            (botlab / "other.dll").unlink()
            self.assertEqual(frozen.adopt({"botlab"}), ["botlab/cantrip-botlab.dll"])
            kept_botlab = root / "builds" / "aaaa" / "botlab"
            self.assertEqual(sorted(p.name for p in kept_botlab.iterdir()), ["cantrip-botlab.dll"])
            self.assertEqual([p.name for p in kept_botlab.parent.iterdir() if "old" in p.name or "tmp" in p.name], [])
            self.assertEqual(moved.path("selfplay").read_bytes(), b"one")
            with self.assertRaises(SystemExit) as caught:
                moved.path("explore")
            self.assertIn("Check out commit c1", str(caught.exception.code))
            self.assertIn("--repin-engine", str(caught.exception.code))
            self.assertEqual(moved.find("explore"), root / "builds" / "aaaa" / "release" / f"cantrip-explore{exe}")
            # Retention's part: a build no kept generation names goes.
            (root / "builds" / "cccc").mkdir()
            engine.prune(root, {"aaaa"}, log=lambda *_: None)
            self.assertEqual(sorted(p.name for p in (root / "builds").iterdir()), ["aaaa"])

    def test_the_builds_of_champions_and_pinned_generations_are_kept(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for n in range(1, 7):
                (root / f"gen{n}").mkdir()
                (root / f"gen{n}" / "generation.json").write_text(json.dumps(dict(engine=dict(id=f"e{n}"))))
            (root / "champions" / "gen2").mkdir(parents=True)
            (root / "gen3" / "PINNED").write_text("")
            self.assertEqual(engine.named(root, 2), {"e2", "e3", "e5", "e6"})

    def test_a_generation_keeps_its_engine_and_mining_refuses_another(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            record = json.loads((root / "gen2" / "generation.json").read_text())
            record["engine"] = dict(id="aaaa", commit="c1", dirty=False)
            record["stages"]["gate"]["engine"] = "aaaa"
            (root / "gen2" / "generation.json").write_text(json.dumps(record))
            (root / "gen2" / "logs").mkdir()
            (root / "gen2" / "logs" / "gate-results.tsv").write_text("")
            moved = lambda repo: dict(id="bbbb", commit="c2", dirty=False)  # noqa: E731
            with unittest.mock.patch.object(engine, "identify", moved):
                # The tree moved on: the generation says so, keeps its engine, and has no build of it to run.
                with self.assertRaises(SystemExit) as caught:
                    quietly(["--root", str(root), "--generation", "2", *BASE], run=lambda *a, **k: "")
                self.assertIn("this generation keeps its own build", quietly.out)
                self.assertIn("this generation's engine is aaaa", str(caught.exception.code))
                # Re-pinned: it goes on with the tree's, and the gate another engine played is not mined.
                with self.assertRaises(SystemExit) as caught:
                    quietly(["--root", str(root), "--generation", "2", *BASE, "--repin-engine"], run=lambda *a, **k: "")
                self.assertIn("the gate was played by engine aaaa and this generation's engine is now bbbb", str(caught.exception.code))
                self.assertIn("MismatchedPositions", str(caught.exception.code))
                self.assertEqual(json.loads((root / "gen2" / "generation.json").read_text())["engine"]["id"], "bbbb")


_real_identify = None


def setUpModule():
    global _real_identify
    import importlib  # noqa: PLC0415
    import importlib.util  # noqa: PLC0415

    # The module's own `identify`, which `quiet` replaced for every other test.
    spec = importlib.util.spec_from_file_location("cantrip_ai._engine_real", engine.__file__,
                                                  submodule_search_locations=None)
    module = importlib.util.module_from_spec(spec)
    module.__package__ = "cantrip_ai"
    spec.loader.exec_module(module)
    _real_identify = module.identify


class LedgerTest(unittest.TestCase):
    def test_a_ledger_that_holds_nothing_is_replaced_and_one_with_games_refuses(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "ledger.json"
            then = {"global": dict(server_args="--precision fp16"), "hosts": {}}
            now = {"global": dict(server_args="--precision fp16 --fp16-graph products"), "hosts": {}}
            led = distributed.Ledger.load(path, 10, 5, identity=then, header=dict(h=1))
            led.save()
            said = []
            again = distributed.Ledger.load(path, 10, 5, identity=now, header=dict(h=1), log=said.append)
            self.assertEqual(again.identity["global"], now["global"])
            self.assertIn("holds no games yet: replaced", said[0])
            self.assertIn("server_args", said[0])
            # One game accepted, and the settings bind.
            d = json.loads(path.read_text())
            d["shards"] = {"a.results.tsv": dict(indices=[[0, 2]], host="here")}
            path.write_text(json.dumps(d))
            with self.assertRaises(SystemExit) as caught:
                distributed.Ledger.load(path, 10, 5, identity=now, header=dict(h=1), log=said.append)
            self.assertIn("was made with other settings", str(caught.exception.code))
            # So does an attempt that was started.
            d["shards"], d["attempts"] = {}, {"x": dict(id="x", host="here", granted=[[0, 2]], state="running", grant_ended=False)}
            path.write_text(json.dumps(d))
            with self.assertRaises(SystemExit):
                distributed.Ledger.load(path, 10, 5, identity=now, header=dict(h=1), log=said.append)


DRIVER = """
import json, os, sys, time
root, plan = sys.argv[1], sys.argv[2]
if '--preflight' in sys.argv and os.path.exists(os.path.join(root, 'preflight.hang')):
    time.sleep(120)
if '--preflight' in sys.argv and os.path.exists(os.path.join(root, 'preflight.once')):
    text = open(os.path.join(root, 'preflight.once')).read()
    os.remove(os.path.join(root, 'preflight.once'))
    print(text)
    sys.exit(1)
if '--preflight' in sys.argv:
    sys.exit(1 if os.path.exists(os.path.join(root, 'preflight.fail')) else 0)
steps = json.load(open(plan))
step = steps.pop(0)
json.dump(steps, open(plan, 'w'))
gen = os.path.join(root, 'gen1')
os.makedirs(gen, exist_ok=True)
now = time.time()
s = dict(generation=1, state='running', stage=step.get('stage', 'train'), pid=os.getpid(), host=__import__('socket').gethostname(),
         stage_started=now, progress_at=now, updated=now, expected_seconds=None)
if step['end'] == 'early':
    sys.exit(2)
if step['end'] != 'dead':
    s.update(state=step['end'], error=step.get('error'), transient=step.get('transient'))
json.dump(s, open(os.path.join(gen, 'status.json'), 'w'))
if step['end'] == 'finished':
    json.dump(dict(generation=1, finished='now', stages={}), open(os.path.join(gen, 'generation.json'), 'w'))
sys.exit(0 if step['end'] == 'finished' else 1)
"""


class LoopTest(unittest.TestCase):
    def loop(self, root: Path, steps: list[dict], **options) -> loop.Loop:
        (root / "driver.py").write_text(DRIVER)
        (root / "plan.json").write_text(json.dumps(steps))
        args = SimpleNamespace(**{**dict(poll=0.05, retries=2, retry_wait=0.0, stall_factor=0.25, stall_floor=900.0,
                                         renotify=3600.0, no_preflight=False, update=None, preflight_timeout=60.0), **options})
        made = loop.Loop(root, args, [])
        made.driver = lambda *extra: [sys.executable, str(root / "driver.py"), str(root), str(root / "plan.json"), *extra]
        made.log = lambda text: None
        made.strangers = lambda: []  # the process table is another test's
        return made

    def test_a_transient_failure_is_run_again_and_the_generation_finishes(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, [dict(end="failed", stage="selfplay", transient="selfplay-memory",
                                         error="self-play stopped by its governor (memory under the floor"),
                                    dict(end="finished")])
            self.assertEqual(made.run(), loop.OK)
            self.assertEqual([n["event"] for n in notifications(root)], ["retry"])
            self.assertIn("retry 1 of 2", notifications(root)[0]["message"])
            self.assertFalse((root / loop.PAUSE).exists())
            self.assertIn("supervisor: gen1", (root / "gen1-driver.log").read_text())

    def test_a_failure_nobody_knows_holds_the_loop_and_tells(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, [dict(end="failed", stage="parity", error="failed (101): cargo test model_parity\nmore")])
            self.assertEqual(made.run(), loop.HELD)
            held = (root / loop.PAUSE).read_text()
            self.assertIn("gen1 failed at stage parity: failed (101): cargo test model_parity", held)
            told = notifications(root)
            self.assertEqual([n["event"] for n in told], ["held"])
            self.assertEqual(status.read_json(root / status.LOOP)["state"], "held")

    def test_transient_failures_are_retried_a_bounded_number_of_times(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            failing = dict(end="failed", stage="selfplay", transient="server-start", error="the inference server did not start")
            made = self.loop(root, [failing] * 5, retries=2)
            self.assertEqual(made.run(), loop.HELD)
            self.assertEqual([n["event"] for n in notifications(root)], ["retry", "retry", "held"])
            self.assertEqual(len(json.loads((root / "plan.json").read_text())), 2, "three runs, no more")

    def test_a_dead_driver_a_stopped_one_and_one_that_never_started(self):
        for step, event, words in ((dict(end="dead", stage="gate"), "dead", "DIED at stage gate without a last word"),
                                   (dict(end="stopped", stage="selfplay", error="stop file"), "stopped", "was stopped at stage selfplay"),
                                   (dict(end="early"), "held", "before it ran a stage")):
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                made = self.loop(root, [step])
                self.assertEqual(made.run(), loop.HELD)
                told = notifications(root)
                self.assertEqual([n["event"] for n in told], [event])
                self.assertIn(words, told[0]["message"])

    def test_a_failed_preflight_holds_before_anything_runs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, [dict(end="finished")])
            (root / "preflight.fail").write_text("")
            self.assertEqual(made.run(), loop.HELD)
            self.assertIn("the preflight of the next generation failed (exit 1)", (root / loop.PAUSE).read_text())
            self.assertEqual(len(json.loads((root / "plan.json").read_text())), 1, "the generation was not run")

    def test_a_stall_is_told_once_and_its_end_too(self):
        import socket  # noqa: PLC0415

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, [])
            clock = [10_000.0]
            made.clock = lambda: clock[0]
            (root / "gen1").mkdir()
            s = dict(generation=1, state="running", stage="train", pid=os.getpid(), host=socket.gethostname(),
                     stage_started=0.0, expected_seconds=4000.0, progress_at=5000.0, updated=9990.0)
            (root / "gen1" / status.STATUS).write_text(json.dumps(s))
            told: dict = {}
            made.watch(1, lambda: True, told)
            made.watch(1, lambda: True, told)
            self.assertEqual([n["event"] for n in notifications(root)], ["stalled"], "told once")
            clock[0] += 3700
            (root / "gen1" / status.STATUS).write_text(json.dumps(dict(s, updated=clock[0] - 5)))
            made.watch(1, lambda: True, told)
            self.assertEqual([n["event"] for n in notifications(root)][-1], "stalled", "told again while it lasts")
            (root / "gen1" / status.STATUS).write_text(json.dumps(dict(s, updated=clock[0] - 5, progress_at=clock[0] - 5)))
            made.watch(1, lambda: True, told)
            self.assertEqual([n["event"] for n in notifications(root)][-2:], ["resumed", "warning"],
                             "its end is told, and that the stage is far past its usual time")

    def test_a_running_driver_is_attached_to_before_any_update_or_preflight(self):
        import socket  # noqa: PLC0415

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, [], update="origin/main")
            (root / "gen3").mkdir()
            (root / "gen3" / "generation.json").write_text(json.dumps(dict(generation=3, stages={})))
            mine = dict(generation=3, state="running", stage="selfplay", pid=os.getpid(), host=socket.gethostname(),
                        pid_start=distributed.process_start(os.getpid()), updated=1.0)
            (root / "gen3" / status.STATUS).write_text(json.dumps(mine))
            gen, unfinished, live = made.pending()
            self.assertEqual((gen, unfinished, live["pid"]), (3, True, os.getpid()))
            did = []
            made.preflight = lambda: did.append("preflight") or loop.OK
            made.run_once = lambda gen: did.append(f"run gen{gen}") or ("finished", {})
            made.git = lambda *words, timeout=None: did.append(f"git {words[0]}") or SimpleNamespace(returncode=0, stdout="", stderr="")
            self.assertEqual(made.update(), loop.OK)
            self.assertEqual(made.run(), loop.OK)
            self.assertEqual(did, ["run gen3"], "no fetch, no merge and no preflight beside a running driver")
            # The same pid, another process (the pid was reused): nothing to attach to, and still no update - the
            # generation is unfinished.
            (root / "gen3" / status.STATUS).write_text(json.dumps(dict(mine, pid_start=mine["pid_start"] + 1)))
            self.assertIsNone(made.pending()[2])
            did.clear()
            self.assertEqual(made.update(), loop.OK)
            self.assertEqual(made.run(), loop.OK)
            self.assertEqual(did, ["preflight", "run gen3"], "resumed after its preflight; the code not touched")
            # Finished: between generations, the update runs.
            (root / "gen3" / "generation.json").write_text(json.dumps(dict(generation=3, finished="now", stages={})))
            (root / "gen3" / status.STATUS).write_text(json.dumps(dict(mine, state="finished")))
            did.clear()
            made.update()
            self.assertEqual(did[0], "git fetch")

    def test_a_git_command_or_a_preflight_that_hangs_is_killed_and_told(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, [dict(end="finished")], update="origin/main", preflight_timeout=1.0)
            real = subprocess.Popen

            def hanging(cmd, **kw):
                return real([sys.executable, "-c", "import time; time.sleep(120)"], **kw)

            with unittest.mock.patch.object(loop.subprocess, "Popen", hanging):
                answer = made.git("fetch", "origin", timeout=0.5)
            self.assertEqual(answer.returncode, 124)
            told = notifications(root)
            self.assertEqual([n["event"] for n in told], ["warning"])
            self.assertIn("git fetch origin` did not end in 0 s and was killed", told[0]["message"].replace("0.5", "0"))
            (root / "preflight.hang").write_text("")
            self.assertEqual(made.run(), loop.HELD)
            self.assertIn("did not end in 1 s and was killed", (root / loop.PAUSE).read_text())
            self.assertEqual(len(json.loads((root / "plan.json").read_text())), 1, "the generation was not run")

    def test_a_server_that_did_not_start_at_the_preflight_is_tried_again_and_a_parity_failure_is_not(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, [dict(end="finished")])
            (root / "preflight.once").write_text("preflight failed\nthe inference server did not start\nCUDA error: out of memory")
            self.assertEqual(made.run(), loop.OK)
            told = notifications(root)
            self.assertEqual([n["event"] for n in told], ["retry"])
            self.assertIn("server-start", told[0]["message"])
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, [dict(end="finished")])
            (root / "preflight.once").write_text("the inference server did not start\ninfer: gen13 references FAIL at fp16: Check {}")
            self.assertEqual(made.run(), loop.HELD)
            self.assertEqual([n["event"] for n in notifications(root)], ["held"], "running it again changes nothing")

    def test_an_interrupt_ends_the_preflight_it_started(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, [])
            (root / "preflight.hang").write_text("")
            real, started = subprocess.Popen, []

            class Interrupted(real):
                def wait(self, timeout=None):
                    if not started:
                        started.append(self)
                        raise KeyboardInterrupt  # Ctrl-C at the supervisor, which the preflight's own group does not hear
                    return super().wait(timeout)

            with unittest.mock.patch.object(loop.subprocess, "Popen", Interrupted):
                with self.assertRaises(KeyboardInterrupt):
                    made.preflight_once(root / "preflight.log")
            self.assertIsNotNone(started[0].poll(), "the preflight was killed on the way out")

    def test_nothing_is_started_beside_a_driver_that_writes_no_status(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            table = [(11, f'python.exe -m cantrip_ai.generation --root {root} --games 6000 --workers all'),
                     (12, f'python -m cantrip_ai.generation --root "{root}" --dry-run'),
                     (13, f'python -m cantrip_ai.generation --root={root} --preflight'),
                     (14, 'python -m cantrip_ai.generation --root D:/another/line --games 6000'),
                     (15, f'python -m cantrip_ai.loop --root {root}'),
                     (16, f"bash -c 'python -m cantrip_ai.generation --root {str(root).upper() if os.name == 'nt' else root} --lanes 2'"),
                     (os.getpid(), f'python -m cantrip_ai.generation --root {root}')]
            self.assertEqual(status.driver_processes(root, table), [11, 16])
            # And in the real process table: a process with a driver's command line on this root, found.
            child = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(120)", "-m", "cantrip_ai.generation",
                                      "--root", str(root), "--games", "6"])
            try:
                self.assertIn(child.pid, status.driver_processes(root))
                made = self.loop(root, [dict(end="finished")])
                del made.strangers
                self.assertEqual(made.run(), loop.HELD)
                self.assertIn("writes no status file", (root / loop.PAUSE).read_text())
                self.assertIn(str(child.pid), (root / loop.PAUSE).read_text())
                self.assertEqual(len(json.loads((root / "plan.json").read_text())), 1, "no preflight, no second driver")
                self.assertFalse((root / "preflight").exists())
            finally:
                child.kill()
                child.wait()

    def test_one_supervisor_a_root_and_a_backslash_in_the_arguments_is_told(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / loop.LOCK).write_text(json.dumps(dict(pid=os.getpid(), start=distributed.process_start(os.getpid()))))
            with self.assertRaises(SystemExit) as caught:
                loop.main(["--root", str(root)])
            self.assertIn("another supervisor", str(caught.exception.code))
            bad = "--decks-dir D:" + chr(92) + "data" + chr(92) + "decks --games 6\n"
            good = '--train-args "--decks a' + chr(92) + 'b" --root-of \'C:' + chr(92) + "x y'  # a comment with a " + chr(92) + "\n"
            (root / "generation.args").write_text(bad + good + "--games 6 " + chr(92) + "\n--lanes 2\n")
            doubts = []
            words = loop.read_args(root / "generation.args", doubts.append)
            self.assertEqual(len(doubts), 1, doubts)
            self.assertIn("an unquoted backslash is dropped", doubts[0])
            self.assertIn("--decks-dir D:", doubts[0])
            self.assertIn("D:datadecks", words, "which is what the shell's rules make of it")

    def test_the_boundary_waits_on_the_pause_file_and_ends_on_the_stop_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, [])
            self.assertEqual(made.boundary(), loop.OK)
            (root / loop.PAUSE).write_text("landing code\n")
            waits = []

            def sleep(seconds):
                waits.append(seconds)
                if len(waits) == 3:
                    (root / loop.PAUSE).unlink()

            made.sleep = sleep
            self.assertEqual(made.boundary(), loop.OK)
            self.assertEqual(len(waits), 3)
            (root / loop.STOP).write_text("")
            self.assertEqual(made.boundary(), loop.END)

    def test_the_arguments_file_and_the_update_by_fast_forward_only(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "generation.args").write_text("# the line's settings\n--games 6000 --simulations 70  # search\n"
                                                  "--train-args \"--residual-l2 0.02 --searched-policy\"\n--decks-dir D:/decks\n")
            self.assertEqual(loop.read_args(root / "generation.args"),
                             ["--games", "6000", "--simulations", "70", "--train-args", "--residual-l2 0.02 --searched-policy",
                              "--decks-dir", "D:/decks"])
            # A tree with local changes, or a ref that is no fast-forward, holds the loop; nothing is merged.
            made = self.loop(root, [], update="origin/main")
            answers = {("fetch", "origin"): (0, ""), ("rev-parse", "HEAD"): (0, "aaa\n"), ("rev-parse", "origin/main"): (0, "bbb\n"),
                       ("merge-base", "--is-ancestor", "bbb", "HEAD"): (1, ""),
                       ("status", "--porcelain", "--untracked-files=no"): (0, " M ai/cantrip_ai/x.py\n")}
            asked = []

            def git(*words, timeout=None):
                asked.append(words)
                code, out = answers[words]
                return SimpleNamespace(returncode=code, stdout=out, stderr="")

            made.git = git
            self.assertEqual(made.update(), loop.HELD)
            self.assertIn("has local changes", (root / loop.PAUSE).read_text())
            self.assertNotIn(("merge", "--ff-only", "origin/main"), asked)
            answers[("status", "--porcelain", "--untracked-files=no")] = (0, "")
            answers[("merge", "--ff-only", "origin/main")] = (1, "fatal: Not possible to fast-forward")
            self.assertEqual(made.update(), loop.HELD)
            self.assertIn("not a fast-forward", (root / loop.PAUSE).read_text())
            answers[("merge", "--ff-only", "origin/main")] = (0, "")
            self.assertEqual(made.update(), loop.OK)
            answers[("rev-parse", "origin/main")] = (0, "aaa\n")
            asked.clear()
            self.assertEqual(made.update(), loop.OK)
            self.assertNotIn(("merge", "--ff-only", "origin/main"), asked)



class MinedTargetsTest(unittest.TestCase):
    """The mining window's check, at the preflight: before a game is played, not three hours in at training."""

    def lost(self, tmp: Path, legacy: bool) -> Path:
        failed = dict(done=False, failed="mine verify: UnboundLocalError", seconds=1.0, finished="then")
        root = promoted_root(tmp, mine=failed, mined=dict(failed, failed="the mine stage started no judgements"))
        record = json.loads((root / "gen2" / "generation.json").read_text())
        record["finished"] = "then"
        if legacy:
            record.pop("engine", None)
        else:
            record["engine"] = dict(id=None, commit=None, dirty=None)
        (root / "gen2" / "generation.json").write_text(json.dumps(record))
        return root

    def test_targets_lost_before_this_driver_are_flagged_and_told_once_never_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self.lost(Path(tmp), legacy=True)
            for _ in range(2):  # the driver run twice: told once
                quietly(["--root", str(root), "--generation", "3", *BASE, "--stop-after", "build"], run=lambda *a, **k: "")
            text = "gen2's mined targets were lost before the hardened driver; training without them"
            record = json.loads((root / "gen3" / "generation.json").read_text())
            self.assertEqual(record["missing_mined"], [text])
            self.assertIn(f"> - **{text}**", (root / "gen3" / "README.md").read_text())
            told = [n for n in notifications(root) if n["event"] == "warning"]
            self.assertEqual([n["message"] for n in told], [text])
            # And the supervisor's preflight passes on it.
            quietly(["--root", str(root), "--generation", "3", *BASE, "--preflight"], run=lambda *a, **k: "")
            self.assertIn("preflight WARNING: " + text, quietly.out)

    def test_a_failure_under_this_driver_refuses_at_the_preflight_unless_allowed(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = self.lost(Path(tmp), legacy=False)
            ran = []
            for mode in (["--stop-after", "selfplay"], ["--preflight"], ["--dry-run"]):
                with self.assertRaises(SystemExit) as caught:
                    quietly(["--root", str(root), "--generation", "3", *BASE, *mode], run=lambda cmd, *a, **k: ran.append(cmd) or "")
                why = str(caught.exception.code)
                self.assertIn("preflight failed; nothing was run", why)
                self.assertIn("training would go without gen2's mined targets (its mine and mined failed)", why)
            self.assertEqual(ran, [], "no self-play was started")
            quietly(["--root", str(root), "--generation", "3", *BASE, "--stop-after", "build", "--allow-data-failure"],
                    run=lambda *a, **k: "")
            flagged = "training without gen2's mined targets (its mine and mined failed) (--allow-data-failure)"
            self.assertEqual(json.loads((root / "gen3" / "generation.json").read_text())["missing_mined"], [flagged])
            self.assertEqual([n["message"] for n in notifications(root) if n["event"] == "warning"], [flagged])

    def test_a_preflight_run_writes_and_bootstraps_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = root_with(Path(tmp), gens=(1,))  # no champion yet: a real run would make gen0 the first
            before = sorted(str(p.relative_to(root)) for p in root.rglob("*"))
            quietly(["--root", str(root), "--generation", "2", *BASE, "--preflight"], run=lambda *a, **k: "")
            self.assertEqual(sorted(str(p.relative_to(root)) for p in root.rglob("*")), before)


class ServingFailuresTest(unittest.TestCase):
    def test_a_server_that_does_not_start_for_another_reason_is_not_a_parity_failure(self):
        self.assertTrue(generation.PARITY_FAILED.search("infer: gen14 references FAIL at fp16: Check { value: 0.0126 }"))
        self.assertTrue(generation.PARITY_FAILED.search("D:/models/gen14: the served model is not its references (...)"))
        self.assertTrue(generation.PARITY_FAILED.search("thread 'model_parity' panicked at tests/eval.rs:435:\nremote: Parity { positions: 512 }"))
        self.assertFalse(generation.PARITY_FAILED.search("remote batch 64: Parity { positions: 512 }\ntest model_parity ... ok"))
        self.assertFalse(generation.PARITY_FAILED.search("CUDA error: out of memory"))
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            started = []

            class Server:
                def __init__(self, models, log_path, device, extra, dry_run, log=print, **kw):
                    self.extra = extra
                    self.address = "127.0.0.1:1"

                def __enter__(self):
                    started.append(" ".join(self.extra))
                    raise SystemExit("the inference server did not start\n--- server-candidate.log ---\nCUDA error: out of memory")

                def __exit__(self, *_):
                    pass

            record = json.loads((root / "gen2" / "generation.json").read_text())
            for name in ("parity", "gate", "promote"):
                record["stages"].pop(name)
            (root / "gen2" / "generation.json").write_text(json.dumps(record))
            with self.assertRaises(SystemExit) as caught:
                quietly(["--root", str(root), "--generation", "2", "--no-build", "--decks-dir", "", "--gpu-server", "on",
                         "--device", "cpu", "--server-impl", "native", "--server-args", "--precision fp16",
                         "--stop-after", "parity"], run=lambda *a, **k: "", InferenceServer=Server)
            self.assertEqual(started, ["--precision fp16"], "no slower option was tried")
            self.assertIn("the inference server did not start", str(caught.exception.code))
            self.assertNotIn("serving", json.loads((root / "gen2" / "generation.json").read_text()).get("serving", {}).get("candidate", {}))
            said = status.read(root, 2)
            self.assertEqual((said["state"], said["transient"]), ("failed", "server-start"), "the supervisor's to retry")

    def test_the_duel_servers_flags_are_checked_and_a_workers_own_take_the_precision(self):
        from test_generation import decks_in, pool_of  # noqa: PLC0415

        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            decks = decks_in(Path(tmp))
            started = []

            class Server:
                def __init__(self, models, log_path, device, extra, dry_run, log=print, native=False, **kw):
                    self.models, self.extra, self.native = models, extra, native
                    self.address = "127.0.0.1:1"

                def __enter__(self):
                    started.append((" ".join(self.extra), self.native, [m.name for m in self.models]))
                    if "--batch 96" in " ".join(self.extra) and "--fp16-graph" not in self.extra and "fp32" not in self.extra:
                        raise SystemExit("the inference server did not start\ninfer: gen0 references FAIL at fp16: Check {}")
                    return self

                def __exit__(self, *_):
                    pass

            record = json.loads((root / "gen2" / "generation.json").read_text())
            for name in ("parity", "gate", "promote"):
                record["stages"].pop(name)
            (root / "gen2" / "generation.json").write_text(json.dumps(record))
            worker = dict(name="w1", host="192.0.2.1", user="u", os="windows", root="D:/c",
                          worker=dict(server_args="--precision fp16 --batch 200"))
            common = ["--root", str(root), "--generation", "2", "--no-build", "--decks-dir", str(decks), "--gpu-server", "on",
                      "--device", "cpu", "--server-impl", "python", "--server-args", "--precision fp16",
                      "--duel-server-args", "--precision fp16 --batch 96", "--workers", "w1", "--no-preflight",
                      "--ort-dylib", str(Path(tmp) / "onnxruntime.dll")]
            with pool_of(Path(tmp), worker):
                quietly([*common, "--stop-after", "parity"], run=lambda *a, **k: "ok", InferenceServer=Server)
                pick = json.loads((root / "gen2" / "generation.json").read_text())["serving"]["candidate"]
                # The configured flags pass in the generation's own server and fail in the duel hosts': the option fails.
                self.assertEqual(started[:2], [("--precision fp16", False, ["gen2", "gen0"]),
                                               ("--precision fp16 --batch 96", True, ["gen2", "gen0"])])
                self.assertEqual((pick["index"], pick["option"]), (1, "--precision fp32"))
                self.assertEqual(pick["checked"], ["--precision fp32", "--batch 96 --precision fp32"])
                self.assertEqual(pick["tried"][0]["args"], "--precision fp16 --batch 96")
                # The gate's hosts serve the checked flags, and a worker's own arguments take the precision.
                rec = generation.Record(root / "gen3", 3, False)
                rec.data.update(champion="gen0", serving=dict(start=dict(pick, models=["gen0"])))
                rec.save()
                plan = quietly([a for a in common if a != "2"][:3] + ["3", *common[4:], "--dry-run"])
            self.assertIn("cantrip-infer --batch 200 --precision fp32", plan)


class NitsTest(unittest.TestCase):
    def test_the_loaders_workers_are_read_in_either_spelling(self):
        self.assertEqual(generation.loader_workers(["--batch", "64"]), generation.TRAIN_WORKERS)
        self.assertEqual(generation.loader_workers(["--workers", "6", "--batch", "64"]), 6)
        self.assertEqual(generation.loader_workers(["--workers=5"]), 5)
        self.assertEqual(generation.loader_workers(["--workers", "6", "--workers=0"]), 0)

    def test_a_hooks_time_limit_holds_what_it_started_and_nothing_in_a_payload_raises(self):
        import time  # noqa: PLC0415

        with tempfile.TemporaryDirectory() as tmp:
            script = Path(tmp) / "hook.py"
            # A hook that leaves a child behind holding its output, and does not end itself.
            script.write_text("import subprocess, sys, time\n"
                              "subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(60)'])\n"
                              "time.sleep(60)\n")
            began = time.monotonic()
            answer = notify._hook(f'"{sys.executable}" "{script}"', notify.payload("test", "m"), timeout=1.0)
            self.assertLess(time.monotonic() - began, 20)
            self.assertIn("no answer in 1 s; killed", answer)
            # Whatever the details hold, sending does not raise, and the line is logged.
            sent = notify.send("warning", "odd details", root=tmp, where=Path(tmp), things={1, 2}, thing=object())
            self.assertEqual(sent, {})
            self.assertEqual(notifications(Path(tmp))[0]["message"], "odd details")
            with unittest.mock.patch.object(notify, "config", side_effect=RuntimeError("a broken configuration")):
                self.assertIn("RuntimeError", notify.send("warning", "x", root=tmp)["error"])



class OneDriverTest(unittest.TestCase):
    def test_the_driver_does_not_start_beside_a_live_driver_of_the_generation(self):
        import socket  # noqa: PLC0415

        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            other = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(120)"])
            try:
                theirs = dict(generation=2, state="running", stage="gate", pid=other.pid, host=socket.gethostname(),
                              pid_start=distributed.process_start(other.pid), updated=__import__("time").time())
                (root / "gen2" / status.STATUS).write_text(json.dumps(theirs))
                record = (root / "gen2" / "generation.json").read_text()
                ran = []
                with self.assertRaises(SystemExit) as caught:
                    quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine"], run=lambda *a, **k: ran.append(a) or "")
                self.assertIn(f"gen2 is being run by a driver already (pid {other.pid}", str(caught.exception.code))
                self.assertEqual(ran, [])
                self.assertEqual(json.loads((root / "gen2" / status.STATUS).read_text()), theirs, "its status is not written over")
                self.assertEqual((root / "gen2" / "generation.json").read_text(), record)
                # A dry run beside it plans, and touches nothing.
                quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine", "--dry-run"])
            finally:
                other.kill()
                other.wait()
            # Its process gone, the generation is this driver's to resume.
            quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine"], run=lambda *a, **k: "")
            self.assertEqual(status.read(root, 2)["state"], "finished")


class StampTest(unittest.TestCase):
    def test_a_stage_names_an_engine_only_while_a_build_of_it_is_pinned(self):
        ident = lambda repo: dict(id="aaaa", commit="c1", dirty=False)  # noqa: E731
        with tempfile.TemporaryDirectory() as tmp, unittest.mock.patch.object(engine, "identify", ident):
            root = promoted_root(Path(tmp))
            (root / "gen2" / "logs").mkdir()
            (root / "gen2" / "logs" / "gate-results.tsv").write_text("")
            # --no-build, and no build of this engine was ever kept: the stages run whatever the tree holds.
            with self.assertRaises(SystemExit):
                quietly(["--root", str(root), "--generation", "2", *BASE], run=lambda *a, **k: "")
            self.assertIn("nothing is pinned (no build this run)", quietly.out)
            self.assertIn("the gate's record names no pinned engine", quietly.out)
            self.assertIn("cannot be checked beforehand", quietly.out)
            quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine"], run=lambda *a, **k: "")
            record = json.loads((root / "gen2" / "generation.json").read_text())
            self.assertEqual(record["engine"]["id"], "aaaa", "the generation's engine is named")
            self.assertNotIn("engine", record["stages"]["retire"], "but no stage vouches for binaries nobody kept")
        with tempfile.TemporaryDirectory() as tmp, unittest.mock.patch.object(engine, "identify", ident):
            root = promoted_root(Path(tmp))
            (root / "builds" / "aaaa").mkdir(parents=True)
            (root / "builds" / "aaaa" / "engine.json").write_text("{}")
            quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine"], run=lambda *a, **k: "")
            record = json.loads((root / "gen2" / "generation.json").read_text())
            self.assertEqual(record["stages"]["retire"]["engine"], "aaaa", "a pinned build: the stages ran it")


if __name__ == "__main__":
    unittest.main()
