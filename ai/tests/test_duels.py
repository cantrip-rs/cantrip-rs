"""Distributed duels (`cantrip_ai.duels`) over the self-play coordinator, with fake hosts and no SSH: every game of a
duel played once across hosts of different speeds, whole pairs to a host, a host that crashes or goes away, a game
that does not finish (played again once, then the duel's failure), the results merged as one machine writes them, a
sequential stop that closes the duel with the games in play counted, a subset of the games (a replay) each on the
host that played it until that host is gone, and replays' directories fetched whole. The last test plays a real duel
(tactical seats, no network) whole and split across this machine and a loopback worker driven through its PowerShell
scripts, and checks that the summaries are the same: it runs only with CANTRIP_DUEL_E2E=1 and the binaries built."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from cantrip_ai import distributed as dist  # noqa: E402
from cantrip_ai import duels  # noqa: E402
from cantrip_ai.distributed import Poll, Unreachable  # noqa: E402

SEED = 500
HEADER = {"format": "duel-results/1", "a": "x", "b": "y"}


def header(games: int) -> dict:
    return dict(HEADER, seeds=f"{SEED}..{SEED + games}")


def row(game: int, winner: str = "1") -> str:
    """A result line: A in seat 1 on even games; `winner` '1', '2', '0' or '-' (did not finish)."""
    seat = 2 if game % 2 else 1
    points = "-" if winner == "-" else "0.5" if winner == "0" else "1" if int(winner) == seat else "0"
    return "\t".join(map(str, [game, game // 2, SEED + game // 2, seat, "ux", "gx", winner, points, 1, 10]))


def winner_of(game: int) -> str:
    return "1" if game % 3 else "2"


class FakeRun:
    """One attempt's part of a duel: each poll plays up to `rate` granted games, in grant order, closing a results
    file every `chunk` games (a replay: one directory at the end); 0 once the grant is ended and played, 3 once asked
    to stop (the games in play finish: `in_play` of them)."""

    def __init__(self, host, a, grant: str):
        self.host, self.id = host, a["id"]
        self.granted, self.ended = parse_grant(grant)
        self.taken: list[int] = []
        self.open: list[str] = []
        self.outputs: list[str] = []
        self.exit = None
        self.stopping = False
        self.n = 0

    def close(self):
        if not self.open:
            return
        text = f"# {json.dumps(header(self.host.games))}\n{duels.COLUMNS}\n" + "".join(r + "\n" for r in self.open)
        if self.host.replay:
            name = f"{self.id}.replay"
            (self.host.dir / name / "record").mkdir(parents=True)
            (self.host.dir / name / "results.tsv").write_text(text)
            (self.host.dir / name / "record" / "traces").mkdir()
            for r in self.open:
                (self.host.dir / name / "record" / f"g{duels.game_of(r)}.shard.zst").write_text(r)
                (self.host.dir / name / "record" / "traces" / f"g{duels.game_of(r)}.jsonl").write_text(r)
        else:
            name = f"{self.id}-{len(self.outputs):03}.results.tsv"
            (self.host.dir / name).write_text(text)
        self.outputs.append(name)
        self.open = []

    def play(self, i: int):
        self.taken.append(i)
        self.host.played.append(i)
        bad = i in self.host.fail or (i in self.host.fail_once and not self.host.fail_once.discard(i))
        self.open.append(row(i, "-" if bad else winner_of(i)))
        if not self.host.replay and len(self.open) >= self.host.chunk:
            self.close()

    def advance(self):
        if self.exit is not None:
            return
        self.granted, self.ended = parse_grant(self.host.files[self.id])
        if self.stopping:
            # The games in play finish.
            for i in [i for i in self.granted if i not in self.taken][:self.host.in_play]:
                self.play(i)
            self.close()
            self.exit = 3
            return
        for _ in range(self.host.rate):
            left = [i for i in self.granted if i not in self.taken]
            if not left:
                break
            self.play(left[0])
        self.n += 1
        if self.host.crash_at is not None and self.n == self.host.crash_at:
            self.host.crash_at = None
            self.exit = 1
            return
        if self.ended and all(i in self.taken for i in self.granted):
            self.close()
            self.exit = 0


def parse_grant(text: str) -> tuple[list[int], bool]:
    out, ended = [], False
    for line in text.splitlines():
        line = line.strip()
        if line == "end":
            ended = True
        elif ".." in line:
            a, b = line.split("..")
            out += range(int(a), int(b))
        elif line:
            out.append(int(line))
    return out, ended


class FakeHost:
    """A host whose runs are `FakeRun`s: remote (outputs fetched from its own directory) or local (written into the
    duel's results directory)."""

    def __init__(self, name: str, base: Path, games: int, rate: int, remote=True, chunk=4, results: Path | None = None,
                 replay=False):
        self.name, self.remote, self.games, self.rate, self.chunk, self.replay = name, remote, games, rate, chunk, replay
        self.dir = results if not remote else base / name
        self.dir.mkdir(parents=True, exist_ok=True)
        self.runs: dict[str, FakeRun] = {}
        self.files: dict[str, str] = {}
        self.down = False
        self.crash_at = None
        self.fail: set[int] = set()
        self.fail_once: set[int] = set()
        self.played: list[int] = []
        self.granted: list[list[int]] = []
        self.in_play = 2
        self.stops = 0

    def up(self):
        if self.down:
            raise Unreachable(f"{self.name} is down")

    def launch(self, a, grant):
        self.up()
        self.files[a["id"]] = grant
        self.runs[a["id"]] = FakeRun(self, a, grant)
        self.granted.append(parse_grant(grant)[0])
        return dict(pids=dict(selfplay=1))

    def grant(self, a, text):
        self.up()
        self.files[a["id"]] = text
        self.granted.append(parse_grant(text)[0])

    def poll(self, a):
        self.up()
        if a["id"] not in self.runs:
            return Poll(alive=False, started=False)
        r = self.runs[a["id"]]
        r.advance()
        status = dict(Pid=1, Taken=len(r.taken), GamesPerHour=self.rate * 120.0, Seconds=30.0 * r.n, Dropped=0,
                      GamesThisRun=len(r.taken), Governor=dict(LanesAllowed=2, LanesCeiling=2, Running=1))
        return Poll(status=status, exit=r.exit, alive=r.exit is None, shards=list(r.outputs) if self.remote else [],
                    report={} if r.exit is not None else None, started=True, grant=self.files[a["id"]])

    def fetch(self, a, name, dest):
        self.up()
        src = self.dir / name
        if src.is_dir():
            shutil.copytree(src, dest)
        else:
            shutil.copy(src, dest)

    def fetch_logs(self, a, dest):
        pass

    def delete(self, a, names):
        self.up()

    def recover(self, a):
        self.up()
        self.runs[a["id"]].close()

    def stop(self, a, now):
        self.up()
        self.stops += 1
        self.runs[a["id"]].stopping = True

    def kill(self, a):
        self.up()
        r = self.runs.get(a["id"])
        if r is not None and r.exit is None:
            r.exit = -9


class DuelBase(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.results = self.tmp / "results"
        self.results.mkdir()
        self.logs: list[str] = []

    def tearDown(self):
        shutil.rmtree(self.tmp, ignore_errors=True)

    def coordinator(self, hosts, games, kind=duels.DUEL, only=None, affinity=None, stop_check=None, chunk=4, **kw):
        led = dist.Ledger.load(self.tmp / "ledger.json", games, SEED, header=header(games), only=only)
        t = [0.0]

        def sleep(s):
            t[0] += s
            if t[0] > 1e6:
                raise AssertionError("the run never ends")

        c = dist.Coordinator(led, hosts, self.results, self.tmp / "dist", read=duels.read_results, log=self.logs.append,
                             clock=lambda: t[0], sleep=sleep, monotonic=lambda: t[0], chunk=chunk, initial=chunk,
                             poll_seconds=30, kind=kind, affinity=affinity, stop_check=stop_check, **kw)
        return led, c


class DuelTest(DuelBase):
    def test_every_game_once_in_whole_pairs_and_merged_as_one_machine_writes_them(self):
        games = 40
        fast = FakeHost("fast", self.tmp, games, rate=5)
        slow = FakeHost("slow", self.tmp, games, rate=1)
        local = FakeHost("here", self.tmp, games, rate=3, remote=False, results=self.results)
        led, c = self.coordinator([local, fast, slow], games, chunk=3)
        self.assertEqual(c.run(), "done")
        played = local.played + fast.played + slow.played
        self.assertEqual(sorted(played), list(range(games)))
        # Each host was granted whole pairs, and every pair was played on one host.
        for h in (local, fast, slow):
            for g in h.granted:
                self.assertTrue(all(i // 2 * 2 in g and i // 2 * 2 + 1 in g for i in g), g)
        rows = duels.merged(led, self.results)
        self.assertEqual(sorted(duels.game_of(r) for r in rows), list(range(games)))
        self.assertEqual({duels.game_of(r): r for r in rows}, {i: row(i, winner_of(i)) for i in range(games)})
        # In the order the outputs were accepted.
        order = [duels.game_of(r) for name in led.shards for r in duels.read_rows(self.results / name)[1]]
        self.assertEqual([duels.game_of(r) for r in rows], order)
        duels.write_results(rows, self.tmp / "merged.tsv")
        self.assertEqual((self.tmp / "merged.tsv").read_text().splitlines()[0], duels.COLUMNS)

    def test_a_crash_and_a_host_that_goes_away_lose_only_their_games_in_flight(self):
        games = 30
        a = FakeHost("a", self.tmp, games, rate=2)
        b = FakeHost("b", self.tmp, games, rate=2)
        a.crash_at = 2
        led, c = self.coordinator([a, b], games, timeout=100)
        steps = [0]
        real = c.step

        def step():
            steps[0] += 1
            if steps[0] == 4:
                b.down = True
            real()

        c.step = step
        self.assertEqual(c.run(), "done")
        rows = duels.merged(led, self.results)
        self.assertEqual(sorted(duels.game_of(r) for r in rows), list(range(games)))
        self.assertTrue(any(x["state"] == "crashed" for x in led.attempts.values()))
        self.assertTrue(any(x["state"] == "lost" for x in led.attempts.values()))

    def test_a_game_that_does_not_finish_is_played_once_more_then_fails_the_duel(self):
        games = 12
        a = FakeHost("a", self.tmp, games, rate=3)
        b = FakeHost("b", self.tmp, games, rate=3)
        a.fail_once = {3}
        a.fail, b.fail = {7}, {7}
        led, c = self.coordinator([a, b], games)
        self.assertEqual(c.run(), "done")
        rows = {duels.game_of(r): r for r in duels.merged(led, self.results)}
        self.assertEqual(sorted(rows), list(range(games)))
        # Game 7 never finished: dropped, its last unfinished line the duel's (one machine's duel fails on it).
        self.assertIn(7, led.dropped)
        self.assertFalse(duels.finished(rows[7]))
        # Game 3 failed once and finished the second time: its finished line, once.
        self.assertTrue(duels.finished(rows[3]))
        self.assertEqual(led.fails.get(3), 1)
        self.assertNotIn(3, led.dropped)

    def test_a_sequential_stop_closes_the_duel_and_counts_the_games_in_play(self):
        games = 60
        a = FakeHost("a", self.tmp, games, rate=2)
        b = FakeHost("b", self.tmp, games, rate=2)

        def stop(ledger):
            return "SPRT verdict after 10 games" if len(ledger.done) >= 10 else None

        led, c = self.coordinator([a, b], games, stop_check=stop)
        self.assertEqual(c.run(), "done")
        self.assertTrue(led.closed.startswith("SPRT verdict"))
        self.assertGreaterEqual(a.stops + b.stops, 1)
        self.assertTrue(led.complete())
        played = set(a.played + b.played)
        self.assertEqual(set(led.done), played)
        self.assertEqual(set(led.skipped), set(range(games)) - played)
        self.assertGreater(len(led.done), 10)  # the games in play after the verdict count
        self.assertEqual(len(duels.merged(led, self.results)), len(played))

    def test_a_replay_plays_each_game_on_the_host_that_played_it_until_that_host_is_gone(self):
        games = 40
        wanted = [0, 2, 4, 6, 8, 10, 12, 14, 16, 18]
        affinity = {i: ("a" if i < 10 else "b") for i in wanted}
        a = FakeHost("a", self.tmp, games, rate=2, replay=True)
        b = FakeHost("b", self.tmp, games, rate=2, replay=True)
        c_host = FakeHost("c", self.tmp, games, rate=2, replay=True)
        led, c = self.coordinator([a, b, c_host], games, kind=duels.REPLAY, only=wanted, affinity=affinity)
        self.assertEqual(c.run(), "done")
        self.assertEqual(sorted(a.played), [0, 2, 4, 6, 8])
        self.assertEqual(sorted(b.played), [10, 12, 14, 16, 18])
        self.assertEqual(c_host.played, [])
        # Its recordings gathered, each shard once, under its output's name.
        n = duels.recordings(led, self.results, "record", self.tmp / "recorded")
        self.assertEqual(n, len(wanted))
        self.assertEqual(len(list((self.tmp / "recorded" / "traces").glob("*.jsonl"))), len(wanted))
        self.assertEqual(sorted(duels.game_of(r) for r in duels.merged(led, self.results)), wanted)
        self.assertEqual(duels.hosts_of(led, "here"), affinity)

    def test_a_replays_games_go_to_the_others_once_its_host_is_gone(self):
        games = 20
        wanted = [1, 3, 5, 7]
        a = FakeHost("a", self.tmp, games, rate=1, replay=True)
        b = FakeHost("b", self.tmp, games, rate=1, replay=True)
        a.down = True
        led, c = self.coordinator([a, b], games, kind=duels.REPLAY, only=wanted, affinity={i: "a" for i in wanted},
                                  timeout=90, retry_seconds=30)
        self.assertEqual(c.run(), "done")
        self.assertEqual(sorted(b.played), wanted)
        # Not before its host was out of reach for the timeout.
        self.assertTrue(any("not ready" in x or "could not start" in x for x in self.logs))

    def test_a_results_file_of_another_duel_is_rejected(self):
        games = 8
        a = FakeHost("a", self.tmp, games, rate=4)
        led = dist.Ledger.load(self.tmp / "ledger.json", games, SEED, header=dict(header(games), a="other"))
        c = dist.Coordinator(led, [a], self.results, self.tmp / "dist", read=duels.read_results, log=self.logs.append,
                             sleep=lambda s: None, kind=duels.DUEL, chunk=4, initial=4, max_failures=1)
        with self.assertRaises(SystemExit) as raised:
            c.run()
        self.assertIn("distributed duel: every host has failed", str(raised.exception))
        self.assertTrue(led.rejected)
        self.assertFalse(led.done)


class LocalStopHost(FakeHost):
    """A local host whose stop is a stop file in the duel's results directory, as `LocalHost.stop` writes it."""

    def stop(self, a, now):
        super().stop(a, now)
        (self.dir / duels.DUEL.stop).write_text("now" if now else "")


class ReadyLater(FakeHost):
    """A worker that answers but makes itself ready (a Linux worker building) until the clock passes `ready_at`."""

    def __init__(self, *a, clock=None, ready_at=0.0, **kw):
        super().__init__(*a, **kw)
        self.clock, self.ready_at = clock, ready_at

    def prepare(self, live):
        if self.clock() < self.ready_at:
            raise dist.NotReady(f"{self.name}: building: ...")


class ReviewFixes(DuelBase):
    def test_a_closed_duel_whose_local_run_is_asked_to_stop_ends_done_not_stopped(self):
        games = 40
        local = LocalStopHost("here", self.tmp, games, rate=2, remote=False, results=self.results)
        w = FakeHost("w", self.tmp, games, rate=2)
        led, c = self.coordinator([local, w], games, stop_check=lambda led: "SPRT verdict" if len(led.done) >= 6 else None)
        self.assertEqual(c.run(), "done")
        self.assertTrue((self.results / duels.DUEL.stop).exists(), "the local run was asked to stop")
        self.assertTrue(led.closed)

    def test_the_whole_runs_stop_file_stops_a_duel(self):
        games = 40
        a = FakeHost("a", self.tmp, games, rate=1)
        led, c = self.coordinator([a], games)
        (self.tmp / "dist").mkdir(parents=True, exist_ok=True)
        (self.tmp / "dist" / "stop").write_text("")
        self.assertEqual(c.run(), "stopped")

    def test_a_host_making_itself_ready_keeps_its_replays_past_the_timeout(self):
        games = 20
        wanted = [0, 2, 4, 6]
        t = [0.0]
        a = ReadyLater("a", self.tmp, games, rate=2, replay=True, clock=lambda: t[0], ready_at=900.0)
        b = FakeHost("b", self.tmp, games, rate=2, replay=True)
        led = dist.Ledger.load(self.tmp / "ledger.json", games, SEED, header=header(games), only=wanted)

        def sleep(s):
            t[0] += s

        c = dist.Coordinator(led, [a, b], self.results, self.tmp / "dist", read=duels.read_results, log=self.logs.append,
                             clock=lambda: t[0], sleep=sleep, monotonic=lambda: t[0], chunk=2, initial=2, poll_seconds=30,
                             timeout=100, retry_seconds=30, kind=duels.REPLAY, affinity={i: "a" for i in wanted})
        self.assertEqual(c.run(), "done")
        self.assertEqual(sorted(a.played), wanted)
        self.assertEqual(b.played, [])
        self.assertGreater(t[0], 900.0)

    def test_a_live_attempt_of_a_host_not_in_this_run_is_given_up_not_a_key_error(self):
        games = 12
        led = dist.Ledger.load(self.tmp / "ledger.json", games, SEED, header=header(games))
        stray = led.new_attempt("old")
        led.grant(stray, [0, 1, 2, 3])
        stray["state"] = "running"
        led.save()
        a = FakeHost("a", self.tmp, games, rate=3)
        led, c = self.coordinator([a], games, stop_check=lambda led: "SPRT verdict" if len(led.done) >= 4 else None)
        self.assertEqual(c.run(), "done")
        self.assertEqual(led.attempts["old-a1"]["state"], "lost")
        self.assertIn(0, a.played)

    def test_an_output_found_here_owned_by_no_attempt_is_this_machines(self):
        games = 2
        (self.results / "x.results.tsv").write_text(f"# {json.dumps(header(games))}\n{duels.COLUMNS}\n{row(0)}\n{row(1)}\n")
        local = FakeHost("here", self.tmp, games, rate=1, remote=False, results=self.results)
        led, c = self.coordinator([local], games)
        self.assertEqual(c.run(), "done")
        self.assertEqual(led.shards["x.results.tsv"]["host"], "here")
        self.assertEqual(duels.hosts_of(led, "here"), {0: "here", 1: "here"})

    def test_a_linux_bundle_is_complete_only_with_every_binary_its_run_needs(self):
        import base64  # noqa: PLC0415
        import contextlib  # noqa: PLC0415
        import io  # noqa: PLC0415
        root = self.tmp / "linux"
        b = root / "bundle" / "h1"
        (b / "bin").mkdir(parents=True)
        (b / "MANIFEST.json").write_text("{}")
        (b / "bin" / "cantrip-native-host").write_text("x")

        def have(binaries):
            args = base64.b64encode(json.dumps(dict(root=str(root), hash="h1", binaries=binaries)).encode()).decode()
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                exec(compile(dist._PY_PRELUDE.replace("@ARGS@", args) + dist._LINUX_HAVE, "have", "exec"), {})
            return json.loads(out.getvalue().strip().splitlines()[-1])["complete"]

        self.assertTrue(have(["cantrip-native-host"]))
        # A replay's bundle is a gate's (one hash) needing the recording host the gate's did not build.
        self.assertFalse(have(["cantrip-native-host", "cantrip-native-host-record"]))

    def test_a_distributed_duel_without_botlab_published_says_so(self):
        with self.assertRaises(SystemExit) as raised:
            duels.check_published({"windows": self.tmp / "none.exe", "linux": self.tmp / "none"},
                                  [{"name": "w", "os": "linux"}])
        self.assertIn("--no-build", str(raised.exception))
        duels.check_published({"windows": self.tmp / "none.exe"}, [])

    def test_a_local_run_through_a_wrapper_is_known_by_its_grant(self):
        import time  # noqa: PLC0415
        script = self.tmp / "run.py"
        script.write_text("import json, sys, time, pathlib\n"
                          "a = sys.argv\n"
                          "out = pathlib.Path(a[1])\n"
                          "grant = a[a.index('--grant') + 1]\n"
                          "(out / 'duel-status.json').write_text(json.dumps(dict(Pid=4, Grant=grant, Taken=0)))\n"
                          "time.sleep(20)\n")
        host = dist.LocalHost("here", [sys.executable, script, self.results], self.results, self.tmp / "dist",
                              self.tmp / "logs", kind=duels.DUEL)
        a = dict(id="here-a1", state="launching")
        a.update(host.launch(a, "0..2\n"))
        try:
            p = None
            for _ in range(100):
                p = host.poll(a)
                if p.status:
                    break
                time.sleep(0.1)
            self.assertEqual((p.status or {}).get("Pid"), 4)
        finally:
            host.kill(a)


def loopback(cmd, input=None, stdout=None, stderr=None, timeout=120.0):
    """`SshHost`'s transport, run here: an ssh command's remote line run as a local process, scp a local copy."""
    if cmd[0] == "ssh":
        p = subprocess.run(cmd[-1], input=input, capture_output=True, timeout=timeout)
        return dist._Ran(p.returncode, p.stdout, p.stderr)
    src, dst = (x.split(":", 1)[1] if "@" in x.split(":", 1)[0] else x for x in cmd[-2:])
    if Path(src).is_dir():
        shutil.copytree(src, dst)
    else:
        shutil.copyfile(src, dst)
    return dist._Ran(0, b"", b"")


REPO = Path(__file__).resolve().parents[2]


@unittest.skipUnless(os.environ.get("CANTRIP_DUEL_E2E") == "1" and os.name == "nt",
                     "a real duel, whole and split: CANTRIP_DUEL_E2E=1, with the native host, BotLab and its publish built")
class WholeAndSplitTest(unittest.TestCase):
    """A real duel (the tactical seat against itself less one behaviour: no network) played whole by BotLab, and split
    across this machine and a loopback worker (an `SshHost` whose scripts run here), gives the same games and the same
    summary."""

    def test_a_duel_split_across_hosts_is_the_duel_played_whole(self):
        tmp = Path(tempfile.mkdtemp(prefix="duel-e2e-"))
        try:
            self.run_both(tmp)
        finally:
            shutil.rmtree(tmp, ignore_errors=True)

    def run_both(self, tmp: Path):
        botlab = REPO / "dotnet/tools/Cantrip.BotLab/bin/Release/net8.0/cantrip-botlab.dll"
        published = REPO / "dotnet/tools/Cantrip.BotLab/bin/publish/win-x64/cantrip-botlab.exe"
        native = REPO / "target/cards/cantrip-native-host.exe"
        games = int(os.environ.get("CANTRIP_DUEL_E2E_GAMES", "16"))
        decks = REPO / "decks" / "standard"
        args = ["duel", "--a", "tactical", "--b", "tactical+policy.holdcounter=0", "--decks", str(decks), "--games",
                str(games), "--seed", "4242"]
        env = {**os.environ, "CANTRIP_NATIVE_HOST": str(native), "DOTNET_ROLL_FORWARD": "Major"}
        whole = tmp / "whole.tsv"
        p = subprocess.run(["dotnet", str(botlab), *args, "--parallel", "2", "--results", str(whole)], cwd=REPO, env=env,
                           capture_output=True, text=True, timeout=1800)
        self.assertEqual(p.returncode, 0, p.stdout + p.stderr)

        worker = dict(name="loop", host="localhost", user="me", root=str(tmp / "worker").replace("\\", "/"), os="windows",
                      duel_parallel=1)
        gen_dir = tmp / "ai" / "gen1"
        (gen_dir / "logs").mkdir(parents=True)
        setup = duels.Setup(root=tmp / "ai", gen=1, gen_dir=gen_dir, repo=REPO, workers=[worker], local=True,
                            local_name="here", botlab_local=["dotnet", str(botlab)],
                            botlab={"windows": published}, native=native, record_native=native, ai_host=native,
                            infer=native, ort_dir=None, cuda_dir=None, cuda_dlls=(), server_args=[], local_parallel=1,
                            worker_parallel=1, server=None, poll=2.0, timeout=600, chunk=4)
        made = []
        real = dist.SshHost.__init__

        def init(self, *a, **kw):
            real(self, *a, **{**kw, "run": loopback})
            made.append(self)

        dist.SshHost.__init__ = init
        try:
            ledger, out, rows, seconds = duels.run(setup, "e2e", args, [], games, 4242, decks, None)
        finally:
            dist.SshHost.__init__ = real
        self.assertTrue(made)
        hosts = duels.host_rows(ledger)
        self.assertEqual(set(hosts), {"here", "loop"}, hosts)

        # The same games: every line of the whole duel's results, and no other.
        whole_rows = sorted(whole.read_text().splitlines()[1:])
        self.assertEqual(sorted(rows), whole_rows)

        # The same summary, from BotLab's own counting, but for the line of time and the answers' (not in a results file).
        merged = tmp / "merged.tsv"
        duels.write_results(rows, merged)
        code, text = duels.summarize(setup, args, merged)
        self.assertEqual(code, 0)

        def lines(t):
            return [x for x in t.splitlines() if x.strip() and " games in " not in x and not x.startswith("An answer took")]

        self.assertEqual(lines(text), lines(p.stdout))
        self.assertIn("tactical beat tactical+policy.holdcounter=0 ", text)
        self.assertIn(f"; {games // 2} pairs, 0 incomplete", text)


if __name__ == "__main__":
    unittest.main()
