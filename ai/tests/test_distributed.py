"""Distributed self-play's split and bookkeeping (`cantrip_ai.distributed`), with fake hosts and no SSH: every game index
played into the generation exactly once, across hosts of different speeds, a worker that crashes, one that goes
unreachable and comes back, a stop, and a coordinator that restarts; and after review: a launch that never started, a
recovery that keeps failing, grant writes that fail (the last one too), fetches and stops that fail, an accepted shard
gone missing, settings that changed, a header that is not this run's, a host dropping games, a lost attempt's host
not used again until the attempt is seen dead, pulled shards deleted, the ledger's lock, and process identity."""

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
from cantrip_ai.distributed import Poll, Unreachable  # noqa: E402

SEED = 1000
HEADER = dict(seeds=None, producer="selfplay:fake@8", mode="decklist", text_hash="h")


def header(games: int) -> dict:
    return dict(HEADER, seeds=f"{SEED}..{SEED + games}")


def read(path: Path):
    """A fake shard: JSON of its header and its games' indices."""
    d = json.loads(path.read_text())
    return d["header"], d["indices"], dict(games=len(d["indices"]))


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


class FakeRun:
    """One attempt's self-play: each poll plays up to `rate` granted games (in grant order), closing a shard every
    `shard` games; it exits 0 once its grant is ended and played, 3 once asked to stop."""

    def __init__(self, host, a, grant: str):
        self.host, self.id = host, a["id"]
        self.granted, self.ended = parse_grant(grant)
        self.taken: list[int] = []
        self.open: list[int] = []
        self.shards: list[str] = []
        self.exit = None
        self.stopping = False
        self.n = 0
        self.runaways: list[int] = []
        self.dropped = 0
        self.killed = False

    def close(self):
        if self.open:
            name = f"{self.id}-{len(self.shards):03}.shard.zst"
            (self.host.dir / name).write_text(json.dumps(dict(header=header(self.host.games), indices=self.open)))
            self.shards.append(name)
            self.open = []

    def advance(self):
        if self.exit is not None:
            return
        # It reads its grant file as the coordinator last wrote it there.
        self.granted, self.ended = parse_grant(self.host.files[self.id])
        if self.stopping:
            self.close()
            self.exit = 3
            return
        for _ in range(self.host.rate):
            left = [i for i in self.granted if i not in self.taken]
            if not left:
                break
            i = left[0]
            self.taken.append(i)
            if i in self.host.runaway:
                self.runaways.append(i)
            elif i in self.host.fail:
                self.dropped += 1
            elif i in self.host.fail_once:
                self.host.fail_once.discard(i)
                self.dropped += 1
            else:
                self.open.append(i)
            if len(self.open) >= self.host.shard:
                self.close()
        self.n += 1
        if self.host.crash_at is not None and self.n == self.host.crash_at:
            self.host.crash_at = None
            self.exit = 1  # its open games in a .part, until it is recovered
            return
        if self.ended and all(i in self.taken for i in self.granted):
            self.close()
            self.exit = 0


class FakeHost:
    """A host whose runs are `FakeRun`s: remote (shards fetched from its own directory) or local (written into the
    self-play directory itself)."""

    def __init__(self, name: str, base: Path, games: int, rate: int, remote=True, shard=5, selfplay: Path | None = None):
        self.name, self.remote, self.games, self.rate, self.shard = name, remote, games, rate, shard
        self.dir = selfplay if not remote else base / name
        self.dir.mkdir(parents=True, exist_ok=True)
        self.runs: dict[str, FakeRun] = {}
        self.down = False
        self.crash_at = None
        self.runaway: set[int] = set()
        self.fail: set[int] = set()
        self.fail_once: set[int] = set()
        self.fail_end = 0
        self.launches = 0
        self.files: dict[str, str] = {}
        self.fail_writes = 0
        self.deleted: list[str] = []
        self.dropping = False

    def up(self):
        if self.down:
            raise Unreachable(f"{self.name} is down")

    def launch(self, a, grant):
        self.up()
        self.launches += 1
        self.files[a["id"]] = grant
        self.runs[a["id"]] = FakeRun(self, a, grant)
        return dict(pids=dict(selfplay=1), gpu=f"fake-{self.name}")

    def grant(self, a, text):
        self.up()
        if self.fail_writes:
            self.fail_writes -= 1
            raise Unreachable("the write failed")
        if self.fail_end and text.endswith("end\n"):
            self.fail_end -= 1
            raise Unreachable("the end's write failed")
        self.files[a["id"]] = text

    def poll(self, a):
        self.up()
        if a["id"] not in self.runs:
            return Poll(alive=False, started=False)
        self.deleted += a.get("to_delete") or []
        r = self.runs[a["id"]]
        r.advance()
        status = dict(Pid=1, Taken=len(r.taken), GamesPerHour=r.host.rate * 120.0, Seconds=30.0 * r.n,
                      Dropped=r.dropped, GamesThisRun=len(r.taken) - r.dropped,
                      Governor=dict(LanesAllowed=4, LanesCeiling=8, Running=2, Limiter="throughput"))
        return Poll(status=status, exit=r.exit, alive=r.exit is None, shards=list(r.shards) if self.remote else [],
                    runaways=list(r.runaways), report=dict(Dropped=r.dropped) if r.exit is not None else None,
                    started=True, grant=self.files[a["id"]])

    def fetch(self, a, name, dest):
        self.up()
        shutil.copy(self.dir / name, dest)

    def fetch_logs(self, a, dest):
        pass

    def delete(self, a, names):
        self.up()
        self.deleted += names

    def recover(self, a):
        self.up()
        self.runs[a["id"]].close()

    def stop(self, a, now):
        self.up()
        self.runs[a["id"]].stopping = True

    def kill(self, a):
        self.up()
        r = self.runs.get(a["id"])
        if r is not None and r.exit is None:
            r.exit, r.killed = -9, True


class Clock:
    def __init__(self):
        self.t = 0.0
        self.hooks: list = []

    def __call__(self):
        return self.t

    def monotonic(self):
        return self.t

    def sleep(self, s):
        self.t += s
        for hook in list(self.hooks):
            hook(self.t)


def setup(tmp: Path, games: int):
    selfplay, d = tmp / "gen1" / "selfplay", tmp / "gen1" / "dist"
    selfplay.mkdir(parents=True)
    return selfplay, d


def coordinator(ledger, hosts, selfplay, d, clock, **kw):
    kw = dict(dict(chunk=5, lookahead=0.0, initial=5, poll_seconds=30, timeout=120, report_every=1e9), **kw)
    return dist.Coordinator(ledger, hosts, selfplay, d, read=read, log=lambda *_: None, clock=clock, sleep=clock.sleep,
                            monotonic=clock.monotonic, **kw)


def ledger(d: Path, games: int, **kw) -> dist.Ledger:
    return dist.Ledger.load(d / "ledger.json", games, SEED, header=header(games), **kw)


def accepted(selfplay: Path) -> list[int]:
    """Every game index in the self-play directory's shards (a duplicate shows twice)."""
    return sorted(i for p in selfplay.glob("*.shard.zst") for i in read(p)[1])


class DistributedTest(unittest.TestCase):
    def test_ranges_and_grant_files(self):
        self.assertEqual(dist.to_ranges([5, 1, 2, 3, 9, 2]), [[1, 4], [5, 6], [9, 10]])
        self.assertEqual(dist.from_ranges([[1, 4], [9, 10]]), [1, 2, 3, 9])
        self.assertEqual(dist.grant_text([[0, 3], [7, 8]], False), "0..3\n7\n")
        self.assertEqual(dist.grant_text([[0, 3]], True), "0..3\nend\n")

    def test_the_ledger_grants_each_index_once_and_accepts_only_its_owners_shards(self):
        led = dist.Ledger(None, 20, SEED)
        led.header = header(20)
        a, b = led.new_attempt("x"), led.new_attempt("y")
        led.grant(a, list(range(0, 10)))
        with self.assertRaises(AssertionError):
            led.grant(b, [9])
        led.grant(b, list(range(10, 15)))
        self.assertEqual(led.pending(), list(range(15, 20)))
        self.assertIsNone(led.check(header(20)))
        self.assertIn("seeds", led.check(header(21)))
        self.assertIsNone(led.accept(a["id"], "a0", [0, 1, 2], {}))
        self.assertIn("accounted", led.accept(b["id"], "b0", [2], {}) or "")
        self.assertIn("not y-a1", led.accept(b["id"], "b1", [3], {}) or "")
        self.assertIn("twice", led.accept(a["id"], "a1", [4, 4], {}) or "")
        # A released attempt's games go back to pending, and what it sends later is no one's.
        self.assertEqual(led.release(a), list(range(3, 10)))
        self.assertEqual(led.pending(), list(range(3, 10)) + list(range(15, 20)))
        self.assertIn("owned by no attempt", led.accept(a["id"], "a2", [5], {}) or "")
        c = led.new_attempt("x")
        self.assertEqual(c["id"], "x-a2")
        led.grant(c, [5])
        self.assertIsNone(led.accept(c["id"], "c0", [5], {}))
        # A shard there before distribution: its indices must be no one's.
        self.assertIsNone(led.accept(None, "old", [19], {}))
        self.assertIn("owned by y-a1", led.accept(None, "old2", [12], {}) or "")

    def test_the_ledger_reads_back_what_it_wrote(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "ledger.json"
            led = dist.Ledger(path, 12, SEED)
            a = led.new_attempt("x")
            led.grant(a, list(range(8)))
            led.accept(a["id"], "s", [0, 1], {})
            led.runaway(a["id"], 2)
            led.save()
            again = dist.Ledger.load(path, 12, SEED)
            self.assertEqual(sorted(again.done), [0, 1])
            self.assertEqual(again.owned(a["id"]), [3, 4, 5, 6, 7])
            self.assertEqual(again.pending(), [8, 9, 10, 11])
            self.assertEqual(set(again.runaways), {2})
            with self.assertRaises(SystemExit):
                dist.Ledger.load(path, 13, SEED)

    def run_to_end(self, tmp: Path, games: int, make_hosts, before=None, **kw):
        selfplay, d = setup(tmp, games)
        clock = Clock()
        hosts = make_hosts(tmp, selfplay)
        led = ledger(d, games)
        c = coordinator(led, hosts, selfplay, d, clock, **kw)
        if before:
            before(c, clock, hosts)
        result = c.run()
        return result, led, hosts, selfplay, clock

    def test_every_game_once_across_a_local_a_fast_and_a_slow_host(self):
        with tempfile.TemporaryDirectory() as tmp:
            def hosts(t, sp):
                return [FakeHost("here", t, 300, 3, remote=False, selfplay=sp), FakeHost("fast", t, 300, 20),
                        FakeHost("slow", t, 300, 1)]
            result, led, hs, selfplay, _ = self.run_to_end(Path(tmp), 300, hosts, lookahead=0.01)
            self.assertEqual(result, "done")
            self.assertEqual(accepted(selfplay), list(range(300)))
            by = dist.host_rows(led)
            self.assertGreater(by["fast"]["games"], 5 * by["slow"]["games"], by)
            self.assertGreater(by["here"]["games"], 0)
            # The slow host never held more than its backlog: the fast one played what it did not take.
            slow = [a for a in led.attempts.values() if a["host"] == "slow"]
            self.assertLess(sum(a["granted_count"] for a in slow), 60)
            manifest = dist.hosts_manifest(led)
            self.assertEqual(sum(len(dist.from_ranges(s["games"])) for s in manifest["shards"].values()), 300)
            self.assertTrue(all(s["host"] in ("here", "fast", "slow") for s in manifest["shards"].values()))

    def test_a_worker_that_crashes_is_recovered_and_its_rest_played_by_the_others(self):
        with tempfile.TemporaryDirectory() as tmp:
            def hosts(t, sp):
                w = FakeHost("w", t, 120, 7)
                w.crash_at = 3
                return [FakeHost("here", t, 120, 2, remote=False, selfplay=sp), w]
            result, led, hs, selfplay, _ = self.run_to_end(Path(tmp), 120, hosts)
            self.assertEqual(result, "done")
            self.assertEqual(accepted(selfplay), list(range(120)))
            crashed = led.attempts["w-a1"]
            self.assertEqual(crashed["state"], "crashed")
            self.assertEqual(crashed["exit"], 1)
            # The games its .part held were recovered, not played again.
            self.assertTrue(any(s["attempt"] == "w-a1" for s in led.shards.values()))
            self.assertGreaterEqual(hs[1].launches, 2, "a new attempt on it after the crash")

    def test_an_unreachable_worker_is_lost_its_games_go_to_the_others_and_it_is_stopped_when_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            def hosts(t, sp):
                return [FakeHost("here", t, 100, 2, remote=False, selfplay=sp), FakeHost("w", t, 100, 5)]

            def before(c, clock, hs):
                w = hs[1]

                def flaky(t):
                    if t == 90:
                        w.down = True
                    if t == 600:
                        w.down = False
                clock.hooks.append(flaky)
            result, led, hs, selfplay, _ = self.run_to_end(Path(tmp), 100, hosts, before=before, max_failures=5)
            self.assertEqual(result, "done")
            self.assertEqual(accepted(selfplay), list(range(100)))
            lost = led.attempts["w-a1"]
            self.assertEqual(lost["state"], "lost")
            self.assertTrue(lost["released"])
            # Back, it was told to stop; nothing it played after it was lost was taken.
            run = hs[1].runs["w-a1"]
            self.assertTrue(run.stopping or run.exit is not None)
            taken_from_it = sum(len(dist.from_ranges(s["indices"])) for s in led.shards.values() if s["attempt"] == "w-a1")
            self.assertLess(taken_from_it, len(run.taken), "what it played while lost was never taken")

    def test_a_late_shard_of_a_lost_attempt_is_rejected_and_kept_out_of_the_training_set(self):
        with tempfile.TemporaryDirectory() as tmp:
            selfplay, d = setup(Path(tmp), 30)
            clock = Clock()
            w = FakeHost("w", Path(tmp), 30, 5)
            led = ledger(d, 30)
            c = coordinator(led, [w], selfplay, d, clock)
            c.launch()
            a = led.attempts["w-a1"]
            run = w.runs["w-a1"]
            run.advance()  # plays 0..4 into a shard nobody pulls yet
            led.release(a)
            a["state"] = "lost"
            b = led.new_attempt("w")
            led.grant(b, list(range(5)))
            # The lost attempt's shard turns up: every index of it is someone else's now.
            c.take(a, w, run.shards[0])
            self.assertEqual(accepted(selfplay), [])
            self.assertEqual(len(led.rejected), 1)
            self.assertTrue((d / "rejected" / f"w-{run.shards[0]}").exists())

    def test_a_coordinator_restart_picks_up_where_it_was(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            selfplay, d = setup(tmp, 150)
            clock = Clock()
            hs = [FakeHost("here", tmp, 150, 3, remote=False, selfplay=selfplay), FakeHost("w", tmp, 150, 6)]

            class Crash(Exception):
                pass

            def crash(t):
                if t == 150:
                    raise Crash
            clock.hooks.append(crash)
            led = ledger(d, 150)
            with self.assertRaises(Crash):
                coordinator(led, hs, selfplay, d, clock).run()
            clock.hooks.clear()
            some = len(led.done)
            self.assertGreater(some, 0)
            self.assertLess(some, 150)
            # A new coordinator, the same hosts (the runs went on meanwhile): from the ledger on disk.
            for h in hs:
                for r in h.runs.values():
                    r.advance()
            again = ledger(d, 150)
            self.assertEqual(coordinator(again, hs, selfplay, d, clock).run(), "done")
            self.assertEqual(accepted(selfplay), list(range(150)))
            self.assertEqual(sum(h.launches for h in hs), 2, "the live attempts re-attached, none started anew")

    def test_a_stop_stops_every_host_and_a_rerun_finishes_the_rest(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            selfplay, d = setup(tmp, 100)
            clock = Clock()
            hs = [FakeHost("here", tmp, 100, 2, remote=False, selfplay=selfplay), FakeHost("w", tmp, 100, 4)]

            def ask(t):
                if t == 120:
                    (selfplay / dist.STOP_FILE).write_text("")
            clock.hooks.append(ask)
            led = ledger(d, 100)
            self.assertEqual(coordinator(led, hs, selfplay, d, clock).run(), "stopped")
            self.assertTrue(all(r.exit == 3 for h in hs for r in h.runs.values()))
            first = accepted(selfplay)
            self.assertEqual(first, sorted(set(first)))
            self.assertLess(len(first), 100)
            self.assertEqual(led.owner, {}, "a stopped attempt owns nothing")
            clock.hooks.clear()
            (selfplay / dist.STOP_FILE).unlink()
            again = ledger(d, 100)
            self.assertEqual(coordinator(again, hs, selfplay, d, clock).run(), "done")
            self.assertEqual(accepted(selfplay), list(range(100)))

    def test_runaways_and_failed_games_are_accounted_as_one_run_accounts_them(self):
        with tempfile.TemporaryDirectory() as tmp:
            def hosts(t, sp):
                w = FakeHost("w", t, 40, 6)
                w.runaway, w.fail = {7}, {11}
                return [w]
            result, led, hs, selfplay, _ = self.run_to_end(Path(tmp), 40, hosts)
            self.assertEqual(result, "done")
            self.assertEqual(accepted(selfplay), [i for i in range(40) if i not in (7, 11)])
            self.assertEqual(set(led.runaways), {7})
            self.assertEqual(set(led.dropped), {11})
            self.assertTrue(led.complete())

    def test_shards_there_before_distribution_count_and_are_not_played_again(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            selfplay, d = setup(tmp, 30)
            (selfplay / "old.shard.zst").write_text(json.dumps(dict(header=header(30), indices=[0, 1, 2, 3])))
            clock = Clock()
            led = ledger(d, 30)
            self.assertEqual(coordinator(led, [FakeHost("w", tmp, 30, 5)], selfplay, d, clock).run(), "done")
            self.assertEqual(accepted(selfplay), list(range(30)))
            self.assertIsNone(led.shards["old.shard.zst"]["attempt"])

    def test_a_host_that_keeps_failing_is_given_up_and_the_rest_play_on(self):
        with tempfile.TemporaryDirectory() as tmp:
            def hosts(t, sp):
                bad = FakeHost("bad", t, 50, 5)
                bad.down = True
                return [FakeHost("here", t, 50, 3, remote=False, selfplay=sp), bad]
            result, led, hs, selfplay, _ = self.run_to_end(Path(tmp), 50, hosts, retry_seconds=60)
            self.assertEqual(result, "done")
            self.assertEqual(accepted(selfplay), list(range(50)))
            self.assertEqual(hs[1].launches, 0)

    def test_a_workers_poll_reads_what_its_script_prints_and_ssh_failing_is_unreachable(self):
        calls = []

        class R:
            def __init__(self, code, out=b"", err=b""):
                self.returncode, self.stdout, self.stderr = code, out, err

        answers = []

        def run(cmd, **kw):
            calls.append((cmd, kw.get("input")))
            return answers.pop(0)

        w = dict(name="w", host="10.0.0.2", user="worker", root="D:/compute", key="k")
        h = dist.SshHost(w, "fp-abc", 6, {}, {}, "h", None, log=lambda *_: None, run=run)
        a = dict(id="w-a1")
        status = json.dumps(dict(Pid=7, Taken=3))
        printed = dict(status=status, exit={"value": json.dumps(dict(code=3)), "PSPath": "x"}, pids=json.dumps(dict(selfplay=7)),
                       alive=False, shards="one.shard.zst", runaways='{"index": 4}\n', report=json.dumps(dict(Dropped=0)))
        answers.append(R(0, ("noise\n" + json.dumps(printed)).encode()))
        p = h.poll(a)
        self.assertEqual((p.exit, p.alive, p.shards, p.runaways, p.status["Taken"]), (3, False, ["one.shard.zst"], [4], 3))
        self.assertEqual(a["pids"], dict(selfplay=7))
        cmd, script = calls[-1]
        self.assertEqual(cmd[:2], ["ssh", "-o"])
        self.assertIn("worker@10.0.0.2", cmd)
        self.assertIn(b"D:/compute/gens/fp-abc/gen6/w/w-a1", script)
        answers.append(R(255, err=b"connection refused"))
        with self.assertRaises(Unreachable):
            h.poll(a)

    def test_load_workers_reads_the_file(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = Path(tmp) / "workers.json"
            p.write_text(json.dumps(dict(key="k", workers=[dict(name="a", host="h", user="u", root="D:/c"),
                                                           dict(name="b", host="h2", user="u", root="D:/c", enabled=False)])))
            w = dist.load_workers(p)
            self.assertEqual([x["name"] for x in w], ["a"])
            self.assertEqual(w[0]["key"], "k")

    # After review.

    def test_a_launch_that_never_started_is_given_up_after_its_grace_and_its_games_played_elsewhere(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            selfplay, d = setup(tmp, 40)
            clock = Clock()
            w, here = FakeHost("w", tmp, 40, 5), FakeHost("here", tmp, 40, 4, remote=False, selfplay=selfplay)
            led = ledger(d, 40)
            # As an interrupted launch leaves it: saved, granted, nothing started there.
            a = led.new_attempt("w", started=0.0)
            led.grant(a, list(range(10)))
            led.save()
            result = coordinator(ledger(d, 40), [here, w], selfplay, d, clock, max_failures=5).run()
            self.assertEqual(result, "done")
            self.assertEqual(accepted(selfplay), list(range(40)))
            again = json.loads((d / "ledger.json").read_text())
            self.assertEqual(again["attempts"]["w-a1"]["state"], "failed")
            self.assertGreater(clock.t, dist.LAUNCH_GRACE, "not before its grace")

    def test_a_recovery_that_keeps_failing_is_given_up_after_the_timeout(self):
        with tempfile.TemporaryDirectory() as tmp:
            def hosts(t, sp):
                w = FakeHost("w", t, 60, 6)
                w.crash_at = 2

                def recover(a):
                    raise Unreachable("cantrip-selfplay --recover exited 2")
                w.recover = recover
                return [FakeHost("here", t, 60, 3, remote=False, selfplay=sp), w]
            result, led, hs, selfplay, clock = self.run_to_end(Path(tmp), 60, hosts, max_failures=1)
            self.assertEqual(result, "done")
            self.assertEqual(accepted(selfplay), list(range(60)))
            a = led.attempts["w-a1"]
            self.assertEqual(a["state"], "lost")
            self.assertIn("recovery failed", a["why"])
            self.assertEqual(hs[1].launches, 1, "a host that failed is not used again (max_failures 1)")

    def test_grant_writes_that_fail_the_end_too_are_written_again(self):
        with tempfile.TemporaryDirectory() as tmp:
            def hosts(t, sp):
                w = FakeHost("w", t, 50, 5)
                w.fail_writes, w.fail_end = 4, 3
                return [w]
            result, led, hs, selfplay, _ = self.run_to_end(Path(tmp), 50, hosts)
            self.assertEqual(result, "done")
            self.assertEqual(accepted(selfplay), list(range(50)))
            self.assertEqual((hs[0].fail_writes, hs[0].fail_end), (0, 0))
            self.assertTrue(hs[0].files["w-a1"].endswith("end\n"))

    def test_a_fetch_or_a_stop_that_fails_is_tried_again_and_does_not_end_the_driver(self):
        with tempfile.TemporaryDirectory() as tmp:
            def hosts(t, sp):
                w = FakeHost("w", t, 40, 5)
                fetch, stop = w.fetch, w.stop
                w.fetch_fails, w.stop_fails = 3, 2

                def flaky_fetch(a, name, dest):
                    if w.fetch_fails:
                        w.fetch_fails -= 1
                        raise Unreachable("scp: connection reset")
                    fetch(a, name, dest)

                def flaky_stop(a, now):
                    if w.stop_fails:
                        w.stop_fails -= 1
                        raise Unreachable("ssh: timed out")
                    stop(a, now)
                w.fetch, w.stop = flaky_fetch, flaky_stop
                return [w]

            def before(c, clock, hs):
                def ask(t):
                    if t == 150:
                        (c.dir / dist.STOP_FILE).write_text("")
                clock.hooks.append(ask)
            result, led, hs, selfplay, _ = self.run_to_end(Path(tmp), 40, hosts, before=before)
            self.assertEqual(result, "stopped")
            got = accepted(selfplay)
            self.assertEqual(got, sorted(set(got)))
            self.assertEqual(sorted(got), sorted(led.done))
            self.assertEqual(hs[0].fetch_fails, 0)

    def test_an_accepted_shard_that_is_gone_is_un_accepted_and_one_left_in_incoming_moved_in(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            selfplay, d = setup(tmp, 20)
            led = ledger(d, 20)
            a = led.new_attempt("w")
            led.grant(a, list(range(10)))
            for name, idx in (("w-x.shard.zst", [0, 1]), ("w-y.shard.zst", [2, 3])):
                led.accept(a["id"], name, idx, {})
            led.release(a)
            led.save()
            (d / "incoming").mkdir()
            (d / "incoming" / "w-y.shard.zst.pulling").write_text(json.dumps(dict(header=header(20), indices=[2, 3])))
            again = ledger(d, 20)
            gone = again.verify(selfplay, d / "incoming", log=lambda *_: None)
            self.assertEqual(gone, ["w-x.shard.zst"])
            self.assertTrue((selfplay / "w-y.shard.zst").exists())
            self.assertEqual(sorted(again.done), [2, 3])
            self.assertEqual(again.pending()[:2], [0, 1], "its games are played again")

    def test_a_shard_is_in_the_self_play_directory_before_it_is_recorded(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            selfplay, d = setup(tmp, 20)
            w = FakeHost("w", tmp, 20, 5)
            led = ledger(d, 20)
            c = coordinator(led, [w], selfplay, d, Clock())
            c.launch()
            w.runs["w-a1"].advance()
            seen = []
            save = led.save

            def checked_save():
                seen.extend(n for n in led.shards if not (selfplay / n).exists())
                save()
            led.save = checked_save
            self.assertTrue(c.take(led.attempts["w-a1"], w, w.runs["w-a1"].shards[0]))
            self.assertEqual(seen, [])
            self.assertEqual(len(led.shards), 1)

    def test_a_ledger_made_with_other_settings_is_refused_and_a_new_host_joins(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp)
            ident = {"global": dict(model="m1", simulations=64), "hosts": {"w": dict(bundle="b1", precision="fp16")}}
            ledger(d, 10, identity=ident).save()
            with self.assertRaises(SystemExit) as e:
                ledger(d, 10, identity={"global": dict(model="m1", simulations=128), "hosts": {}})
            self.assertIn("simulations", str(e.exception))
            with self.assertRaises(SystemExit) as e:
                ledger(d, 10, identity={"global": ident["global"], "hosts": {"w": dict(bundle="b1", precision="fp32")}})
            self.assertIn("w.precision", str(e.exception))
            joined = ledger(d, 10, identity={"global": ident["global"], "hosts": {"v": dict(bundle="b1")}})
            self.assertEqual(sorted(joined.identity["hosts"]), ["v", "w"])
            with self.assertRaises(SystemExit):
                dist.Ledger.load(d / "ledger.json", 10, SEED, header=dict(header(10), producer="selfplay:other@8"))

    def test_a_shard_with_another_header_is_rejected_and_counts_against_its_host(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            selfplay, d = setup(tmp, 20)
            w = FakeHost("w", tmp, 20, 5)
            led = ledger(d, 20)
            c = coordinator(led, [w], selfplay, d, Clock())
            c.launch()
            run = w.runs["w-a1"]
            run.advance()
            name = run.shards[0]
            (w.dir / name).write_text(json.dumps(dict(header=dict(header(20), producer="selfplay:stale@8"), indices=run.taken)))
            c.take(led.attempts["w-a1"], w, name)
            self.assertEqual(led.shards, {})
            self.assertIn("producer", led.rejected[0]["why"])
            self.assertEqual(c.failures("w"), 1)

    def test_a_host_dropping_games_is_stopped_and_its_games_go_to_the_others(self):
        with tempfile.TemporaryDirectory() as tmp:
            def hosts(t, sp):
                bad = FakeHost("bad", t, 60, 6)
                bad.fail = set(range(60))  # its server is gone: every game it plays fails
                return [FakeHost("here", t, 60, 3, remote=False, selfplay=sp), bad]
            result, led, hs, selfplay, _ = self.run_to_end(Path(tmp), 60, hosts, max_failures=1)
            self.assertEqual(result, "done")
            self.assertEqual(accepted(selfplay), list(range(60)))
            self.assertEqual(led.dropped, {})
            bad = led.attempts["bad-a1"]
            self.assertTrue(bad["dropping"])
            self.assertEqual(bad["state"], "stopped")
            self.assertEqual(hs[1].launches, 1, "not fed again")

    def test_a_game_that_fails_on_two_attempts_is_dropped_and_one_that_fails_once_is_played_again(self):
        with tempfile.TemporaryDirectory() as tmp:
            def hosts(t, sp):
                w, v = FakeHost("w", t, 30, 5), FakeHost("v", t, 30, 5)
                w.fail, v.fail, w.fail_once = {3}, {3}, {4}
                return [w, v]
            result, led, hs, selfplay, _ = self.run_to_end(Path(tmp), 30, hosts, drop_limit=10)
            self.assertEqual(result, "done")
            got = accepted(selfplay)
            self.assertNotIn(3, got)
            self.assertEqual(set(led.dropped), {3})
            self.assertIn(4, got, "failed once, played again")

    def test_a_lost_attempts_host_gets_no_new_one_until_it_is_seen_dead(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            selfplay, d = setup(tmp, 60)
            clock = Clock()
            w, here = FakeHost("w", tmp, 60, 2), FakeHost("here", tmp, 60, 1, remote=False, selfplay=selfplay)
            kill = w.kill
            w.kills = 0

            def slow_kill(a):
                w.kills += 1
                if w.kills >= 3:
                    kill(a)
            w.kill = slow_kill

            def flaky(t):
                if t == 60:
                    w.down = True
                if t == 300:
                    w.down = False
                if w.kills and not w.runs["w-a1"].killed:
                    self.assertEqual(w.launches, 1, "a second run on w while the lost one may still play")
            clock.hooks.append(flaky)
            led = ledger(d, 60)
            self.assertEqual(coordinator(led, [here, w], selfplay, d, clock, max_failures=5).run(), "done")
            self.assertEqual(accepted(selfplay), list(range(60)))
            self.assertTrue(w.runs["w-a1"].killed)
            self.assertTrue(led.attempts["w-a1"]["lost_over"])

    def test_pulled_shards_are_deleted_on_the_worker_at_the_next_poll(self):
        with tempfile.TemporaryDirectory() as tmp:
            result, led, hs, selfplay, _ = self.run_to_end(Path(tmp), 30, lambda t, sp: [FakeHost("w", t, 30, 5)])
            self.assertEqual(result, "done")
            pulled = [n for a in led.attempts.values() for n in a["pulled"]]
            self.assertEqual(sorted(hs[0].deleted), sorted(pulled), "every pulled shard, the last ones at the attempt's end")

    def test_the_ledger_lock_keeps_a_second_driver_out_and_is_taken_over_from_a_dead_one(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "ledger.lock"
            with dist.LedgerLock(path):
                with self.assertRaises(SystemExit):
                    with dist.LedgerLock(path):
                        pass
            self.assertFalse(path.exists())
            dead = subprocess.Popen([sys.executable, "-c", "pass"])
            dead.wait()
            path.write_text(json.dumps(dict(pid=dead.pid, start=1)))
            with dist.LedgerLock(path):
                self.assertEqual(json.loads(path.read_text())["pid"], os.getpid())

    def test_a_process_is_known_by_its_id_and_its_start_time(self):
        me = dist.process_start(os.getpid())
        self.assertIsNotNone(me)
        self.assertTrue(dist.process_matches(os.getpid(), me))
        self.assertFalse(dist.process_matches(os.getpid(), me + 1), "the same id, another process")
        self.assertFalse(dist.process_matches(None, None))

    def test_a_workers_server_arguments_refuse_shapes_and_default_to_the_native_servers(self):
        self.assertEqual(dist.worker_server_args(dict(name="w"), dist.WORKER_SERVER_ARGS), ["--precision", "fp16", "--pipelines", "1"])
        self.assertEqual(dist.worker_server_args(dict(name="w", server_args="--precision fp32"), "x"), ["--precision", "fp32"])
        with self.assertRaises(SystemExit):
            dist.worker_server_args(dict(name="w", server_args="--precision fp16 --shapes 32,64"), "")
        self.assertEqual(dist.precision_of(["--pipelines", "1"]), "fp32")

    def test_a_local_launch_leaves_a_stop_asked_in_place_and_its_lease_is_kept_by_polls(self):
        # Windows may hold the killed process's log a moment longer than it takes to clean up.
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
            tmp = Path(tmp)
            selfplay, d = setup(tmp, 10)
            (selfplay / dist.STOP_FILE).write_text("")
            host = dist.LocalHost("here", [sys.executable, "-c", "import time; time.sleep(30)"], selfplay, d, tmp / "logs", lease=60)
            a = dict(id="here-a1", state="launching", granted=[[0, 5]], grant_ended=False)
            a.update(host.launch(a, "0..5\n"))
            try:
                self.assertTrue((selfplay / dist.STOP_FILE).exists(), "a stop asked is not undone by a launch")
                beat = host.heartbeat(a)
                old = beat.stat().st_mtime - 100
                os.utime(beat, (old, old))
                p = host.poll(a)
                self.assertTrue(p.alive)
                self.assertGreater(beat.stat().st_mtime, old + 50, "the poll renewed its lease")
                self.assertEqual(p.grant, "0..5\n")
                # A coordinator that restarted knows it by its id and start time only.
                fresh = dist.LocalHost("here", host.command, selfplay, d, tmp / "logs")
                self.assertTrue(fresh.poll(a).alive)
                a["pids"]["selfplay"]["start"] += 1
                self.assertFalse(fresh.poll(a).alive, "another process with its id")
            finally:
                host.kill(a)


if __name__ == "__main__":
    unittest.main()
