"""A stage that lives and moves at a fraction of its speed (docs/ai-training.md, "Watching a run", "The loop"): the
projection of a stage's end from its ledger's rate, what it is held against, and the supervisor stopping the stage's
work and holding the loop. Fakes and a fake clock: nothing here plays a game or waits on time, but for the one
supervisor run that waits for its (fake) driver to stop."""

from __future__ import annotations

import contextlib
import io
import json
import os
import socket
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import quiet  # noqa: E402,F401
from cantrip_ai import loop, notify, status  # noqa: E402

FAST = dict(simulations=70, full_simulations=8192, full_search=["Worlds=4"], decks="d")
DEEP = dict(simulations=70, full_simulations=131072, full_search=["Worlds=16"], decks="d")
HOSTS = {"w": dict(bundle="b1", backend="gpu", server_args="--precision fp16", precision="fp16")}


def identity(settings: dict, gen: int, games: int, hosts: dict | None = None) -> dict:
    """A self-play ledger's identity: the settings, and what every generation changes."""
    return {"global": dict(settings, model=f"m{gen}", seed=gen * 1000, games=games, pairings=f"p{gen}"),
            "hosts": {n: dict(h, bundle=f"bundle{gen}") for n, h in (hosts or HOSTS).items()}}


def ledger(games: int, played: int, ident: dict, attempts: dict | None = None, **more) -> dict:
    return {**dict(games=games, seed=1, identity=ident, attempts=attempts or {}, only=None, closed=None, skipped={},
                   runaways={}, dropped={}, fails={},
                   shards={"s": dict(attempt="w-a1", games=played, indices=[[0, played]])} if played else {}), **more}


def live(taken: int = 0, finished: int = 0, **more) -> dict:
    """An attempt at work, as its coordinator last saw it."""
    return dict(id="w-a1", host="w", state="running", started=0.0,
                status=dict(Taken=taken, GamesThisRun=finished, Dropped=0, Runaways=0), **more)


def counts(d: dict) -> dict:
    return {k: v for k, v in status.ledger_work(d).items() if k in ("total", "played", "skipped", "finished", "flight")}


def write(path: Path, d: dict):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(d), encoding="utf-8")


def earlier(root: Path, gen: int, settings: dict, games: int, seconds: float, hosts: dict | None = None,
            rel: str = "dist/ledger.json"):
    """Generation `gen` played `games` of `rel` to the end in `seconds`."""
    write(root / f"gen{gen}" / rel, ledger(games, games, identity(settings, gen, games, hosts),
                                           {"w-a1": dict(id="w-a1", host="w", state="finished", started=100.0,
                                                         ended=100.0 + seconds)}))


class Run:
    """A generation's driver at its self-play, on a fake clock: its heartbeat, and its ledger as the coordinator
    would write it."""

    def __init__(self, root: Path, gen: int, settings: dict, games: int, stage: str = "selfplay",
                 rel: str = "dist/ledger.json", start: float = 10_000.0):
        self.root, self.gen, self.games, self.settings, self.rel = root, gen, games, settings, rel
        self.clock = [start]
        self.beat = status.Heartbeat(root, gen, clock=lambda: self.clock[0])
        self.beat.gen_dir.mkdir(parents=True, exist_ok=True)
        self.beat.begin(stage)
        self.put(0)

    def put(self, played: int, taken: int | None = None, finished: int | None = None, **more):
        finished = played if finished is None else finished
        write(self.root / f"gen{self.gen}" / self.rel,
              ledger(self.games, played, identity(self.settings, self.gen, self.games),
                     {"w-a1": live(finished if taken is None else taken, finished)}, **more))
        self.beat.beat()

    def play(self, seconds: float, per_hour: float, lanes: int = 0, every: float = 60.0):
        """`seconds` of play at `per_hour` games an hour finished, `lanes` games in flight throughout."""
        played = max(self.played(), getattr(self, "count", 0.0))
        for _ in range(round(seconds / every)):
            self.clock[0] += every
            played += per_hour * every / 3600.0
            self.put(int(played), taken=int(played) + lanes)
        self.count = played  # the part of a game played so far is carried to the next call
        return self

    def played(self) -> float:
        return float((self.beat.data.get("work") or {}).get("finished") or 0)

    def judge(self, **limits) -> dict:
        return status.judge(status.read(self.root, self.gen), self.clock[0], alive=lambda pid: True, **limits)


class LedgerWorkTest(unittest.TestCase):
    def test_a_ledgers_games_are_those_accepted_those_finished_on_a_host_and_those_in_flight(self):
        d = ledger(100, 10, identity(FAST, 2, 100), {"w-a1": live(taken=30, finished=18)})
        self.assertEqual(counts(d), dict(total=100, played=10, skipped=0, finished=18, flight=12))
        self.assertEqual({k: status.ledger_work(d)[k] for k in ("credit", "grain", "hosts")}, dict(credit=0.0, grain=1.0, hosts=["w"]))
        # A runaway the ledger has is no longer counted on the host; an attempt that is over counts nothing more.
        d["runaways"] = {"40": "w-a1"}
        d["attempts"]["w-a1"]["status"]["Runaways"] = 1
        self.assertEqual(status.ledger_work(d)["finished"], 19)
        d["attempts"]["w-a1"]["state"] = "stopped"
        self.assertEqual(counts(d), dict(total=100, played=11, skipped=0, finished=11, flight=0))
        # A replay's subset, and a duel its test closed: only what is to be played.
        d = ledger(6000, 0, {}, only=[[0, 50], [80, 130]])
        self.assertEqual(status.ledger_work(d)["total"], 100)
        d = ledger(6000, 2000, {}, skipped={str(i): "sprt" for i in range(2000, 6000)})
        w = status.ledger_work(d)
        self.assertEqual((w["played"], w["skipped"], w["total"]), (2000, 4000, 6000))
        # A status that counts more than is left (a stale one) never counts past the end.
        d = ledger(20, 15, {}, {"w-a1": live(taken=40, finished=40)})
        self.assertEqual(counts(d), dict(total=20, played=15, skipped=0, finished=20, flight=0))

    def test_the_time_a_ledger_took_is_the_time_any_attempt_ran(self):
        attempts = dict(a=dict(started=0.0, ended=100.0), b=dict(started=50.0, ended=150.0),  # two hosts at once
                        c=dict(started=1000.0, ended=1100.0),  # resumed after a wait
                        d=dict(started=2000.0))  # one whose end is not known
        self.assertEqual(status.ledger_seconds(dict(attempts=attempts)), 250.0)

    def test_settings_are_the_identity_less_what_every_generation_changes(self):
        same = status.pace_settings(identity(FAST, 3, 6000))
        self.assertEqual(status.pace_settings(identity(FAST, 4, 1500)), same, "the model, seed, games and bundles differ")
        self.assertNotEqual(status.pace_settings(identity(DEEP, 4, 1500)), same, "deeper searches are other settings")
        other = {"w": HOSTS["w"], "x": HOSTS["w"]}
        self.assertNotEqual(status.pace_settings(identity(FAST, 4, 1500, other)), same, "another set of hosts")
        slower = {"w": dict(HOSTS["w"], server_args="--precision fp32", precision="fp32")}
        self.assertNotEqual(status.pace_settings(identity(FAST, 4, 1500, slower)), same, "another precision")
        flags = {"w": dict(HOSTS["w"], server_args="--precision fp16 --inputs-tf32")}
        self.assertEqual(status.pace_settings(identity(FAST, 4, 1500, flags)), same, "a serving option's other flags")
        duel = {"global": dict(header=dict(a="neural@70", models=["gen4:x", "gen3:y"], seeds="1..9"), games=6000,
                               server_args="--precision fp16", stops=[]), "hosts": {"w": dict(bundle="b")}}
        other = {"global": dict(header=dict(a="neural@70", models=["gen5:z", "gen4:x"], seeds="10..19"), games=3000,
                                server_args="--precision fp16 --inputs-tf32", stops=[1]), "hosts": {"w": dict(bundle="c")}}
        self.assertEqual(status.pace_settings(duel), status.pace_settings(other))
        deeper = {"global": dict(duel["global"], header=dict(a="neural@128")), "hosts": duel["hosts"]}
        self.assertNotEqual(status.pace_settings(duel), status.pace_settings(deeper))
        self.assertIsNone(status.pace_settings(None))
        self.assertIsNone(status.pace_settings({}))

    def test_the_pace_is_the_earlier_generations_with_these_settings_played_to_the_end(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, FAST, 100, 1000.0)   # 10 s a game
            earlier(root, 2, DEEP, 100, 90000.0)  # other settings
            earlier(root, 3, FAST, 200, 4000.0)   # 20 s a game
            write(root / "gen4" / "dist" / "ledger.json", ledger(100, 40, identity(FAST, 4, 100)))  # not played out
            fast = status.pace_settings(identity(FAST, 9, 5))
            deep = status.pace(root, 5, "dist/ledger.json", status.pace_settings(identity(DEEP, 9, 5)))
            self.assertEqual((deep["seconds_per_game"], deep["generations"], deep["other"]), (900.0, [2], False))
            # A host served another way now than then (another precision) has no rate of its own from then.
            fp32 = {"w": dict(HOSTS["w"], precision="fp32")}
            self.assertEqual(status.pace(root, 5, "dist/ledger.json", status.pace_settings(identity(FAST, 9, 5, fp32)))["rates"], {})
            found = status.pace(root, 5, "dist/ledger.json", fast)
            self.assertEqual((found["seconds_per_game"], found["generations"], found["other"]), (15.0, [3, 1], False))
            self.assertAlmostEqual(found["rates"]["w"], 0.075)  # the median of 0.1 and 0.05 games a second
            self.assertIsNone(status.pace(root, 5, "dist/gate/ledger.json", fast), "another ledger")
            # No generation with these settings: the last that played the ledger to its end at all, and it says so.
            slow = status.pace_settings(identity(dict(FAST, simulations=4096), 9, 5))
            self.assertEqual(status.pace(root, 5, "dist/ledger.json", slow),
                             dict(seconds_per_game=20.0, generations=[3], other=True, rates=dict(w=0.05)))
            self.assertEqual(status.pace(root, 5, "dist/ledger.json", None),
                             dict(seconds_per_game=20.0, generations=[3], other=True, rates=dict(w=0.05)))
            self.assertEqual(status.pace(root, 3, "dist/ledger.json", fast)["generations"], [1], "these settings' first")
            self.assertEqual(status.pace(root, 2, "dist/ledger.json", slow),
                             dict(seconds_per_game=10.0, generations=[1], other=True, rates=dict(w=0.1)))
            self.assertIsNone(status.pace(root, 1, "dist/ledger.json", fast), "no generation before the first")

    def test_a_host_that_joins_the_ledger_later_is_part_of_what_the_pace_is_asked_for(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, FAST, 100, 1000.0, hosts={"w": dict(HOSTS["w"], precision="fp32")})
            run = Run(root, 2, FAST, 200)  # its host served at fp16 now, at fp32 then: no rate of its own from then
            self.assertEqual(run.beat.data["work"]["pace"]["rates"], {})
            write(root / "gen2" / "dist" / "ledger.json",
                  ledger(200, 0, identity(FAST, 2, 200, {"w": dict(HOSTS["w"], precision="fp32")}), {"w-a1": live()}))
            run.beat.beat()
            self.assertEqual(run.beat.data["work"]["pace"], dict(seconds_per_game=10.0, generations=[1], other=False,
                                                                 rates=dict(w=0.1)), "asked again when the hosts change")
            self.assertEqual(run.beat.data["work"]["since"], 10_000.0, "the same work, seen since the same moment")


class ProjectionTest(unittest.TestCase):
    def test_a_stage_at_a_tenth_of_its_pace_is_overdue_by_projection_after_the_window(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, FAST, 100, 1000.0)  # 10 s a game: 360 an hour
            run = Run(root, 2, FAST, 200)
            run.play(1500, per_hour=36)
            early = run.judge()
            self.assertEqual(early["state"], "running", "25 minutes in: the lanes are still growing")
            self.assertNotIn("projection", early)
            run.play(360, per_hour=36)
            j = run.judge()
            self.assertEqual(j["state"], status.PROJECTED)
            p = j["projection"]
            self.assertEqual(p["expected"], 2000.0, "200 games at gen1's 10 s a game")
            self.assertAlmostEqual(p["rate"], 36.0, delta=3.0)
            self.assertGreater(p["ratio"], 8.0)
            self.assertIn("selfplay projected to finish in", j["why"])
            self.assertIn("expected 33 min (at gen1's pace)", j["why"])
            self.assertIn("more than 2 times over", j["why"])
            self.assertEqual(status.EXIT[j["state"]], 6)
            # The limits are the caller's: a wider factor, a longer window, a stage allowed to be slow.
            self.assertEqual(run.judge(overrun_factor=20.0)["state"], "running")
            self.assertEqual(run.judge(overrun_window=7200.0)["state"], "running")
            allowed = run.judge(allow_slow={"selfplay"})
            self.assertEqual(allowed["state"], "running")
            self.assertIn("allowed to be slow", allowed["why"])
            self.assertEqual(run.judge(allow_slow={"all"})["state"], "running")
            self.assertEqual(run.judge(allow_slow={"gate"})["state"], status.PROJECTED)

    def test_a_stage_at_its_pace_is_running_and_says_where_it_is_going(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, FAST, 100, 1000.0)
            run = Run(root, 2, FAST, 2000)
            run.play(2400, per_hour=300, lanes=40)  # a little slower than before: not twice
            j = run.judge()
            self.assertEqual(j["state"], "running")
            self.assertLess(j["projection"]["ratio"], 1.5)
            self.assertNotIn("allowed", j["why"])

    def test_lanes_that_grow_are_projected_by_the_rate_they_reach(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, FAST, 100, 1000.0)
            run = Run(root, 2, FAST, 2000)
            # Forty minutes at a tenth of the pace while the lanes grow, then the pace itself for forty.
            run.play(2400, per_hour=36).play(2400, per_hour=360)
            j = run.judge()
            self.assertEqual(j["state"], "running", j["why"])
            self.assertAlmostEqual(j["projection"]["rate"], 360.0, delta=20.0)

    def test_games_in_flight_count_before_any_has_finished(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, DEEP, 100, 18_000.0)  # 20 games an hour
            run = Run(root, 2, DEEP, 100)
            # 20 lanes take a game each and none ends in the first 35 minutes: half of each counts as done.
            run.play(2100, per_hour=0, lanes=20)
            j = run.judge()
            self.assertEqual(j["state"], "running", j["why"])
            self.assertAlmostEqual(j["projection"]["rate"], 10 * 3600 / 2100, delta=0.5)
            # One lane, and the same: far over.
            slow = Run(root, 3, DEEP, 100)
            slow.play(2100, per_hour=0, lanes=1)
            self.assertNotIn("projection", slow.judge(), "half a game is nothing to take a rate from")
            slow.play(1800 * 9, per_hour=1, lanes=1)
            self.assertEqual(slow.judge()["state"], status.PROJECTED)

    def test_new_settings_are_held_against_the_last_generation_that_played_the_ledger_at_all(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            # A night it happened: 6,000 games of fast searches took 2.7 h a generation; then 1,500 games with
            # full searches sixteen times deeper, on a governor that had backed off to one lane: 10 games an hour.
            earlier(root, 1, FAST, 6000, 9750.0)
            earlier(root, 2, FAST, 6000, 9750.0)
            run = Run(root, 3, DEEP, 1500)
            run.play(1500, per_hour=10, lanes=1)
            self.assertEqual(run.judge()["state"], "running", "inside the window")
            run.play(600, per_hour=10, lanes=1)
            j = run.judge()
            self.assertEqual(j["state"], status.PROJECTED, "held within its first hour, with no generation of its settings")
            p = j["projection"]
            self.assertFalse(p["known"])
            self.assertAlmostEqual(p["expected"], 1500 * 9750.0 / 6000, msg="gen2's seconds a game over these 1,500")
            self.assertGreater(p["total"], 100 * 3600, "more than a hundred hours")
            self.assertRegex(j["why"], r"^selfplay projected to finish in 1\d\d h .*expected 41 min \(at gen2's pace, the "
                                       r"last to play dist/ledger.json: with other settings")
            # An owner's word for what it should take is what it is held against instead; or it is allowed.
            j = run.judge(expect=dict(selfplay=8.0))
            self.assertEqual(j["state"], status.PROJECTED)
            self.assertIn("expected 8.0 h (as --expect says)", j["why"])
            self.assertTrue(j["projection"]["known"])
            self.assertEqual(run.judge(expect=dict(selfplay=100.0))["state"], "running")
            self.assertEqual(run.judge(expect=dict(gate=1.0))["state"], status.PROJECTED, "another stage's")
            self.assertEqual(run.judge(allow_slow={"selfplay"})["state"], "running")
            # Deep searches that go as fast as the fast ones did per game are nothing to hold.
            fine = Run(root, 4, DEEP, 1500)
            fine.play(2400, per_hour=1800, lanes=30)
            j = fine.judge()
            self.assertEqual((j["state"], j["projection"]["known"]), ("running", False))

    def test_a_ledger_no_generation_played_is_held_against_the_stages_usual_time_and_a_first_stage_against_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            # The stage ran before (its record says how long) and left no ledger of this name.
            write(root / "gen1" / "generation.json", dict(stages=dict(selfplay=dict(done=True, seconds=3000.0))))
            run = Run(root, 2, DEEP, 1500)
            self.assertEqual(run.beat.data["expected_seconds"], 3000.0)
            self.assertIsNone(run.beat.data["work"]["pace"])
            run.play(2400, per_hour=10, lanes=1)
            j = run.judge()
            self.assertEqual(j["state"], status.PROJECTED)
            self.assertIn("expected 50 min (the stage's usual time: no earlier generation played dist/ledger.json)", j["why"])
            self.assertFalse(j["projection"]["known"])
            # Nothing ever ran it: told, and held against nothing.
            first = Run(root, 1, DEEP, 1500)
            first.play(2400, per_hour=10, lanes=1)
            j = first.judge()
            self.assertEqual(j["state"], "running")
            self.assertIsNone(j["projection"]["expected"])
            self.assertRegex(j["projection"]["text"], r"^selfplay projected 1\d\d h at the current rate \(")
            self.assertIn("nothing says what it should take", j["projection"]["text"])

    def test_a_resumed_stage_is_projected_over_what_is_left_to_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, FAST, 100, 10_000.0)  # 100 s a game: 36 an hour
            # 1,900 of 2,000 were played before this run of the stage, and the last 100 go faster than the pace:
            # nothing is overdue, and what is expected is the 100's time, not the 2,000's.
            run = Run(root, 2, FAST, 2000)
            run.put(1900)
            self.assertEqual(run.beat.data["work"]["start"], 0.0)
            again = Run(root, 3, FAST, 2000)
            write(root / "gen3" / "dist" / "ledger.json", ledger(2000, 1900, identity(FAST, 3, 2000), {"w-a1": live(1900, 1900)}))
            again.beat.data["work"] = None
            again.beat.beat()
            self.assertEqual(again.beat.data["work"]["start"], 1900.0)
            again.clock[0] += 1800
            again.put(1950, taken=1950, finished=1950)
            j = again.judge()
            self.assertEqual(j["state"], "running", j["why"])
            self.assertEqual(j["projection"]["expected"], 10_000.0, "100 games at 100 s")
            self.assertEqual(j["projection"]["left"], 50.0)

    def test_a_stage_long_past_its_time_with_nothing_to_show_is_overdue(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, FAST, 100, 1000.0)
            run = Run(root, 2, FAST, 200)  # 2,000 s expected
            run.play(2400, per_hour=0, lanes=2)
            j = run.judge()
            self.assertEqual(j["state"], "running", "past its time, and not yet twice past it")
            self.assertAlmostEqual(j["projection"]["ratio"], 1.2)
            run.play(1800, per_hour=0, lanes=2)
            self.assertEqual(run.judge()["state"], "stalled", "nothing at all moves: that is the first thing to say")
            # Its hosts are heard and its log grows (a part granted whole, at work): alive, moving, and far over.
            run.beat.data["progress_at"] = run.clock[0]
            status.write_atomic(run.beat.path, run.beat.data)
            j = run.judge()
            self.assertEqual(j["state"], status.PROJECTED)
            self.assertIn("is about 1 games' worth of time into its 200 games after 70 min, expected 33 min", j["why"])
            self.assertNotIn("finished", j["why"])

    def test_a_ledger_played_out_and_a_stage_without_one_project_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, FAST, 100, 1000.0)
            run = Run(root, 2, FAST, 200)
            run.play(3600, per_hour=36)
            self.assertEqual(run.judge()["state"], status.PROJECTED)
            run.put(200)
            self.assertIsNone(status.read(root, 2)["work"], "played to its end")
            self.assertEqual(run.judge()["state"], "running")
            # The next stage has no ledger: the last one's, written before it began, is not its work.
            run.put(100)
            os.utime(root / "gen2" / "dist" / "ledger.json", (run.clock[0] - 5, run.clock[0] - 5))
            run.clock[0] += 60
            run.beat.begin("train")
            self.assertIsNone(status.read(root, 2)["work"])
            # A duel's ledger under the same stage's name is projected by its own name's history.
            earlier(root, 1, FAST, 100, 500.0, rel="dist/gate/ledger.json")
            gate = Run(root, 3, FAST, 1000, stage="gate", rel="dist/gate/ledger.json")
            gate.play(2400, per_hour=72)
            j = gate.judge()
            self.assertEqual((j["state"], j["projection"]["expected"]), (status.PROJECTED, 5000.0))
            self.assertEqual(status.read(root, 3)["work"]["ledger"], "dist/gate/ledger.json")
            # A status file from before all this, or one half-written, is judged as it always was.
            s = dict(status.read(root, 3), work=dict(stage="gate", done="?"))
            self.assertEqual(status.judge(s, gate.clock[0], alive=lambda pid: True)["state"], "running")

    def test_the_command_says_overdue_projected_with_the_supervisors_limits(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, FAST, 100, 1000.0)
            run = Run(root, 2, FAST, 200, start=__import__("time").time() - 3700)
            run.play(3600, per_hour=36)
            run.beat.data.update(updated=run.clock[0] + 100, progress_at=run.clock[0] + 100)
            status.write_atomic(run.beat.path, run.beat.data)

            def said(*argv) -> tuple[int, str]:
                out = io.StringIO()
                with contextlib.redirect_stdout(out):
                    code = status.main(["--root", str(root), *argv])
                return code, out.getvalue()

            code, text = said()
            self.assertEqual(code, 6)
            self.assertIn("gen2: OVERDUE-PROJECTED - selfplay projected to finish in", text)
            self.assertEqual(said("--allow-slow", "selfplay")[0], 0)
            self.assertEqual(said("--overrun-factor", "50")[0], 0)
            self.assertEqual(said("--expect", "selfplay=100")[0], 0)
            # The supervisor's own limits, from its state, where the command is given none.
            write(root / status.LOOP, dict(state="running", watch=dict(overrun_factor=50.0, overrun_window=1800.0,
                                                                        expect={}, allow_slow=[])))
            code, text = said()
            self.assertEqual(code, 0)
            self.assertIn("projection: selfplay projected to finish in", text)
            self.assertEqual(said("--overrun-factor", "2")[0], 6, "its own wins")
            write(root / status.LOOP, dict(state="running", watch=dict(overrun_factor=2.0, allow_slow=["selfplay"])))
            self.assertEqual(said()[0], 0)
            (root / status.LOOP).unlink()
            (root / status.ALLOW_SLOW).write_text("# tonight's deep run\nselfplay gate  # both\n")
            self.assertEqual(said()[0], 0)
            self.assertEqual(status.allowed_slow(root, ["mine"]), {"selfplay", "gate", "mine"})
            with self.assertRaises(SystemExit):
                status.parse_expect(["selfplay"])
            with self.assertRaises(SystemExit):
                status.parse_expect(["selfplay=0"])
            self.assertEqual(status.parse_expect(["selfplay=8", "gate=1.5"]), dict(selfplay=8.0, gate=1.5))


# A judgement's ledger as generation 14 played it (judge-forks-deep): 941 positions in three parts granted whole to
# the one host, one after the other; an attempt writes no status, and nothing of a part is seen until it is delivered.
JUDGE = {"global": dict(header=dict(seeds="0..941", simulations="8192", worlds="32", seed="1", targets=True),
                        server_args="--precision fp16", model="m", plan="p"), "hosts": {"beta": dict(bundle="b")}}
PARTS = ((0.0, 905.85, 314), (908.47, 1845.55, 314), (1848.12, 2754.89, 313))  # started, ended (seconds in), games


def parts_ledger(t: float, parts=PARTS, ident=JUDGE, host="beta", base: float = 0.0, slow: float = 1.0,
                 kind: str | None = "judge") -> dict:
    """The ledger `t` seconds in: the parts over are accepted, the one at work is a live attempt with no status."""
    total = sum(n for _, _, n in parts)
    attempts, shards, at = {}, {}, 0
    for k, (start, end, n) in enumerate(parts, 1):
        start, end = start * slow, end * slow
        if t >= start:
            over = t >= end
            # (A judgement's attempt never leaves `launching`: only a status moves it to `running`.)
            attempts[f"{host}-a{k}"] = dict(id=f"{host}-a{k}", host=host, state="finished" if over else "launching",
                                            started=base + start, granted_count=n, **(dict(kind=kind) if kind else {}),
                                            **(dict(ended=base + end) if over else {}))
            if over:
                shards[f"s{k}"] = dict(attempt=f"{host}-a{k}", games=n, indices=[[at, at + n]])
        at += n
    return dict(games=total, seed=1, identity=ident, attempts=attempts, shards=shards, only=None, closed=None,
                skipped={}, runaways={}, dropped={}, fails={})


# The gate as generation 13 played it on two hosts at once, and as generation 14 played it on one of them alone.
GATE = {"global": dict(header=dict(a="neural:decklist@70+Worlds=1", b="neural:decklist@70+Worlds=1", decks="3:d",
                                   seeds="1..2", models=["a", "b"]), server_args="--precision fp16", games=6000, seed=1,
                       models="x", stops=[])}
TWO = dict(alpha=(0.0, 2931.70, 2980), beta=(0.007, 2932.97, 3020))


def duel_ledger(hosts: dict, t: float | None = None, base: float = 0.0, games: int = 6000) -> dict:
    """A duel's ledger: each host `(started, ended, games)`; with `t`, as it is `t` seconds in, the hosts at work
    counting the games they have played at their steady rate."""
    attempts, shards, at = {}, {}, 0
    for host, (start, end, n) in hosts.items():
        if t is not None and t < start:
            at += n
            continue  # not launched yet
        over = t is None or t >= end
        played = n if over else int(n * max(0.0, t - start) / (end - start))
        attempts[f"{host}-a1"] = dict(id=f"{host}-a1", host=host, state="finished" if over else "running",
                                      started=base + start, granted_count=n, kind="duel",
                                      **(dict(ended=base + end) if over else {}),
                                      status=dict(Taken=min(n, played + 32), GamesThisRun=played, Dropped=0))
        sent = n if over else played - played % 25  # its results come back 25 games at a time
        if sent:
            shards[f"{host}-s"] = dict(attempt=f"{host}-a1", games=sent, indices=[[at, at + sent]])
        at += n
    return dict(games=games, seed=1, identity=dict(GATE, hosts={h: dict(bundle="b") for h in hosts}), attempts=attempts,
                shards=shards, only=None, closed=None, skipped={}, runaways={}, dropped={}, fails={})


def replay(root: Path, gen: int, stage: str, rel: str, seconds: float, at, step: float = 60.0, **limits) -> list[tuple]:
    """A driver at `stage` on a fake clock, its ledger `rel` written as `at(t, base)` says it is `t` seconds in, a
    beat a `step`: each reading, `(t, state, projection)`."""
    clock = [1_000_000.0]
    beat = status.Heartbeat(root, gen, clock=lambda: clock[0])
    beat.gen_dir.mkdir(parents=True, exist_ok=True)
    beat.begin(stage)
    base, out, t = clock[0], [], 0.0
    while t <= seconds:
        clock[0] = base + t
        write(root / f"gen{gen}" / rel, at(t, base))
        beat.beat()
        beat.data["progress_at"] = clock[0]  # its hosts are heard at every poll
        status.write_atomic(beat.path, beat.data)
        j = status.judge(status.read(root, gen), clock[0], alive=lambda pid: True, **limits)
        out.append((t, j["state"], j.get("projection")))
        t += step
    return out


class WholePartsTest(unittest.TestCase):
    """B1: a ledger of parts granted whole moves only at a delivery; a healthy one was read as twice over."""

    REL = "dist/judge-forks-deep/ledger.json"

    def test_a_judgement_at_its_own_earlier_pace_is_never_overdue(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write(root / "gen1" / self.REL, parts_ledger(1e9))
            read = replay(root, 2, "mine", self.REL, 2750.0, lambda t, base: parts_ledger(t, base=base), step=10.0)
            self.assertEqual({state for _, state, _ in read}, {"running"})
            # Three parts are the whole of it: never three deliveries' worth to take a rate from, so nothing is
            # projected from one (at 30.7 min, two parts' time gone and one delivered, it read 2.0015 times over).
            # All that is ever said is, in its last seconds, that it has spent the time it took before.
            self.assertEqual([p for t, _, p in read if p and t < 2749], [])
            said = [p for _, _, p in read if p]
            self.assertTrue(all(p["rate"] == 0.0 and p["ratio"] < 1.001 for p in said), said)
            w = status.read(root, 2)["work"]
            self.assertAlmostEqual(w["grain"], 941 / 3)
            self.assertEqual(w["hosts"], ["beta"])
            self.assertAlmostEqual(w["pace"]["rates"]["beta"], 941 / (905.85 + 937.08 + 906.77), places=4)

    def test_a_part_at_work_counts_for_its_time_and_never_for_all_of_itself(self):
        d = parts_ledger(1300.0)  # the first part delivered in 905.85 s; the second 391.53 s in
        w = status.ledger_work(d, now=1300.0)
        self.assertAlmostEqual(w["credit"], 314 * 391.53 / 905.85, places=3)
        self.assertEqual((w["played"], w["finished"], w["flight"], w["hosts"]), (314, 314, 0, ["beta"]))
        self.assertEqual(status.ledger_work(d)["credit"], 0.0, "with no clock, only what is delivered")
        # Late: nine tenths of it at most - the rest is its delivery's.
        self.assertAlmostEqual(status.ledger_work(parts_ledger(1844.0), now=1844.0)["credit"], 0.9 * 314)
        self.assertAlmostEqual(status.ledger_work(parts_ledger(1844.0, slow=1.0), now=99_999.0)["credit"], 0.9 * 314)
        # The first part has no part of this ledger to go by: its host's earlier record, where there is one.
        first = parts_ledger(450.0)
        self.assertEqual(status.ledger_work(first, now=450.0)["credit"], 0.0)
        self.assertAlmostEqual(status.ledger_work(first, now=450.0, rates=dict(beta=314 / 900.0))["credit"], 157.0)
        self.assertEqual(status.ledger_work(first, now=450.0, rates=dict(other=1.0))["credit"], 0.0)
        # A host between two parts (the next is granted seconds later) still plays; long after its last, it does not.
        between = parts_ledger(906.0)
        self.assertEqual(status.ledger_work(between)["hosts"], [])
        self.assertEqual(status.ledger_work(between, now=906.0)["hosts"], ["beta"])
        self.assertEqual(status.ledger_work(between, now=906.0 + 301)["hosts"], [])

    def test_a_judgement_at_two_and_a_half_times_its_pace_is_overdue_once_it_has_spent_twice_its_time(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write(root / "gen1" / self.REL, parts_ledger(1e9))
            read = replay(root, 2, "mine", self.REL, 6600.0, lambda t, base: parts_ledger(t, base=base, slow=2.5))
            first = next(t for t, state, _ in read if state == status.PROJECTED)
            self.assertTrue(5500 <= first <= 5580, first)  # twice its 45.8 min, and not at 30 min on one late part
            p = next(p for t, _, p in read if t == first)
            self.assertAlmostEqual(p["ratio"], 2.0, delta=0.03)
            self.assertRegex(p["text"], r"^mine is about \d+ games' worth of time into its 941 games after 1\.5 h, expected 46 min")
            self.assertTrue(p["known"])

    def test_only_a_judgements_part_at_work_or_delivered_is_a_part(self):
        # A duel's attempts before either has written a status, one of them never started: no grain but a game's,
        # and no credit for time, however long they have been there.
        d = duel_ledger(TWO, 600.0)
        for a in d["attempts"].values():
            del a["status"]
            a["state"] = "launching"
        d["attempts"]["alpha-a1"].update(state="failed", why="its launch never started a process")
        w = status.ledger_work(d, now=50_000.0, rates=dict(alpha=1.0, beta=1.0))
        self.assertEqual((w["grain"], w["credit"], w["hosts"]), (1.0, 0.0, ["beta"]))
        # A self-play's ledger from before attempts said their kind: nothing delivered with no status, so no part.
        d["shards"] = {}
        for a in d["attempts"].values():
            del a["kind"]
        self.assertEqual((status.ledger_work(d, now=50_000.0, rates=dict(beta=1.0))["grain"],
                          status.ledger_work(d, now=50_000.0, rates=dict(beta=1.0))["credit"]), (1.0, 0.0))
        # A judgement's: the part at work and the parts delivered; one released unplayed (crashed, nothing sent) is none.
        j = parts_ledger(1300.0)
        j["attempts"]["beta-a9"] = dict(id="beta-a9", host="beta", kind="judge", state="crashed", started=0.0, ended=5.0,
                                        granted_count=900)
        self.assertEqual(status.ledger_work(j, now=1300.0)["grain"], 314.0)
        # And a judgement's ledger from before attempts said their kind is known by its delivered part.
        old = parts_ledger(1300.0, kind=None)
        self.assertAlmostEqual(status.ledger_work(old, now=1300.0)["credit"], 314 * 391.53 / 905.85, places=3)
        self.assertEqual(status.ledger_work(parts_ledger(450.0, kind=None), now=450.0, rates=dict(beta=1.0))["credit"], 0.0,
                         "nothing delivered yet and no kind said: not known to be a part")
        self.assertAlmostEqual(status.ledger_work(parts_ledger(450.0), now=450.0, rates=dict(beta=314 / 900.0))["credit"], 157.0)

    def test_many_parts_are_projected_from_three_parts_worth_and_smoothly(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            twelve = tuple((300.0 * k, 300.0 * k + 298.0, 100) for k in range(12))
            write(root / "gen1" / self.REL, parts_ledger(1e9, twelve))
            read = replay(root, 2, "mine", self.REL, 3500.0, lambda t, base: parts_ledger(t, twelve, base=base))
            ratios = [p["ratio"] for _, _, p in read if p]
            self.assertGreater(len(ratios), 20)
            self.assertTrue(0.9 < min(ratios) and max(ratios) < 1.15, (min(ratios), max(ratios)))
            self.assertEqual({state for _, state, _ in read}, {"running"})
            # Three times as slow: seen as soon as three parts' worth of work is behind it.
            slow = replay(root, 3, "mine", self.REL, 4200.0, lambda t, base: parts_ledger(t, twelve, base=base, slow=3.0))
            first = next(t for t, state, _ in slow if state == status.PROJECTED)
            self.assertTrue(1800 <= first <= 3000, first)
            self.assertAlmostEqual(next(p for t, _, p in slow if t == first)["ratio"], 3.0, delta=0.4)


class HostsTest(unittest.TestCase):
    """S1: what is expected is what the hosts playing now made before, not what another set of hosts made together."""

    REL = "dist/gate/ledger.json"

    def test_one_host_of_two_is_held_to_what_that_host_made(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write(root / "gen1" / self.REL, duel_ledger(TWO))
            # Generation 14's gate: beta alone, 6,000 games in 95.2 min; generation 13's two hosts took 48.9.
            alone = dict(beta=(0.0, 5710.75, 6000))
            read = replay(root, 2, "gate", self.REL, 5700.0, lambda t, base: duel_ledger(alone, t, base))
            self.assertEqual({state for _, state, _ in read}, {"running"})
            ratios = [p["ratio"] for _, _, p in read if p]
            self.assertTrue(0.95 < min(ratios) and max(ratios) < 1.02, (min(ratios), max(ratios)))  # it read 1.99 to 2.05
            p = read[-1][2]
            self.assertAlmostEqual(p["expected"], 6000 / (3020 / 2932.963), delta=40.0)  # less the games in flight at first sight
            self.assertEqual((p["known"], p["wide"]), (True, False))
            self.assertIn("(at gen1's pace on beta)", p["text"])
            # The two together, as before: what the two made.
            both = replay(root, 3, "gate", self.REL, 2900.0, lambda t, base: duel_ledger(TWO, t, base))
            ratios = [p["ratio"] for _, _, p in both if p]
            self.assertTrue(0.97 < min(ratios) and max(ratios) < 1.03, (min(ratios), max(ratios)))
            self.assertIn("(at gen1's pace)", both[-1][2]["text"])
            # beta alone at 40% of what it made: held.
            slow = dict(beta=(0.0, 5710.75 * 2.5, 6000))
            read = replay(root, 4, "gate", self.REL, 3000.0, lambda t, base: duel_ledger(slow, t, base))
            self.assertEqual(read[-1][1], status.PROJECTED)
            self.assertAlmostEqual(read[-1][2]["ratio"], 2.45, delta=0.1)

    def test_a_host_that_joins_hours_late_is_no_overrun(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            # Before: the two together, alpha at 0.2066 games a second and beta at 0.326: 6,000 games in 188 min.
            write(root / "gen1" / "dist" / "ledger.json", duel_ledger(dict(alpha=(0.0, 11265.0, 2327), beta=(0.0, 11265.0, 3673))))
            # Now alpha alone for three hours, beta from then on: 298 min in all, 1.6 times the two's time - and it read
            # 2.35 ten minutes after beta joined (all 6,000 games over the two's rate, against alpha's three hours).
            late = dict(alpha=(0.0, 17877.0, 3693), beta=(10800.0, 17877.0, 2307))
            read = replay(root, 2, "selfplay", "dist/ledger.json", 17800.0, lambda t, base: duel_ledger(late, t, base))
            self.assertEqual({state for _, state, _ in read}, {"running"})
            alone = [p["ratio"] for t, _, p in read if p and t < 10800]
            self.assertTrue(0.97 < min(alone) and max(alone) < 1.03, (min(alone), max(alone)))  # alpha at alpha's own rate
            self.assertIn("(at gen1's pace on alpha)", next(p for t, _, p in read if p)["text"])
            # The watch begins again when beta joins: nothing is said for a window, then what is left at the two's rate.
            self.assertEqual([t for t, _, p in read if p and 10800 <= t < 10800 + 1800], [])
            joined = [p for t, _, p in read if p and t >= 10800]
            ratios = [p["ratio"] for p in joined]
            self.assertTrue(0.97 < min(ratios) and max(ratios) < 1.03, (min(ratios), max(ratios)))
            self.assertAlmostEqual(joined[0]["expected"], (6000 - 0.2066 * 10800) / (0.2066 + 0.326), delta=120.0)
            self.assertIn("(at gen1's pace)", joined[0]["text"])
            work = status.read(root, 2)["work"]
            self.assertEqual(work["hosts"], ["alpha", "beta"])
            self.assertAlmostEqual(work["since"], 1_000_000.0 + 10800, delta=61.0)
            # A host lost for good: after the gap, what is left is held to the one that plays.
            lost = dict(alpha=(0.0, 20000.0, 4132), beta=(0.0, 3000.0, 978))
            read = replay(root, 3, "selfplay", "dist/ledger.json", 9000.0, lambda t, base: duel_ledger(lost, t, base, games=5110))
            self.assertEqual({state for _, state, _ in read}, {"running"})
            self.assertIn("(at gen1's pace on alpha)", read[-1][2]["text"])
            self.assertAlmostEqual(read[-1][2]["ratio"], 1.0, delta=0.05)

    def test_a_host_with_no_record_widens_the_limit_and_is_not_guessed_at(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write(root / "gen1" / self.REL, duel_ledger(TWO))
            # A new host alone, at 44% of what the two made together: 2.3 times their time, and inside the wider limit.
            new = dict(x=(0.0, 6700.0, 6000))
            read = replay(root, 2, "gate", self.REL, 3000.0, lambda t, base: duel_ledger(new, t, base))
            self.assertEqual({state for _, state, _ in read}, {"running"})
            p = read[-1][2]
            self.assertAlmostEqual(p["ratio"], 2.28, delta=0.05)
            self.assertTrue(p["wide"])
            self.assertIn("made by other hosts: the limit is wider", p["text"])
            self.assertEqual(status.held_to(p, 2.0), 3.0)
            # At a quarter: past three times.
            read = replay(root, 3, "gate", self.REL, 3000.0, lambda t, base: duel_ledger(dict(x=(0.0, 12000.0, 6000)), t, base))
            self.assertEqual(read[-1][1], status.PROJECTED)
            self.assertIn("more than 3 times over", status.judge(status.read(root, 3), 1_000_000.0 + 3000, alive=lambda pid: True)["why"])
            # A new host beside one with a record: what the known one made is expected, more widely.
            pair = dict(beta=(0.0, 4000.0, 4000), x=(0.0, 4000.0, 2000))
            read = replay(root, 4, "gate", self.REL, 3000.0, lambda t, base: duel_ledger(pair, t, base))
            p = read[-1][2]
            self.assertEqual((read[-1][1], p["wide"]), ("running", True))
            self.assertAlmostEqual(p["expected"], 6000 / (3020 / 2932.963), delta=40.0)  # less the games in flight at first sight
            self.assertIn("on beta; x played no part of it then", p["text"])


class WindowTest(unittest.TestCase):
    """S2: once there is a whole window, its rate decides."""

    def test_a_collapse_mid_stage_is_seen_within_about_a_window(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, FAST, 100, 1000.0)  # 360 an hour
            run = Run(root, 2, FAST, 6000)
            run.play(7200, per_hour=360, lanes=30)  # two hours at its pace
            self.assertLess(run.judge()["projection"]["ratio"], 1.1)
            seen = None
            for minutes in range(1, 61):  # then a twentieth of it
                run.play(60, per_hour=18, lanes=30)
                if seen is None and run.judge()["state"] == status.PROJECTED:
                    seen = minutes
            # The whole run's rate would still read 245 an hour an hour later (1.4 times over, and for hours yet).
            self.assertTrue(10 <= seen <= 35, seen)
            self.assertAlmostEqual(run.judge()["projection"]["rate"], 18.0, delta=2.0)

    def test_a_window_with_too_little_in_it_reaches_back_for_three_games(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            earlier(root, 1, DEEP, 100, 360_000.0)  # an hour a game
            run = Run(root, 2, DEEP, 100)
            run.play(4 * 3600, per_hour=1)  # a game an hour, as before: no window of half an hour holds three
            j = run.judge()
            self.assertEqual(j["state"], "running", j["why"])
            self.assertAlmostEqual(j["projection"]["rate"], 1.0, delta=0.3)  # three or four games, by the moment
            self.assertAlmostEqual(j["projection"]["ratio"], 1.0, delta=0.4)


def notifications(root: Path) -> list[dict]:
    path = root / notify.LOG
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines()] if path.exists() else []


# A driver at its self-play, a tenth as fast as the generation before: it plays on until it is asked to stop
# (`selfplay.stop`, as `generation.py --stop` writes it), and then ends `stopped`, as the driver does.
SLOW_DRIVER = """
import json, os, sys, time
root = sys.argv[1]
if '--preflight' in sys.argv:
    sys.exit(0)
gen = os.path.join(root, 'gen1')
os.makedirs(gen, exist_ok=True)
now = time.time()
work = json.load(open(os.path.join(root, 'work.json')))
work.update(since=now - 3600, at=now, samples=[[now - 3600, 0.0], [now, work['done']]])
s = dict(generation=1, state='running', stage='selfplay', pid=os.getpid(), host=__import__('socket').gethostname(),
         stage_started=now - 3660, progress_at=now, updated=now, expected_seconds=None, work=work)
def put():
    tmp = os.path.join(gen, 'status.json.tmp')
    with open(tmp, 'w') as f:
        json.dump(s, f)
    while True:
        try:  # a reader that has the file open this instant refuses the rename on Windows
            return os.replace(tmp, os.path.join(gen, 'status.json'))
        except PermissionError:
            time.sleep(0.01)
put()
deadline = now + 60
while time.time() < deadline and not os.path.exists(os.path.join(gen, 'selfplay', 'selfplay.stop')):
    time.sleep(0.02)
    s.update(updated=time.time(), progress_at=time.time())
    put()
stopped = os.path.exists(os.path.join(gen, 'selfplay', 'selfplay.stop'))
s.update(state='stopped' if stopped else 'failed', error='self-play stopped by its governor (stop file)' if stopped else 'nobody stopped it')
put()
sys.exit(1)
"""


class SupervisorTest(unittest.TestCase):
    def loop(self, root: Path, **options) -> loop.Loop:
        args = SimpleNamespace(**{**dict(poll=0.05, retries=2, retry_wait=0.0, stall_factor=0.25, stall_floor=900.0,
                                         renotify=3600.0, no_preflight=False, update=None, preflight_timeout=60.0,
                                         overrun_factor=2.0, overrun_window=1800.0, expect=[], allow_slow=[],
                                         overrun_persist=0.0, overrun_stop_timeout=1200.0), **options})
        made = loop.Loop(root, args, [])
        (root / "driver.py").write_text(SLOW_DRIVER)
        made.driver = lambda *extra: [sys.executable, str(root / "driver.py"), str(root), *extra]
        made.log = lambda text: None
        made.strangers = lambda: []
        return made

    def status(self, root: Path, now: float, done: float = 36.0, total: int = 1500, pace: dict | None = None, **more) -> dict:
        """Generation 1 an hour into a self-play that has `done` of `total` games."""
        work = dict(stage="selfplay", ledger="dist/ledger.json", since=now - 3600, start=0.0, total=total, done=done,
                    at=now, samples=[[now - 3600, 0.0], [now, done]], pace=pace)
        s = dict(generation=1, state="running", stage="selfplay", pid=os.getpid(), host=socket.gethostname(),
                 stage_started=now - 3660, expected_seconds=None, progress_at=now, updated=now, work=work, **more)
        write(root / "gen1" / status.STATUS, s)
        return s

    def test_a_projected_overrun_is_told_once_and_the_stage_is_asked_to_stop(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root)
            clock = [50_000.0]
            made.clock = lambda: clock[0]
            self.status(root, clock[0], pace=dict(seconds_per_game=6.5, generations=[14, 13]))
            told: dict = {}
            made.watch(1, lambda: True, told)
            made.watch(1, lambda: True, told)
            sent = notifications(root)
            self.assertEqual([n["event"] for n in sent], ["overrun"], "told once")
            self.assertRegex(sent[0]["message"], r"gen1: selfplay projected to finish in 4\d\.\d h \(4\d\.\d h in all: 1464 of 1500 "
                                                 r"games left at 36\.0 an hour\), expected 2\.7 h \(at gen14, gen13's pace\)")
            self.assertEqual(sent[0]["stage"], "selfplay")
            self.assertGreater(sent[0]["details"]["projection"]["ratio"], 10)
            # Asked to stop as `generation.py --stop` asks: gracefully, the games in flight finishing.
            self.assertEqual((root / "gen1" / "selfplay" / "selfplay.stop").read_text(), "")
            self.assertEqual((root / "gen1" / "dist" / "stop").read_text(), "")
            self.assertEqual(told["overrun"]["stage"], "selfplay")

    def test_one_look_stops_nothing_the_reading_has_to_last(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, overrun_persist=600.0)
            clock = [50_000.0]
            made.clock = lambda: clock[0]
            pace = dict(seconds_per_game=6.5, generations=[14, 13])
            told: dict = {}

            def look(seconds: float, done: float):
                clock[0] += seconds
                s = self.status(root, clock[0], done=done, pace=pace)
                s["work"]["since"] = 46_000.0  # one watch of the ledger throughout
                write(root / "gen1" / status.STATUS, s)
                made.watch(1, lambda: True, told)

            stop = root / "gen1" / "selfplay" / "selfplay.stop"
            look(0, 36.0)       # far over
            look(300, 36.0)     # and five minutes later
            self.assertEqual(notifications(root), [])
            self.assertFalse(stop.exists())
            look(30, 1400.0)    # a late delivery: not over at this look, and the count begins again
            look(30, 36.0)
            look(570, 36.0)     # nine and a half minutes over: not yet
            self.assertEqual(notifications(root), [])
            look(30, 36.0)      # ten
            sent = notifications(root)
            self.assertEqual([n["event"] for n in sent], ["overrun"])
            self.assertIn("more than 2 times over, at every look for 10 min. Its work is being stopped", sent[0]["message"])
            self.assertIn("if its driver has not ended in 20 min its games in flight are stopped too", sent[0]["message"])
            self.assertEqual(stop.read_text(), "")

    def test_other_hosts_playing_begin_the_count_again_and_a_stop_asked_is_not_taken_back(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, overrun_persist=600.0)
            clock = [50_000.0]
            made.clock = lambda: clock[0]
            pace = dict(seconds_per_game=6.5, generations=[14])
            told: dict = {}
            stop = root / "gen1" / "selfplay" / "selfplay.stop"

            def look(seconds: float, since: float, done: float = 36.0):
                clock[0] += seconds
                s = self.status(root, clock[0], done=done, pace=pace)
                s["work"]["since"] = since
                write(root / "gen1" / status.STATUS, s)
                made.watch(1, lambda: True, told)

            look(0, 40_000.0)
            look(590, 40_000.0)      # over for 9 min 50 s of one watch
            look(20, 45_000.0)       # the ledger is watched anew (a host joined): still read as over, and counted from here
            look(590, 45_000.0)
            self.assertEqual(notifications(root), [])
            look(20, 45_000.0)       # ten minutes of this watch
            self.assertEqual([n["event"] for n in notifications(root)], ["overrun"])
            self.assertTrue(stop.exists())
            # The reading recovers while the stage stops: the stop stands, and nothing more is said.
            look(60, 45_000.0, done=1400.0)
            look(60, 45_000.0, done=1450.0)
            self.assertEqual([n["event"] for n in notifications(root)], ["overrun"])
            self.assertTrue(stop.exists())
            self.assertIn("overrun", told)

    def test_a_stage_that_does_not_stop_has_its_games_in_flight_stopped_after_the_timeout(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, overrun_stop_timeout=1200.0)
            clock = [50_000.0]
            made.clock = lambda: clock[0]
            pace = dict(seconds_per_game=6.5, generations=[14])
            told: dict = {}
            stop, whole = root / "gen1" / "selfplay" / "selfplay.stop", root / "gen1" / "dist" / "stop"
            for seconds in (0, 600, 590):
                clock[0] += seconds
                self.status(root, clock[0], pace=pace)
                made.watch(1, lambda: True, told)
            self.assertEqual([n["event"] for n in notifications(root)], ["overrun"])
            self.assertEqual((stop.read_text(), whole.read_text()), ("", ""), "19 min 50 s: still the graceful stop")
            for _ in range(3):
                clock[0] += 10
                self.status(root, clock[0], pace=pace)
                made.watch(1, lambda: True, told)
            sent = notifications(root)
            self.assertEqual([n["event"] for n in sent], ["overrun", "overrun"], "the escalation is told once")
            self.assertIn("was asked to stop 20 min ago and its driver has not ended: its games in flight are stopped too "
                          "(as --stop-now does", sent[1]["message"])
            self.assertEqual((stop.read_text().strip(), whole.read_text().strip()), ("now", "now"))

    def test_a_stop_is_left_only_where_the_running_stage_reads_it(self):
        from cantrip_ai import generation  # noqa: PLC0415

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            self.status(root, 50_000.0)
            write(root / "gen1" / status.STATUS, dict(status.read(root, 1), stage="gate"))
            said: list[str] = []
            path = generation.request_stop(root, 1, now=False, log=said.append)
            self.assertEqual(path, root / "gen1" / "dist" / "stop")
            self.assertEqual(path.read_text(), "")
            self.assertFalse((root / "gen1" / "selfplay").exists(), "nothing of the gate reads a stop there")
            self.assertIn("asked gen1's gate to stop", said[0])
            # A self-play: its local run's file and the whole distributed run's.
            write(root / "gen1" / status.STATUS, dict(status.read(root, 1), stage="selfplay"))
            path = generation.request_stop(root, 1, now=True, log=said.append)
            self.assertEqual(path, root / "gen1" / "selfplay" / "selfplay.stop")
            self.assertEqual((path.read_text().strip(), (root / "gen1" / "dist" / "stop").read_text().strip()), ("now", "now"))
            # No driver's status to go by: both, as ever.
            generation.request_stop(root, 2, now=False, log=said.append)
            self.assertTrue((root / "gen2" / "selfplay" / "selfplay.stop").exists() and (root / "gen2" / "dist" / "stop").exists())

    def test_a_stage_allowed_to_be_slow_is_not_stopped(self):
        for options, file in ((dict(allow_slow=["selfplay"]), None), ({}, "selfplay\n"), ({}, "all\n"),
                              (dict(overrun_factor=100.0), None), (dict(expect=["selfplay=40"]), None)):
            with tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                made = self.loop(root, **options)
                made.clock = lambda: 50_000.0
                if file:
                    (root / status.ALLOW_SLOW).write_text(file)
                self.status(root, 50_000.0, pace=dict(seconds_per_game=6.5, generations=[14]))
                told: dict = {}
                made.watch(1, lambda: True, told)
                self.assertEqual(notifications(root), [], (options, file))
                self.assertFalse((root / "gen1" / "selfplay").exists())
                self.assertNotIn("overrun", told)

    def test_other_settings_are_told_once_and_held_as_well(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root)
            clock = [50_000.0]
            made.clock = lambda: clock[0]
            told: dict = {}
            other = dict(seconds_per_game=1.9, generations=[14], other=True)
            for _ in range(3):
                clock[0] += 60
                self.status(root, clock[0], done=10.0, pace=other)
                made.watch(1, lambda: True, told)
            sent = notifications(root)
            self.assertEqual([n["event"] for n in sent], ["projected", "overrun"], "each once")
            for n in sent:
                self.assertRegex(n["message"], r"selfplay projected to finish in 149 h .*expected 48 min \(at gen14's pace, "
                                               r"the last to play dist/ledger.json: with other settings")
            self.assertEqual((root / "gen1" / "selfplay" / "selfplay.stop").read_text(), "")
            # Allowed to be slow: still told where it is going, and not stopped.
            (root / "gen1" / "selfplay" / "selfplay.stop").unlink()
            (root / status.ALLOW_SLOW).write_text("selfplay\n")
            told = {}
            made.watch(1, lambda: True, told)
            self.assertEqual([n["event"] for n in notifications(root)], ["projected", "overrun", "projected"])
            self.assertFalse((root / "gen1" / "selfplay" / "selfplay.stop").exists())

    def test_a_list_of_events_never_keeps_an_overrun_or_a_projection_from_being_sent(self):
        import unittest.mock  # noqa: PLC0415

        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            got = root / "got.txt"
            hook = f'"{sys.executable}" -c "import os; open(r\'{got}\', \'a\').write(os.environ[\'CANTRIP_AI_EVENT\'] + chr(10))"'
            config = root / "notify.json"
            config.write_text(json.dumps(dict(hook=hook, events=["stage-failed", "dead"])))
            env = {k: v for k, v in os.environ.items() if k not in (notify.ENV_HOOK, notify.ENV_WEBHOOK)}
            with unittest.mock.patch.dict(os.environ, {**env, notify.ENV_FILE: str(config)}, clear=True):
                sent = {e: notify.send(e, "m", root=root) for e in ("warning", "held", "dead", "overrun", "projected")}
            self.assertEqual(got.read_text().split(), ["dead", "overrun", "projected"])
            self.assertEqual(sent["warning"], {}, "not in the list: logged only")
            self.assertEqual(sent["overrun"], dict(hook="ok"))
            self.assertEqual(len(notifications(root)), 5, "every one is in the log")

    def test_with_nothing_to_expect_the_projection_is_told_once_and_nothing_is_stopped(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root)
            clock = [50_000.0]
            made.clock = lambda: clock[0]
            told: dict = {}
            # Before the window: nothing.
            s = self.status(root, clock[0], done=10.0)
            s["work"]["since"] = clock[0] - 1200
            write(root / "gen1" / status.STATUS, s)
            made.watch(1, lambda: True, told)
            self.assertEqual(notifications(root), [])
            for _ in range(3):
                clock[0] += 600
                self.status(root, clock[0], done=10.0)
                made.watch(1, lambda: True, told)
            sent = notifications(root)
            self.assertEqual([n["event"] for n in sent], ["projected"], "once")
            self.assertRegex(sent[0]["message"], r"gen1: selfplay projected 150 h at the current rate")
            self.assertFalse((root / "gen1" / "selfplay").exists())
            # An owner's word, and it is held against it.
            made.args.expect = ["selfplay=8"]
            made.watch(1, lambda: True, told)
            self.assertEqual([n["event"] for n in notifications(root)], ["projected", "overrun"])
            self.assertIn("overrun", notify.EVENTS)
            self.assertIn("projected", notify.EVENTS)

    def test_the_supervisor_stops_the_stage_and_holds_the_loop_with_the_projection(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root)
            write(root / "work.json", dict(stage="selfplay", ledger="dist/ledger.json", start=0.0, total=1500, done=36.0,
                                            pace=dict(seconds_per_game=6.5, generations=[14, 13])))
            self.assertEqual(made.run(), loop.HELD)
            held = (root / loop.PAUSE).read_text()
            self.assertRegex(held, r"held .*: gen1: selfplay projected to finish in 4\d\.\d h .*expected 2\.7 h")
            self.assertIn("Its work was stopped (what was played is kept) and resumes when this file is removed", held)
            self.assertIn(f"`selfplay` in {root / status.ALLOW_SLOW}", held)
            self.assertEqual([n["event"] for n in notifications(root)], ["overrun", "held"])
            state = status.read_json(root / status.LOOP)
            self.assertEqual(state["state"], "held")
            self.assertEqual(state["watch"], dict(overrun_factor=2.0, overrun_window=1800.0, expect={}, allow_slow=[]))
            self.assertEqual(status.read(root, 1)["state"], "stopped", "the driver ended as --stop ends it: resumable")

    def test_a_stop_somebody_else_asked_for_is_still_theirs(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            made = self.loop(root, allow_slow=["selfplay"])
            write(root / "work.json", dict(stage="selfplay", ledger="dist/ledger.json", start=0.0, total=1500, done=36.0,
                                            pace=dict(seconds_per_game=6.5, generations=[14])))
            (root / "gen1" / "selfplay").mkdir(parents=True)
            (root / "gen1" / "selfplay" / "selfplay.stop").write_text("")
            self.assertEqual(made.run(), loop.HELD)
            self.assertEqual([n["event"] for n in notifications(root)], ["stopped"])
            self.assertIn("was stopped at stage selfplay, as asked", (root / loop.PAUSE).read_text())

    def test_the_flags_are_the_supervisors(self):
        args = loop.parser().parse_args(["--root", "x"])
        self.assertEqual((args.overrun_factor, args.overrun_window, args.expect, args.allow_slow), (2.0, 1800.0, [], []))
        self.assertEqual((args.overrun_persist, args.overrun_stop_timeout), (600.0, 1200.0))
        # A window longer than the samples kept would never have a rate to take; none at all is no window.
        for bad in (0.0, -5.0, 10_801.0):
            with self.assertRaises(SystemExit):
                status.check_window(bad)
        status.check_window(10_800.0)
        status.check_window(None)
        with self.assertRaises(SystemExit), contextlib.redirect_stderr(io.StringIO()):
            loop.main(["--root", "x", "--overrun-window", "20000"])
        with self.assertRaises(SystemExit):
            status.main(["--root", "x", "--overrun-window", "0"])
        args = loop.parser().parse_args(["--root", "x", "--overrun-factor", "3", "--overrun-window", "900", "--expect",
                                         "selfplay=8", "--allow-slow", "gate", "--allow-slow", "mine"])
        made = loop.Loop(Path("x"), args, [])
        self.assertEqual(made.limits(), dict(overrun_factor=3.0, overrun_window=900.0, expect=dict(selfplay=8.0),
                                             allow_slow=["gate", "mine"]))


if __name__ == "__main__":
    unittest.main()
