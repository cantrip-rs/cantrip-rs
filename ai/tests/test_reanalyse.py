"""The deep reanalysis (`reanalyse.py`, the driver's `reanalyse` stage): where its positions come from, its plan, its
lanes, its time budget, the corpus it leaves and how training and the evaluation read it. Nothing here runs the
engine, leases a machine, serves a network or waits on time: the engine's commands and the miner's distribution are
stand-ins that write what the real ones write."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
import unittest.mock
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import quiet  # noqa: E402,F401
from cantrip_ai import dataset, distributed, duels, generation, mining, reanalyse, status  # noqa: E402
from test_dataset import sample, write_shard  # noqa: E402
from test_hardening import BASE, promoted_root, quietly, started  # noqa: E402
from test_mining import FakeJudgeHost  # noqa: E402

GIB = 1 << 30


def recordings(directory: Path, seeds, shard_games: int = 2, writer: str = "selfplay-l0-11") -> dict[int, dict]:
    """A recorded directory of `seeds`: shards of `shard_games` games (stand-ins: what they hold is returned, as
    `reanalyse.recorded_games` reads it) and a trace line a game."""
    (directory / "traces").mkdir(parents=True, exist_ok=True)
    games = {}
    for k, seed in enumerate(seeds):
        name = f"{writer}-170000-{k // shard_games:05d}.shard.zst"
        (directory / name).write_text("shard")
        games[seed] = dict(shard=name, turns=8 + k % 5, decks=["red", "blue"])
    (directory / "traces" / f"{writer}.jsonl").write_text("".join(json.dumps(dict(seed=s, steps=[1])) + "\n" for s in seeds))
    return games


def target(agree: bool = True, **more) -> dict:
    """A judged position's `target`, as `mine::against_prior` writes it."""
    return dict(dict(kl=0.05 if agree else 0.9, top_agree=agree, simulations=64, tree_bytes=1 << 20, memory_bound=False,
                     target=[dict(token="a", target=0.7, prior=0.6)]), **more)


def a_setup(base: Path, workers=(), local: bool = True) -> duels.Setup:
    return duels.Setup(root=base, gen=7, gen_dir=base / "gen7", repo=base, workers=list(workers), local=local,
                       local_name="here", botlab_local=[], botlab={}, native=None, record_native=None, ai_host=None,
                       infer=None, ort_dir=None, cuda_dir=None, cuda_dlls=(), server_args=[], local_parallel=0,
                       worker_parallel=0, log=lambda *_: None)


class Sources(unittest.TestCase):
    def test_self_play_with_traces_first_then_the_gates_recordings(self):
        with tempfile.TemporaryDirectory() as tmp:
            gen = Path(tmp) / "gen7"
            with self.assertRaises(SystemExit) as caught:
                reanalyse.sources(gen, "auto")
            self.assertIn("--selfplay-traces", str(caught.exception))
            for half in ("first", "second"):
                recordings(gen / "mine" / "recorded" / half, [1, 2])
            (gen / "selfplay").mkdir()
            (gen / "selfplay" / "a.shard.zst").write_text("x")  # self-play played without traces
            self.assertEqual(reanalyse.sources(gen, "auto"), ("gate", [("first", gen / "mine" / "recorded" / "first"),
                                                                       ("second", gen / "mine" / "recorded" / "second")]))
            with self.assertRaises(SystemExit):
                reanalyse.sources(gen, "selfplay")
            recordings(gen / "selfplay", [5, 6])
            self.assertEqual(reanalyse.sources(gen, "auto"), ("selfplay", [("selfplay", gen / "selfplay")]))
            self.assertEqual(reanalyse.sources(gen, "gate")[0], "gate")

    def test_a_game_needs_its_shard_and_its_trace(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp) / "selfplay"
            games = recordings(d, [10, 11, 12, 13])
            games.pop(11)                       # moved to the held-out set: its trace stays, its shard is gone
            (d / "traces" / "selfplay-l0-11.jsonl").write_text('{"seed":10,"steps":[]}\n{ "seed" : 11}\n{"seed":13}\n')
            listed = reanalyse.list_games([("selfplay", d)], scan=lambda _: games)
            self.assertEqual([g["seed"] for g in listed], [10, 13], "12 has no trace, 11 no shard")
            self.assertEqual(reanalyse.traced(d), {10, 11, 13})

    def test_a_trace_line_cut_short_is_skipped_and_said_and_loses_no_other_game(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "selfplay"
            games = recordings(src, [40, 41, 42])
            # The run was killed while it wrote game 42's line; and a line in the middle that is no JSON.
            (src / "traces" / "selfplay-l0-11.jsonl").write_text(
                '{"seed":40,"steps":[1]}\n{"seed":41,"steps":[1}}\n{"seed":42,"steps":[1,2')
            self.assertEqual(reanalyse.traced(src), {40, 41}, "a line cut short is no trace")
            said = []
            wanted = [dict(dir="selfplay", seed=k, **games[k]) for k in (40, 41, 42)]
            reanalyse.materialise(Path(tmp) / "data", [("selfplay", src)], wanted, log=said.append)
            kept = (Path(tmp) / "data" / "recorded" / "selfplay" / "traces" / "games.jsonl").read_text().splitlines()
            self.assertEqual([json.loads(x)["seed"] for x in kept], [40], "every line the judge is given parses")
            self.assertEqual(len(said), 2)
            self.assertTrue(all("WARNING" in line and "skipped" in line for line in said))

    def test_the_order_is_random_and_the_same_every_time(self):
        games = [dict(dir="selfplay", seed=s) for s in range(50)]
        a = reanalyse.ordered(games, 5)
        self.assertEqual(a, reanalyse.ordered(list(reversed(games)), 5))
        self.assertNotEqual(a, reanalyse.ordered(games, 6))
        self.assertNotEqual([g["seed"] for g in a], sorted(g["seed"] for g in a))
        self.assertEqual(sorted(g["seed"] for g in a), list(range(50)))

    def test_only_the_recordings_of_the_games_asked_for_are_laid_out(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "selfplay"
            games = recordings(src, range(20, 30))
            wanted = [dict(dir="selfplay", seed=s, **games[s]) for s in (21, 26)]
            dest = Path(tmp) / "data"
            reanalyse.materialise(dest, [("selfplay", src)], wanted)
            out = dest / "recorded" / "selfplay"
            self.assertEqual(sorted(p.name for p in out.glob("*.shard.zst")), sorted({games[21]["shard"], games[26]["shard"]}))
            lines = [json.loads(x) for x in (out / "traces" / "games.jsonl").read_text().splitlines()]
            self.assertEqual([x["seed"] for x in lines], [21, 26])
            # Laid out as a worker's bundle's data: every file by its place there.
            files = reanalyse.Data(dest, Path(tmp) / "judge").files()
            self.assertIn("data/recorded/selfplay/traces/games.jsonl", files)
            self.assertEqual(len(files), 3)

    def test_a_game_played_twice_keeps_the_trace_of_the_process_that_wrote_its_shard(self):
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "selfplay"
            (src / "traces").mkdir(parents=True)
            # Game 7 was played by a worker's first attempt (lost) and again by its second, whose shard was accepted.
            (src / "w-selfplay-l3-222-170000-00000.shard.zst").write_text("x")
            (src / "traces" / "w-a1-selfplay-l3-111.jsonl").write_text('{"seed":7,"by":"the lost attempt"}\n')
            (src / "traces" / "w-a2-selfplay-l3-222.jsonl").write_text('{"seed":7,"by":"the one in the shard"}\n')
            (src / "traces" / "w-a3-selfplay-l9-333.jsonl").write_text('{"seed":7,"by":"another, later"}\n')
            game = dict(dir="selfplay", seed=7, shard="w-selfplay-l3-222-170000-00000.shard.zst")
            reanalyse.materialise(Path(tmp) / "data", [("selfplay", src)], [game])
            kept = (Path(tmp) / "data" / "recorded" / "selfplay" / "traces" / "games.jsonl").read_text()
            self.assertEqual([json.loads(x)["by"] for x in kept.splitlines()], ["the one in the shard"])


class Plan(unittest.TestCase):
    def setUp(self):
        self.games = [dict(dir="selfplay", seed=100 + k, decks=["red", "blue"]) for k in range(40)]
        # Seat 0 decides at the even decisions, seat 1 at the odd ones; a decision's turn is its number.
        self.decisions = {("selfplay", g["seed"]): [[d, d % 2, d] for d in range(12)] for g in self.games}

    def test_a_few_decisions_of_each_game_in_the_games_order_and_no_more_than_asked(self):
        plan = reanalyse.plan_positions(self.games, self.decisions, 2, 30, 0.0, 5)
        self.assertEqual(len(plan), 30)
        self.assertEqual([x["seed"] for x in plan], [100 + k // 2 for k in range(30)], "two a game, the games in order")
        self.assertEqual(len({x["id"] for x in plan}), 30)
        for x in plan:
            self.assertEqual((x["seat"], x["turn"]), (x["decision"] % 2, x["decision"]))
        self.assertEqual({x["split"] for x in plan}, {"train"})
        self.assertEqual(plan, reanalyse.plan_positions(self.games, self.decisions, 2, 30, 0.0, 5), "the same every time")
        self.assertNotEqual(plan, reanalyse.plan_positions(self.games, self.decisions, 2, 30, 0.0, 6))
        self.assertEqual(len(reanalyse.plan_positions(self.games, self.decisions, 2, 31, 0.0, 5)), 31, "a game in part")
        self.assertEqual(reanalyse.plan_games(plan), [("selfplay", 100 + k) for k in range(15)])

    def test_a_game_with_no_decision_gives_none_and_one_with_few_gives_what_it_has(self):
        self.decisions[("selfplay", 100)] = []
        self.decisions.pop(("selfplay", 101))
        self.decisions[("selfplay", 102)] = [[4, 0, 2]]
        plan = reanalyse.plan_positions(self.games, self.decisions, 3, 7, 0.0, 5)
        self.assertEqual([x["seed"] for x in plan], [102, 103, 103, 103, 104, 104, 104])

    def test_games_are_held_out_whole_in_the_share_asked_evenly_through_the_plan(self):
        plan = reanalyse.plan_positions(self.games, self.decisions, 2, 80, 0.25, 5)
        by_game = {}
        for x in plan:
            by_game.setdefault(x["seed"], set()).add(x["split"])
        self.assertTrue(all(len(v) == 1 for v in by_game.values()), "a game's positions are on one side")
        held = [k for k, g in enumerate(self.games) if by_game[g["seed"]] == {"heldout"}]
        self.assertEqual(held, list(range(3, 40, 4)), "every fourth game: any stretch of the plan holds the share")
        # However few the games, both sides have some; a tenth is every tenth game; none asked for is none.
        self.assertEqual([reanalyse.held_out(0.5, k) for k in range(3)], [False, True, False])
        self.assertEqual([k for k in range(30) if reanalyse.held_out(0.1, k)], [9, 19, 29])
        self.assertFalse(any(reanalyse.held_out(0.0, k) for k in range(30)))
        # A game with nothing to draw is not counted: the share is of the games in the plan.
        self.decisions[("selfplay", 100)] = []
        plan = reanalyse.plan_positions(self.games, self.decisions, 2, 16, 0.25, 5)
        self.assertEqual(sorted({x["seed"] for x in plan if x["split"] == "heldout"}), [104, 108])

    def test_the_probes_one_lane_step_takes_the_latest_turns(self):
        latest = reanalyse.plan_positions(self.games[:3], self.decisions, 1, 3, 0.0, 5, latest=True)
        self.assertEqual([x["turn"] for x in latest], [11, 11, 11])


class Lanes(unittest.TestCase):
    def test_the_memory_bound_is_above_the_largest_tree_measured(self):
        # 131,072 simulations: 66 KB a simulation at the most was measured, and a bound of 40 KB stopped 18% short.
        self.assertEqual(reanalyse.memory_mb(131072), 8192)
        self.assertEqual(reanalyse.memory_mb(64), 2048, "never under the search's default")
        self.assertEqual(reanalyse.Settings(simulations=131072).search_words(), "RootShortcut=false,MemoryMb=8192")
        self.assertEqual(reanalyse.Settings(memory_mb=12000, search=("Gumbel=false",)).search_words(),
                         "RootShortcut=false,MemoryMb=12000,Gumbel=false")

    def test_lanes_fit_the_free_memory_less_its_headroom_and_the_cpus_less_the_servers(self):
        got = reanalyse.pick_lanes(8 * GIB, 256 * GIB, 64, headroom=0.25)
        self.assertEqual((got["lanes"], got["limiter"], got["by_memory"], got["by_cpu"]), (24, "memory", 24, 61))
        self.assertEqual(reanalyse.pick_lanes(1 * GIB, 256 * GIB, 64, headroom=0.25)["limiter"], "cpu")
        self.assertEqual(reanalyse.pick_lanes(1 * GIB, 256 * GIB, 64, most=10)["lanes"], 10)
        self.assertEqual(reanalyse.pick_lanes(40 * GIB, 32 * GIB, 64)["lanes"], 1, "a lane larger than the machine")
        self.assertEqual(reanalyse.pick_lanes(8 * GIB, None, None)["lanes"], 1, "nothing known: one lane")

    def test_the_climb_stops_at_the_first_step_that_is_not_faster(self):
        rates = {8: 7000.0, 16: 12000.0, 24: 16000.0, 32: 16500.0, 48: 17000.0}
        asked = []
        got = reanalyse.climb((8, 16, 24, 32, 48, 64), 48, lambda n: asked.append(n) or rates[n])
        self.assertEqual((got, asked), ((24, "throughput"), [8, 16, 24, 32]))
        self.assertEqual(reanalyse.climb((8, 16), 12, lambda n: n * 1000.0), (12, "cap"), "every step faster: the most it may")

    def test_a_runs_searches_are_summarised(self):
        plan = [dict(id=f"p{k}") for k in range(6)]
        judged = {"p0": dict(target=target()), "p1": dict(target=target(False)), "p2": dict(target=target(memory_bound=True)),
                  "p3": dict(error="decision 4 has no choice in the recorded game"), "p4": dict(target=dict(error="x"))}
        got = reanalyse.summarise(plan, judged)
        self.assertEqual((got["planned"], got["searched"], got["disagree"], got["memory_bound"]), (6, 3, 1, 1))
        self.assertEqual(sum(got["not_searched"].values()), 3)
        self.assertAlmostEqual(got["kl_median"], 0.05)


class Runner(unittest.TestCase):
    """The stage over stand-ins: `call` is the engine's commands, `judge` the miner's distribution."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.gen_dir = self.root / "gen7"
        self.recorded = recordings(self.gen_dir / "selfplay", range(700, 760))
        self.patch = unittest.mock.patch.object(reanalyse, "recorded_games", lambda d: self.recorded)
        self.patch.start()
        # The held-out games' keys are read from the corpus's shards: the stand-in shards hold their keep lists.
        self.keys = unittest.mock.patch.object(reanalyse, "heldout_keys", lambda dirs: {
            json.loads(line)["seed"] for d in dirs for f in d.glob("*.shard.zst") for line in f.read_text().splitlines()})
        self.keys.start()
        self.called, self.judgements, self.failing, self.logs = [], [], set(), []
        self.now = 1000.0
        self.seconds_a_game = 10.0      # what a game of a part takes on the stand-in's clock
        self.mismatched = 0
        self.bound = set()              # plan ids whose search stops at its memory bound

    def tearDown(self):
        self.patch.stop()
        self.keys.stop()
        self.tmp.cleanup()

    def call(self, cmd, log):
        cmd = [str(c) for c in cmd]
        self.called.append(cmd[1])
        arg = lambda k: Path(cmd[cmd.index(k) + 1])  # noqa: E731
        if cmd[1] == "mine-decisions":
            self.assertTrue((arg("--dir") / "traces" / "games.jsonl").exists(), "asked of the recordings laid out")
            games = reanalyse.read_jsonl(arg("--games"))
            reanalyse.write_lines(arg("--out"), [dict(seed=g["seed"], decisions=[[d, d % 2, d] for d in range(10)]) for g in games])
            return '{\n  "Games": %d,\n  "Errors": 0\n}\n' % len(games)
        if cmd[1] == "mine-filter":
            kept = reanalyse.read_jsonl(arg("--keep"))
            arg("--out").mkdir(parents=True)
            (arg("--out") / "mine-00000.shard.zst").write_text(arg("--keep").read_text())
            return 'cantrip-explore: a note\n{\n  "Asked": %d,\n  "Decisions": %d,\n  "Games": 1\n}\n' % (len(kept), len(kept))
        raise AssertionError(cmd)

    def judge(self, setup, p, s, model, decks, explore, local_lanes, worker_lanes, runs=None, data=None, chunk=None,
              stop_check=None):
        """`mining.judge_distributed`'s effects for one run: its ledger, its parts' reports and its merged
        judgements; a part a `chunk` of games, the clock moved by each, the run closed when `stop_check` says so."""
        run = runs("model", "decks", p.root, str(p.judge))[0]
        args, name = run["args"], run["name"]
        hosts = [w["name"] for w in setup.workers] + (["here"] if setup.local else [])
        lanes = {w["name"]: w["judge_lanes"] for w in setup.workers} | ({"here": local_lanes} if setup.local else {})
        self.judgements.append(dict(name=name, hosts=hosts, lanes=lanes, chunk=chunk, args=args, data=sorted(data)))
        if name in self.failing:
            raise SystemExit("the judgements stopped: run the driver again to go on")
        plan = reanalyse.read_jsonl(Path(args[args.index("--plan") + 1]))
        for x in plan:
            self.assertIn(f"data/recorded/{x['dir']}/{self.recorded[x['seed']]['shard']}", data, "its shard is pushed")
        games = reanalyse.plan_games(plan)
        dist = setup.gen_dir / "dist" / f"judge-{name}"
        ledger = dict(games=len(games), shards={}, closed=None, skipped={})
        lines = []
        for k in range(0, len(games), chunk):
            why = stop_check(None) if stop_check else None
            if why:
                ledger["closed"] = why
                ledger["skipped"] = {str(i): why for i in range(k, len(games))}
                break
            part, host = f"{hosts[0]}-a{k // chunk + 1}.judged", hosts[(k // chunk) % len(hosts)]
            mine_ = [x for x in plan if (x["dir"], x["seed"]) in games[k:k + chunk]]
            self.now += self.seconds_a_game * len(games[k:k + chunk])
            ledger["shards"][part] = dict(host=host, indices=[[k, min(k + chunk, len(games))]])
            (dist / "results" / part).mkdir(parents=True, exist_ok=True)
            (dist / "results" / part / "judge-report.json").write_text(json.dumps(dict(
                PeakBytes=lanes[host] << 29, AvailableBytes=40 * GIB, Cpus=64, Lanes=lanes[host],
                MismatchedPositions=self.mismatched, Samples=len(mine_), Seconds=len(mine_) * 64 / (min(lanes[host], 24) * 1000.0))))
            for x in mine_:
                lines.append(dict(id=x["id"], seat=x["seat"], turn=x["turn"], kind="Priority",
                                  target=target(x["decision"] % 3 != 0, memory_bound=x["id"] in self.bound)))
                (p.judge / name / x["split"] / x["dir"]).mkdir(parents=True, exist_ok=True)
                (p.judge / name / x["split"] / x["dir"] / f"{part}-mine.shard.zst").write_text("targets")
        dist.mkdir(parents=True, exist_ok=True)
        (dist / "ledger.json").write_text(json.dumps(ledger))
        reanalyse.write_lines(p.judge / name / "judged.jsonl", lines)
        return {name: {h: sum(len(distributed.from_ranges(sh["indices"])) for sh in ledger["shards"].values() if sh["host"] == h)
                       for h in hosts}}

    def stage(self, gen: int = 7, workers=(), local: bool = True, **settings):
        s = reanalyse.Settings(**{**dict(positions=24, worlds=2, simulations=64, mode="decklist", lanes=4, heldout=0.25), **settings})
        setup = duels.Setup(**{**a_setup(self.root, workers, local).__dict__, "gen": gen, "gen_dir": self.root / f"gen{gen}"})
        return reanalyse.Reanalysis(self.root, gen, s, self.root / "champions" / "gen6", self.root / "decks",
                                    Path("cantrip-explore"), setup, self.call, engine=dict(id="e1", commit="abc"),
                                    log=self.logs.append, clock=lambda: self.now, judge=self.judge)

    def test_the_whole_stage_leaves_a_corpus_and_nothing_is_made_twice(self):
        got = self.stage().run()
        corpus = self.root / "corpus" / "deep" / "gen7"
        record = json.loads((corpus / "CORPUS.json").read_text())
        self.assertTrue(record["complete"])
        self.assertEqual((record["judge"], record["engine"], record["source"], record["producer"]),
                         ("gen6", dict(id="e1", commit="abc"), "selfplay", "deep:gen6@gen7"))
        self.assertEqual((record["simulations"], record["worlds"], record["search"]), (64, 2, "RootShortcut=false,MemoryMb=2048"))
        self.assertEqual(record["planned"], 24)
        made = record["directories"]["seed-1"]
        self.assertEqual(made["train"] + made["heldout"], 24)
        self.assertGreater(made["heldout"], 0)
        for split in ("train", "heldout"):
            d = corpus / "seed-1" / split
            self.assertTrue(any(d.glob("*.shard.zst")))
            self.assertEqual(json.loads((d / "effort.json").read_text()), dict(simulations=64, worlds=2))
        # What the stage's record says.
        self.assertEqual((got["corpus"], got["judge"], got["searched"], got["positions"]), ("corpus/deep/gen7", "gen6", 24, 24))
        self.assertEqual((made["train"], made["heldout"]), (18, 6), "a quarter of the twelve games is held out")
        # The held-out games are named, for every later training to leave out of the self-play that holds them.
        named = json.loads((corpus / reanalyse.KEYS).read_text())
        self.assertEqual((len(named), record["heldout_games"]), (3, 3))
        self.assertEqual(reanalyse.exclusions(self.root), [corpus / reanalyse.KEYS])
        self.assertEqual(got["disagree"], sum(1 for x in reanalyse.read_jsonl(self.gen_dir / "reanalyse" / "data" / "plan.jsonl")
                                              if x["decision"] % 3 == 0))
        # The search each position got, and that only the first seed has a budget's stop.
        j = self.judgements[0]
        self.assertEqual(j["name"], "deep-s1")
        arg = lambda k: j["args"][j["args"].index(k) + 1]  # noqa: E731
        self.assertEqual((arg("--simulations"), arg("--worlds"), arg("--seed"), arg("--targets"), arg("--mode"), arg("--lanes")),
                         ("64", "2", "1", "true", "decklist", "1"))
        self.assertEqual(arg("--dir"), f"selfplay={(self.gen_dir / 'reanalyse' / 'data').as_posix()}/recorded/selfplay")
        self.assertEqual((j["hosts"], j["lanes"], j["chunk"]), (["here"], {"here": 4}, 4))
        self.assertEqual(self.called.count("mine-decisions"), 1)
        # Run again: nothing is sampled, judged or written again.
        calls, judgements = len(self.called), len(self.judgements)
        again = self.stage().run()
        self.assertEqual((len(self.called), len(self.judgements)), (calls, judgements))
        self.assertEqual(again["positions"], 24)

    def test_a_stage_stopped_while_judging_goes_on_with_its_plan(self):
        self.failing = {"deep-s1"}
        with self.assertRaises(SystemExit):
            self.stage().run()
        plan = (self.gen_dir / "reanalyse" / "data" / "plan.jsonl").read_text()
        self.assertFalse((self.root / "corpus" / "deep" / "gen7" / "CORPUS.json").exists())
        self.failing = set()
        self.stage().run()
        self.assertEqual((self.gen_dir / "reanalyse" / "data" / "plan.jsonl").read_text(), plan, "the same positions")
        self.assertEqual(self.called.count("mine-decisions"), 1, "not sampled again")
        self.assertTrue((self.root / "corpus" / "deep" / "gen7" / "CORPUS.json").exists())

    def test_a_time_budget_plans_what_it_buys_and_stops_the_judging_when_it_is_spent(self):
        # Four lanes at 700 simulations a second each, 64 a search: 0.05 h buys 7,875 searches; the games hold fewer.
        stage = self.stage(positions=0, hours=0.05, per_game=2)
        self.seconds_a_game = 30.0     # the searches turn out slower than the estimate: 6 games a part, 3 minutes a part
        got = stage.run()
        said = "\n".join(self.logs)
        self.assertIn("the 0.05 h budget buys about", said)
        plan = reanalyse.read_jsonl(self.gen_dir / "reanalyse" / "data" / "plan.jsonl")
        self.assertEqual(len(plan), 2 * len(self.recorded) - 0, "every game there is: the budget buys more")
        # 180 s of budget, a part of four games 120 s: closed after the second part, the rest skipped.
        self.assertEqual(got["closed"], "the time budget (0.05 h) is spent")
        self.assertEqual(got["searched"], 16)
        self.assertEqual(got["planned"], len(plan))
        ledger = json.loads((self.gen_dir / "dist" / "judge-deep-s1" / "ledger.json").read_text())
        self.assertEqual(len(ledger["skipped"]), len(self.recorded) - 8)
        record = json.loads((self.root / "corpus" / "deep" / "gen7" / "CORPUS.json").read_text())
        self.assertEqual(record["positions"], 16)
        self.assertAlmostEqual(record["seconds"], 240.0)
        # What it has spent of its budget is said beside the ledger, for the projection (`status.BUDGET`).
        said = json.loads((self.gen_dir / "dist" / "judge-deep-s1" / status.BUDGET).read_text())
        self.assertEqual((said["seconds"], said["spent"]), (180.0, 240.0))
        self.assertEqual(status.budget_left(said, self.now), 0.0)
        self.assertAlmostEqual(record["seconds_a_position"], 15.0)

    def test_the_budget_counts_what_an_earlier_run_of_the_stage_spent(self):
        stage = self.stage(positions=40, hours=0.05)
        self.seconds_a_game = 30.0
        self.failing = {"deep-s1"}
        with self.assertRaises(SystemExit):
            stage.run()
        # An earlier run judged for 150 s before it was stopped.
        stage.save_state(spent={"deep-s1": 150.0})
        self.failing = set()
        got = self.stage(positions=40, hours=0.05).run()
        self.assertEqual(got["searched"], 8, "one part more (120 s), and the budget of 180 s is spent")

    def test_no_more_positions_than_asked_for_whatever_the_budget_buys(self):
        got = self.stage(positions=10, hours=5.0).run()
        self.assertEqual((got["planned"], got["searched"], got["closed"]), (10, 10, None))

    def test_a_later_seed_searches_what_the_first_did(self):
        stage = self.stage(positions=0, hours=0.05, seeds=(1, 2))
        self.seconds_a_game = 30.0
        stage.run()
        first, second = self.judgements
        self.assertEqual((first["name"], second["name"]), ("deep-s1", "deep-s2"))
        plan2 = reanalyse.read_jsonl(Path(second["args"][second["args"].index("--plan") + 1]))
        self.assertEqual(len(plan2), 16, "the positions the first seed searched before its budget was spent")
        self.assertEqual(second["args"][second["args"].index("--seed") + 1], "2")
        record = json.loads((self.root / "corpus" / "deep" / "gen7" / "CORPUS.json").read_text())
        self.assertEqual(sorted(record["directories"]), ["seed-1", "seed-2"])
        self.assertEqual(record["positions"], 32)
        self.assertIsNone(record["runs"]["deep-s2"]["closed"], "no budget stops a later seed")

    def test_each_host_is_probed_by_itself_once_a_line(self):
        worker = dict(name="w1", os="windows", lease="L")
        self.stage(workers=[worker], local=True, lanes=0, probe_ladder=(8, 16, 24, 32), probe_games=1).run()
        names = [j["name"] for j in self.judgements]
        # One lane first, then up the ladder until a step is not faster (the stand-in's GPU serves 24 lanes' worth).
        self.assertEqual(names[:5], ["probe-w1-1", "probe-w1-8", "probe-w1-16", "probe-w1-24", "probe-w1-32"])
        self.assertEqual([j["hosts"] for j in self.judgements[:5]], [["w1"]] * 5, "the worker alone")
        self.assertEqual(names[5:10], ["probe-local-1", "probe-local-8", "probe-local-16", "probe-local-24", "probe-local-32"])
        self.assertEqual(self.judgements[5]["hosts"], ["here"])
        self.assertEqual(names[10:], ["deep-s1"])
        self.assertEqual(self.judgements[-1]["lanes"], {"w1": 24, "here": 24})
        self.assertEqual(self.judgements[-1]["chunk"], 24, "a part is a host's lanes of games")
        probe = json.loads((self.root / "corpus" / "deep" / "probe.json").read_text())
        self.assertEqual(probe["lanes"], {"w1": 24, "local": 24})
        self.assertEqual(probe["hosts"]["w1"]["limiter"], "throughput")
        # The probe's games are not the plan's.
        probed = {x["seed"] for f in (self.gen_dir / "reanalyse" / "probe").glob("plan-probe*.jsonl") for x in reanalyse.read_jsonl(f)}
        planned = {x["seed"] for x in reanalyse.read_jsonl(self.gen_dir / "reanalyse" / "data" / "plan.jsonl")}
        self.assertFalse(probed & planned)
        # The next generation's stage, the same searches: no probe.
        self.recorded = recordings(self.root / "gen8" / "selfplay", range(800, 860))
        before = len(self.judgements)
        self.stage(gen=8, workers=[worker], local=True, lanes=0).run()
        self.assertEqual([j["name"] for j in self.judgements[before:]], ["deep-s1"])
        self.assertEqual(self.judgements[-1]["lanes"], {"w1": 24, "here": 24})
        # Deeper searches are other searches: probed again.
        self.recorded = recordings(self.root / "gen9" / "selfplay", range(900, 960))
        self.stage(gen=9, workers=[worker], local=False, lanes=0, simulations=128, probe_ladder=(8,), probe_games=1).run()
        self.assertEqual(self.judgements[before + 1]["name"], "probe-w1-1")

    def test_a_host_short_of_memory_runs_what_fits(self):
        # The stand-in's lane holds half a gigabyte and its host has 40 GB free: with three quarters of it left
        # unused, 20 lanes - fewer than its GPU would serve, so the climb ends at what fits.
        self.stage(lanes=0, headroom=0.75, probe_ladder=(8, 16, 24), probe_games=1).run()
        self.assertEqual(self.judgements[-1]["lanes"], {"here": 20})
        self.assertEqual([j["name"] for j in self.judgements], ["probe-local-1", "probe-local-8", "probe-local-16",
                                                               "probe-local-20", "deep-s1"])
        self.assertEqual(json.loads((self.root / "corpus" / "deep" / "probe.json").read_text())["hosts"]["local"]["limiter"], "memory")

    def test_a_probe_whose_replay_is_not_the_recording_stops_the_stage(self):
        self.mismatched = 2
        with self.assertRaises(SystemExit) as caught:
            self.stage(lanes=0).run()
        self.assertIn("MismatchedPositions", str(caught.exception))
        self.assertEqual([j["name"] for j in self.judgements], ["probe-local-1"])

    def test_this_machine_left_out_judges_nothing(self):
        worker = dict(name="w1", os="windows", lease="L")
        self.stage(workers=[worker], local=False).run()
        self.assertEqual(self.judgements[0]["hosts"], ["w1"])
        self.assertEqual(self.judgements[0]["lanes"], {"w1": 4})
        self.assertNotIn("judge_lanes", worker, "the held worker's own record is not written into")
        with self.assertRaises(SystemExit) as caught:
            self.stage(gen=8, local=False).run()
        self.assertIn("nothing to reanalyse", str(caught.exception))
        self.recorded = recordings(self.root / "gen8" / "selfplay", range(800, 810))
        with self.assertRaises(SystemExit) as caught:
            self.stage(gen=8, local=False).run()
        self.assertIn("no host to judge on", str(caught.exception))
        self.assertEqual(status.transient(str(caught.exception)), "reanalyse-no-search", "a worker out: run again, not held")

    def test_other_settings_are_another_reanalysis(self):
        self.failing = {"deep-s1"}
        with self.assertRaises(SystemExit):
            self.stage().run()
        # Nothing searched yet: started over with the new settings.
        self.failing = set()
        self.stage(worlds=4).run()
        self.assertIn("started over", "\n".join(self.logs))
        # Searches are in (and a corpus): other settings are refused, not mixed in.
        with self.assertRaises(SystemExit) as caught:
            self.stage(worlds=8).run()
        self.assertIn("other settings (worlds)", str(caught.exception))

    def test_positions_are_searched_in_the_mode_their_games_were_recorded_in(self):
        # A line in full mode whose gate was recorded in decklist mode, as the miner records it.
        now = dict(schema=dataset.LAYOUT["schema"], encoder_hash=dataset.encoder_hash())
        with unittest.mock.patch.object(reanalyse, "shard_header", lambda path: dict(now, mode="decklist")):
            self.stage(mode="full").run()
        args = self.judgements[0]["args"]
        self.assertEqual(args[args.index("--mode") + 1], "decklist")
        record = json.loads((self.root / "corpus" / "deep" / "gen7" / "CORPUS.json").read_text())
        self.assertEqual((record["mode"], record["schema"], record["encoder_hash"]), ("decklist", now["schema"], now["encoder_hash"]))
        self.assertEqual(reanalyse.readable(record, dict(now, mode="full")), "its mode is decklist, this generation's full")
        self.assertIsNone(reanalyse.readable(record, dict(now, mode="decklist")))

    def test_a_worker_outage_is_a_failure_to_run_again_and_a_real_one_is_not(self):
        real = self.judge

        def out(*a, **k):
            raise SystemExit("distributed judge: every host has failed; nothing left to play the 12 pending (see the log above)")

        self.judge = out
        with self.assertRaises(SystemExit) as caught:
            self.stage().run()
        why = str(caught.exception)
        self.assertTrue(why.startswith(reanalyse.NO_SEARCH), why)
        self.assertIn("every host has failed", why)
        # As the driver says it, a data stage's failure and all: still the supervisor's to run again after a wait.
        said = f"stage reanalyse failed, and its output is training data: ... go on without them.\n{why}"
        self.assertEqual(status.transient(said), "reanalyse-no-search")
        self.assertFalse(status.asked_to_stop(said))
        # Any other failure of the judgements is itself, and holds the loop.

        def broken(*a, **k):
            raise SystemExit("judge-deep-s1 was made with other settings")

        self.judge = broken
        with self.assertRaises(SystemExit) as caught:
            self.stage().run()
        self.assertIsNone(status.transient(str(caught.exception)))
        # A stop asked for while the searches ran is a stop, neither a failure nor one to run again unasked.
        stopped = "the judgements stopped: run the driver again to go on (ledger.json)"
        self.assertTrue(status.asked_to_stop(stopped))
        self.assertIsNone(status.transient(stopped))
        self.judge = real
        self.stage().run()

    def test_a_probe_that_searched_nothing_is_a_failure_to_run_again(self):
        real = self.judge

        def empty(setup, p, *a, runs=None, **k):
            run = runs("model", "decks", p.root, str(p.judge))[0]
            reanalyse.write_lines(p.judge / run["name"] / "judged.jsonl", [])
            return {run["name"]: {}}

        self.judge = empty
        with self.assertRaises(SystemExit) as caught:
            self.stage(lanes=0).run()
        self.assertIn("searched nothing", str(caught.exception))
        self.assertEqual(status.transient(str(caught.exception)), "reanalyse-no-search")
        self.judge = real

    def test_nothing_accepted_and_nothing_written_is_a_failure_to_run_again(self):
        def none(setup, p, *a, runs=None, **k):
            run = runs("model", "decks", p.root, str(p.judge))[0]
            reanalyse.write_lines(p.judge / run["name"] / "judged.jsonl", [])
            (setup.gen_dir / "dist" / f"judge-{run['name']}").mkdir(parents=True, exist_ok=True)
            (setup.gen_dir / "dist" / f"judge-{run['name']}" / "ledger.json").write_text(json.dumps(dict(games=12, shards={})))
            return {run["name"]: {}}

        self.judge = none
        with self.assertRaises(SystemExit) as caught:
            self.stage().run()
        self.assertIn("wrote no target", str(caught.exception))
        self.assertEqual(status.transient(str(caught.exception)), "reanalyse-no-search")

    def test_the_probe_is_kept_for_the_network_size_the_engine_and_the_lane_caps_it_was_made_for(self):
        def stage(gen, model="gen6", **more):
            self.recorded = recordings(self.root / f"gen{gen}" / "selfplay", range(gen * 100, gen * 100 + 60))
            st = self.stage(gen=gen, lanes=0, probe_ladder=(8,), probe_games=1, **more)
            st.model = self.root / "champions" / model
            return st

        for name, size in (("gen6", 7_000_000), ("gen7", 7_000_000), ("big", 30_000_000)):
            (self.root / "champions" / name).mkdir(parents=True)
            (self.root / "champions" / name / "manifest.json").write_text(json.dumps(dict(parameters=size, config="{'width': 256}")))
        probes = lambda: sum(1 for j in self.judgements if j["name"].startswith("probe-"))  # noqa: E731
        stage(7).run()
        first = probes()
        self.assertGreater(first, 0)
        stage(8, model="gen7").run()
        self.assertEqual(probes(), first, "a new champion of the same size is the same network to a probe")
        stage(9, model="big").run()
        self.assertGreater(probes(), first, "a larger network is served and searched at another rate")
        again = probes()
        st = stage(10, model="big")
        st.engine = dict(id="e2", commit="def")
        st.run()
        self.assertGreater(probes(), again, "another engine")
        again = probes()
        st = stage(11, model="big", local_lanes=6)
        st.engine = dict(id="e2", commit="def")
        st.run()
        self.assertGreater(probes(), again, "another cap on the lanes")
        self.assertEqual(self.judgements[-1]["lanes"], {"here": 6})

    def test_a_host_whose_free_memory_moved_is_probed_again_the_next_time(self):
        self.stage(lanes=0, probe_ladder=(8,), probe_games=1).run()
        probe = self.root / "corpus" / "deep" / "probe.json"
        self.assertIn("local", json.loads(probe.read_text())["hosts"])
        # The next generation's parts report half the memory free that the probe saw.
        kept = json.loads(probe.read_text())
        kept["hosts"]["local"]["available_bytes"] = 80 * GIB
        probe.write_text(json.dumps(kept))
        self.recorded = recordings(self.root / "gen8" / "selfplay", range(800, 860))
        self.stage(gen=8, lanes=0, probe_ladder=(8,), probe_games=1).run()
        self.assertNotIn("local", json.loads(probe.read_text())["hosts"])
        self.assertTrue(any("probed again next time" in line for line in self.logs))

    def test_searches_that_wrote_nothing_fail_the_stage(self):
        real = self.call

        def nothing(cmd, log):
            if str(cmd[1]) == "mine-filter":
                return '{"Decisions": 0}'
            return real(cmd, log)

        self.call = nothing
        with self.assertRaises(SystemExit) as caught:
            self.stage().run()
        self.assertIn("wrote no target", str(caught.exception))
        self.assertIsNone(status.transient(str(caught.exception)), "parts were accepted: running it again changes nothing")
        self.assertFalse((self.root / "corpus" / "deep" / "gen7" / "CORPUS.json").exists())


class LedgerSaved(unittest.TestCase):
    def test_a_ledger_read_while_it_is_saved_is_saved_all_the_same(self):
        # Windows refuses to rename over a file a reader has open (the status file's keeper reads the ledgers).
        real, refused = distributed.os.replace, []

        def busy(src, dst):
            if len(refused) < 3:
                refused.append(dst)
                raise PermissionError(5, "Access is denied")
            return real(src, dst)

        with tempfile.TemporaryDirectory() as tmp, unittest.mock.patch.object(distributed.os, "replace", busy):
            with unittest.mock.patch.object(distributed.time, "sleep", lambda s: None):
                led = distributed.Ledger(Path(tmp) / "ledger.json", 4, 0)
                led.save()
                self.assertEqual(json.loads((Path(tmp) / "ledger.json").read_text())["games"], 4)
                self.assertEqual(len(refused), 3)
                distributed.write_atomic(Path(tmp) / "state.json", "{}")
                self.assertEqual((Path(tmp) / "state.json").read_text(), "{}")

    def test_a_rename_that_is_refused_for_good_is_an_error(self):
        def never(src, dst):
            raise PermissionError(5, "Access is denied")

        with tempfile.TemporaryDirectory() as tmp, unittest.mock.patch.object(distributed.os, "replace", never):
            (Path(tmp) / "a.tmp").write_text("x")
            with self.assertRaises(PermissionError):
                distributed.replace(Path(tmp) / "a.tmp", Path(tmp) / "a", seconds=0.0)


class Budget(unittest.TestCase):
    """The mechanism the time budget rests on, with the real coordinator: a `stop_check` closes the ledger, the part
    in flight is taken, the rest is skipped, and the run ends done and merges what it has."""

    def test_a_closed_run_keeps_its_parts_and_skips_the_rest(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            plan = tmp / "plan.jsonl"
            plan.write_text("".join(json.dumps(dict(id=f"p{k:02}", dir="selfplay", seed=k)) + "\n" for k in range(6)))
            header = mining.judge_header(6, ["mine-judge", "--simulations", "64", "--worlds", "2", "--seed", "1", "--targets", "true"])
            host = FakeJudgeHost("a", tmp, plan, header)
            led = distributed.Ledger(tmp / "ledger.json", 6, 0)
            led.header = header
            out = tmp / "results"
            out.mkdir()
            polls = []

            def spent(ledger):
                polls.append(len(ledger.done))
                return "the time budget (1 h) is spent" if len(ledger.done) >= 2 else None

            c = distributed.Coordinator(led, [host], out, tmp / "dist", read=lambda path: mining.read_judged(path, 6),
                                        log=lambda *_: None, sleep=lambda s: None, kind=mining.JUDGE, chunk=2, initial=2,
                                        lookahead=0.0, stop_check=spent)
            self.assertEqual(c.run(), "done")
            self.assertEqual(led.closed, "the time budget (1 h) is spent")
            self.assertEqual(sorted(led.done), [0, 1], "what was judged before the budget was spent is kept")
            self.assertEqual(sorted(led.skipped), [2, 3, 4, 5])
            self.assertTrue(led.complete())
            mining.merge_judged(led, out, tmp / "judge" / "deep-s1")
            lines = [json.loads(x) for x in (tmp / "judge" / "deep-s1" / "judged.jsonl").read_text().splitlines()]
            self.assertEqual([x["id"] for x in lines], ["p00", "p01"])
            # What the status reads of it: its games, and that it was played to its end; and its attempts are
            # judgements' (`status.WHOLE_KINDS`): parts granted whole, which the projection counts as such.
            saved = json.loads((tmp / "ledger.json").read_text())
            self.assertEqual((saved["games"], len(saved["skipped"])), (6, 4))
            self.assertEqual({a["kind"] for a in saved["attempts"].values()}, {"judge"})
            self.assertIn(mining.JUDGE.name, status.WHOLE_KINDS)
            work = status.ledger_work(saved)
            self.assertEqual((work["total"], work["played"], work["skipped"], work["grain"]), (6, 2, 4, 2.0))
            self.assertGreaterEqual(work["played"] + work["skipped"], work["total"])


class Projected(unittest.TestCase):
    """The stage under the overrun projection (`status.project`): held to an owner's `--expect reanalyse=H` or the
    earlier generations' pace as any stage is, but never projected past the end of its own time budget."""

    def reading(self, budget=None, rate=1.0):
        # Watched for an hour: `rate` games a minute of 1,200 planned, a sample a minute.
        now = 10_000.0
        samples = [[now - 3600 + 60 * k, rate * k] for k in range(1, 61)]
        work = dict(stage="reanalyse", ledger="dist/judge-deep-s1/ledger.json", since=now - 3600, start=0.0, done=rate * 60,
                    total=1200, grain=1.0, samples=samples, at=now, hosts=["w1"], pace=None, budget=budget)
        return dict(stage="reanalyse", state="running", work=work, expected_seconds=None), now

    def test_a_slow_stage_is_held_to_what_its_owner_expects(self):
        s, now = self.reading()
        p = status.project(s, now, expect=status.parse_expect(["reanalyse=5"]))
        # 60 of 1,200 games in an hour: nineteen hours more, against five expected.
        self.assertAlmostEqual(p["remaining"] / 3600, 19.0, places=3)
        self.assertTrue(status.overrun(p))
        self.assertIn("reanalyse projected to finish in", p["text"])

    def test_with_a_time_budget_it_ends_with_the_budget_and_is_no_overrun(self):
        # Four hours' budget, one spent: the searches are slow, so fewer are made - the stage still ends on time.
        s, now = self.reading(budget=dict(seconds=4 * 3600.0, spent=3600.0, at=10_000.0 - 30))
        p = status.project(s, now, expect=status.parse_expect(["reanalyse=5"]))
        self.assertTrue(p["budgeted"])
        self.assertAlmostEqual(p["remaining"], 3 * 3600.0 - 30)
        self.assertFalse(status.overrun(p))
        self.assertIn("ends with its time budget in", p["text"])
        # A budget that reaches past what is left changes nothing.
        s, now = self.reading(budget=dict(seconds=40 * 3600.0, spent=3600.0, at=10_000.0))
        p = status.project(s, now, expect=status.parse_expect(["reanalyse=5"]))
        self.assertNotIn("budgeted", p)
        self.assertTrue(status.overrun(p))

    def test_a_budget_spent_with_the_stage_still_running_counts_as_any_time(self):
        # Parts granted whole and none delivered: no rate. While the budget lasts nothing is projected; once it is
        # spent and the run goes on and on, what it has spent is its projection.
        for spent, said in ((3600.0, False), (4 * 3600.0, True)):
            s, now = self.reading(budget=dict(seconds=4 * 3600.0, spent=spent, at=10_000.0), rate=0.0)
            s["work"]["since"] = now - 7 * 3600
            p = status.project(s, now, expect=status.parse_expect(["reanalyse=3"]))
            self.assertEqual(p is not None, said)
        self.assertTrue(status.overrun(p))
        self.assertEqual(status.budget_left(dict(seconds=100.0, spent=40.0, at=50.0), 70.0), 40.0)
        self.assertIsNone(status.budget_left(None, 70.0))
        self.assertIsNone(status.budget_left(dict(seconds="x"), 70.0))

    def test_a_budgeted_stage_is_expected_to_take_its_budget_and_a_grace_whoever_plays_it(self):
        # Four hours' budget, one spent when this run first saw the ledger an hour ago (so two are spent now).
        budget = dict(seconds=4 * 3600.0, spent=2 * 3600.0, at=10_000.0)
        s, now = self.reading(budget=budget)
        # The earlier generations' two hosts made twenty times this pace: one host of two is playing now.
        s["work"]["pace"] = dict(seconds_per_game=3.0, generations=[3], rates={"w1": 1 / 6.0, "w2": 1 / 6.0}, other=False)
        e = status.expectation(s)
        self.assertEqual((e["known"], e["wide"]), (True, False))
        self.assertAlmostEqual(e["seconds"], 3 * 3600.0 + 0.15 * 4 * 3600.0, msg="what was left of the budget, and 15% of it")
        self.assertIn("its time budget of", e["basis"])
        p = status.project(s, now)
        self.assertTrue(p["budgeted"])
        self.assertFalse(status.overrun(p), "within its budget: never held, however slow against the hosts' history")
        # Without the budget the same reading is an overrun against that history.
        s["work"]["budget"] = None
        self.assertTrue(status.overrun(status.project(s, now)))
        # An owner's --expect still comes first.
        s["work"]["budget"] = budget
        self.assertEqual(status.expectation(s, status.parse_expect(["reanalyse=9"]))["seconds"], 9 * 3600.0)

    def test_a_first_generations_runaway_after_its_budget_is_caught(self):
        # No history at all. The budget of an hour was spent five hours ago; the parts in flight never come.
        budget = dict(seconds=3600.0, spent=6 * 3600.0, at=10_000.0)
        s, now = self.reading(budget=budget, rate=0.0)
        s["work"]["since"] = now - 6 * 3600
        self.assertEqual(status.budget_left(budget, now), 0.0)
        p = status.project(s, now)
        self.assertIsNotNone(p)
        self.assertAlmostEqual(p["expected"], 3600.0 * 1.15)
        self.assertTrue(status.overrun(p))
        # And one that still moves, slowly, is projected at its rate once the budget is spent - not at zero.
        s, now = self.reading(budget=budget, rate=0.05)
        s["work"]["since"] = now - 3600
        p = status.project(s, now)
        self.assertNotIn("budgeted", p)
        self.assertGreater(p["remaining"], 100 * 3600.0)
        self.assertTrue(status.overrun(p))

    def test_the_status_file_carries_the_budget_written_beside_the_ledger(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            dist = root / "gen4" / "dist" / "judge-deep-s1"
            dist.mkdir(parents=True)
            led = distributed.Ledger(dist / "ledger.json", 40, 0)
            a = led.new_attempt("w1", started=100.0)
            a.update(kind="judge", state="running")
            led.grant(a, list(range(8)))
            led.save()
            (dist / status.BUDGET).write_text(json.dumps(dict(seconds=7200.0, spent=600.0, at=150.0)))
            heart = status.Heartbeat(root, 4, clock=lambda: 200.0)
            heart.data.update(stage="reanalyse", stage_started=0.0)
            work = heart.work()
            self.assertEqual((work["ledger"], work["total"]), ("dist/judge-deep-s1/ledger.json", 40))
            self.assertEqual(work["budget"], dict(seconds=7200.0, spent=600.0, at=150.0))
            self.assertEqual(status.budget_left(work["budget"], 200.0), 6550.0)
            # A ledger with none beside it (a duel's, the miner's) carries none.
            (dist / status.BUDGET).unlink()
            self.assertIsNone(heart.work()["budget"])


class TracesHome(unittest.TestCase):
    """Distributed self-play with `--traces`: a worker's traces come home when its attempt is over."""

    class Host:
        remote, name = True, "w1"

        def __init__(self, there: Path, fail: int = 0):
            self.there, self.fail, self.asked = there, fail, 0

        def fetch_traces(self, a, dest: Path):
            import shutil  # noqa: PLC0415
            self.asked += 1
            if self.asked <= self.fail:
                raise distributed.Unreachable("the copy failed")
            shutil.copytree(self.there, dest)

    def coordinator(self, tmp: Path, host, traces=True):
        led = distributed.Ledger(tmp / "ledger.json", 4, 0)
        return distributed.Coordinator(led, [host], tmp / "selfplay", tmp / "dist", log=self.logs.append, traces=traces)

    def setUp(self):
        self.logs = []

    def test_an_attempts_traces_are_named_after_it_and_kept_beside_the_shards(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            there = tmp / "worker" / "traces"
            there.mkdir(parents=True)
            (there / "selfplay-l0-4242.jsonl").write_text('{"seed":1}\n')
            (there / "selfplay-l1-4242.jsonl").write_text('{"seed":2}\n')
            host = self.Host(there, fail=1)      # the first copy fails: tried again
            c = self.coordinator(tmp, host)
            a = dict(id="w1-a1", host="w1")
            c.take_traces(a, host)
            home = tmp / "selfplay" / "traces"
            self.assertEqual(sorted(p.name for p in home.iterdir()), ["w1-a1-selfplay-l0-4242.jsonl", "w1-a1-selfplay-l1-4242.jsonl"])
            self.assertEqual((a["traces"], host.asked), (2, 2))
            self.assertFalse((tmp / "dist" / "incoming" / "w1-a1-traces.pulling").exists())
            # The stage reads them there, and knows each line's writer from the name (`reanalyse.WRITER`).
            self.assertEqual(reanalyse.traced(tmp / "selfplay"), {1, 2})
            self.assertEqual(reanalyse.WRITER.search("w1-a1-selfplay-l0-4242.jsonl").group(1),
                             reanalyse.WRITER.search("w1-selfplay-l0-4242-1795295659300-00003.shard.zst").group(1))

    def test_a_failed_fetch_is_flagged_at_once_and_fetched_again_when_the_reanalysis_starts(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            there = tmp / "worker" / "traces"
            there.mkdir(parents=True)
            (there / "selfplay-l0-4242.jsonl").write_text('{"seed":1}\n')
            host = self.Host(there, fail=99)
            flagged = []
            led = distributed.Ledger(tmp / "dist" / "ledger.json", 4, 0)
            c = distributed.Coordinator(led, [host], tmp / "selfplay", tmp / "dist", log=self.logs.append, traces=True,
                                        warn=flagged.append)
            failed = led.new_attempt("w1")
            failed["state"] = "finished"
            c.take_traces(failed, host)
            self.assertEqual(len(flagged), 1, "told through the driver's warnings as it happens, not hours later")
            self.assertIn("w1-a1's traces were not fetched from w1", flagged[0])
            lost = led.new_attempt("w1")            # given up as lost: its traces were never asked for
            lost["state"] = "lost"
            done = led.new_attempt("w1")            # its traces are home
            done.update(state="finished", traces=2)
            elsewhere = led.new_attempt("w2")       # a host not held now
            elsewhere.update(state="lost")
            led.attempts[elsewhere["id"]]["host"] = "w2"
            led.save()
            # The worker answers again when the reanalysis starts.
            host.fail, host.asked = 0, 0
            came = distributed.retry_traces(tmp / "dist" / "ledger.json", tmp / "selfplay", tmp / "dist", {"w1": host},
                                            self.logs.append)
            self.assertEqual(came, dict(fetched=["w1-a1", "w1-a2"], failed=[]))
            self.assertEqual(host.asked, 2, "only the attempts that lack theirs, on the hosts held")
            saved = json.loads((tmp / "dist" / "ledger.json").read_text())["attempts"]
            self.assertEqual((saved["w1-a1"]["traces"], saved["w1-a2"]["traces"]), (1, 1))
            self.assertNotIn("traces_failed", saved["w1-a1"])
            self.assertEqual(sorted(p.name for p in (tmp / "selfplay" / "traces").iterdir()),
                             ["w1-a1-selfplay-l0-4242.jsonl", "w1-a2-selfplay-l0-4242.jsonl"])
            # Asked again, nothing is left to fetch; a host still out is said and recorded.
            self.assertEqual(distributed.retry_traces(tmp / "dist" / "ledger.json", tmp / "selfplay", tmp / "dist",
                                                      {"w1": host}, self.logs.append), dict(fetched=[], failed=[]))
            out = self.Host(tmp / "nowhere", fail=99)
            out.name = "w2"
            self.assertEqual(distributed.retry_traces(tmp / "dist" / "ledger.json", tmp / "selfplay", tmp / "dist",
                                                      {"w2": out}, self.logs.append)["failed"], [elsewhere["id"]])

    def test_a_rename_that_fails_is_a_fetch_that_failed_not_an_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            there = tmp / "worker" / "traces"
            there.mkdir(parents=True)
            (there / "selfplay-l0-4242.jsonl").write_text('{"seed":1}\n')
            host = self.Host(there)
            a = dict(id="w1-a1", host="w1")

            def refused(src, dst):
                raise OSError(13, "Permission denied")

            with unittest.mock.patch.object(distributed.os, "replace", refused):
                why = distributed.take_traces(a, host, tmp / "selfplay" / "traces", tmp / "incoming", self.logs.append)
            self.assertIn("Permission denied", why)
            self.assertIn("Permission denied", a["traces_failed"])
            self.assertEqual(host.asked, 3)

    def test_traces_that_cannot_be_fetched_are_said_and_recorded_and_fail_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            host = self.Host(tmp / "nowhere", fail=99)
            a = dict(id="w1-a1", host="w1")
            self.coordinator(tmp, host).take_traces(a, host)
            self.assertIn("the copy failed", a["traces_failed"])
            self.assertTrue(any("WARNING" in line and "cannot be" in line for line in self.logs))
            self.assertEqual(host.asked, 3)


def corpus(root: Path, gen: int, simulations: int = 131072, complete: bool = True, seeds=(1,), keys=(), targets: int = 90,
           **record) -> Path:
    """A corpus generation as the stage leaves it: this encoding's, its held-out games named (`keys`)."""
    d = root / reanalyse.CORPUS / f"gen{gen}"
    for seed in seeds:
        for split in ("train", "heldout"):
            (d / f"seed-{seed}" / split).mkdir(parents=True)
            (d / f"seed-{seed}" / split / "mine-0.shard.zst").write_text("x")
            mining.write_effort(d / f"seed-{seed}" / split, simulations, 16)
    if keys is not None:
        (d / reanalyse.KEYS).write_text(json.dumps(sorted(keys)))
    (d / reanalyse.RECORD).write_text(json.dumps(dict(dict(
        generation=gen, complete=complete, judge=f"gen{gen}", simulations=simulations, worlds=16,
        schema=dataset.LAYOUT["schema"], encoder_hash=dataset.encoder_hash(), mode="decklist",
        directories={f"seed-{s}": dict(train=targets, heldout=10) for s in seeds}), **record)))
    return d


def weights(dirs: list[str], root: Path) -> dict:
    return {Path(x.rsplit(":", 1)[0]).relative_to(root / reanalyse.CORPUS).as_posix(): float(x.rsplit(":", 1)[1]) for x in dirs}


class Corpus(unittest.TestCase):
    def test_training_takes_every_earlier_generation_whole_weighted_by_effort_and_age(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            corpus(root, 3)
            corpus(root, 9, simulations=32768)
            corpus(root, 10, seeds=(1, 2))
            corpus(root, 11)                       # this generation's own, made after its training
            corpus(root, 8, complete=False)        # a stage that did not finish
            said = []
            dirs, left = reanalyse.corpus_dirs(root, 11, 8192, decay=1.0, log=said.append)
            # Self-play's full search is 8192: a 131,072 search's targets count 16 times, a 32,768 one's 4, and
            # with no decay whatever the generation's age (gen3 is long out of any window).
            self.assertEqual(weights(dirs, root), {"gen3/seed-1/train": 16.0, "gen9/seed-1/train": 4.0,
                                                  "gen10/seed-1/train": 16.0, "gen10/seed-2/train": 16.0})
            self.assertEqual(left, [])
            self.assertTrue(any("90 targets at 16" in line for line in said), said)
            self.assertTrue(any("no cap" in line for line in said), "nothing known of the window: said")
            # The default: 0.85 a generation of age. gen10 is the newest at 11 (age 0), gen9 one older, gen3 seven.
            got = weights(reanalyse.corpus_dirs(root, 11, 8192, log=lambda *_: None)[0], root)
            self.assertEqual(got["gen10/seed-1/train"], 16.0)
            self.assertAlmostEqual(got["gen9/seed-1/train"], 4.0 * 0.85)
            self.assertAlmostEqual(got["gen3/seed-1/train"], 16.0 * 0.85 ** 7)
            # A weight of its own times the effort and the age.
            got = weights(reanalyse.corpus_dirs(root, 11, 8192, weight=0.5, decay=0.5, log=lambda *_: None)[0], root)
            self.assertEqual((got["gen10/seed-1/train"], got["gen9/seed-1/train"]), (8.0, 1.0))

    def test_the_corpus_is_capped_at_its_share_of_the_policy_target_weight(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            corpus(root, 5, targets=1000)
            corpus(root, 6, targets=1000)
            # Uncapped at gen7: 1,000 x 16 and 1,000 x 16 x 0.85 = 29,600 weighted targets.
            said = []
            dirs, _ = reanalyse.corpus_dirs(root, 7, 8192, log=said.append, share=0.25, window_targets=60_000)
            got = weights(dirs, root)
            # A quarter of the whole beside 60,000 window targets is 20,000: every weight scaled by 20,000 / 29,600.
            total = 1000 * got["gen5/seed-1/train"] + 1000 * got["gen6/seed-1/train"]
            self.assertAlmostEqual(total, 20_000, places=3)
            self.assertAlmostEqual(total / (total + 60_000), 0.25)
            self.assertAlmostEqual(got["gen5/seed-1/train"] / got["gen6/seed-1/train"], 0.85, msg="their proportions are kept")
            self.assertTrue(any("25.0% of the policy-target weight" in line and "capped at --corpus-share 0.25" in line
                                for line in said), said)
            self.assertTrue(any("capped x0.676" in line for line in said), "each directory's effective weight is said")
            # Under the cap nothing is scaled; with no cap (0) or nothing known of the window, neither.
            for share, window in ((0.25, 600_000), (0.0, 60_000), (0.25, 0), (0.25, None)):
                got = weights(reanalyse.corpus_dirs(root, 7, 8192, log=lambda *_: None, share=share, window_targets=window)[0], root)
                self.assertEqual(got["gen6/seed-1/train"], 16.0)

    def test_a_windows_policy_targets_are_its_full_searches_decisions(self):
        with tempfile.TemporaryDirectory() as tmp:
            d = Path(tmp) / "selfplay"
            d.mkdir()
            # Five searched decisions, three of them fast searches (value targets alone); and a game with no search.
            write_shard(d / "a.shard.zst", "selfplay", [
                (dict(seed=1, root_values=[[1, 0.1], [2, 0.2], [5, 0.3], [7, 0.1], [9, 0.0]], fast_decisions=[1, 2, 9]),
                 [sample(1, 0, [(0, 1.0)])]),
                (dict(seed=2), [sample(1, 0, [(0, 1.0)])])])
            # A reanalysed game's decisions are all policy targets.
            write_shard(d / "b.shard.zst", "deep", [(dict(seed=3, reanalysed=True, root_values=[[4, 0.5], [8, 0.5]]),
                                                    [sample(4, 0, [(0, 1.0)])])])
            self.assertEqual(reanalyse.policy_targets(d), 2 + 0 + 2)
            (d / "c.shard.zst").write_text("not a shard")
            self.assertEqual(reanalyse.policy_targets(d), 4, "a shard that does not read counts nothing")

    def test_a_generation_is_read_by_the_loaders_own_rule(self):
        now = (dataset.LAYOUT["schema"], dataset.encoder_hash())
        self.assertIsNone(reanalyse.readable(dict(schema=now[0], encoder_hash=now[1], mode="decklist")))
        # An earlier encoding this one only added to reads as this one (`dataset.UPGRADABLE`), as the loader reads it.
        for (schema, encoder), reads_as in dataset.UPGRADABLE.items():
            self.assertEqual(reanalyse.readable(dict(schema=schema, encoder_hash=encoder)) is None, reads_as == now[0])
        self.assertIn("does not read as this one", reanalyse.readable(dict(schema=now[0], encoder_hash="0123456789abcdef")))
        self.assertIsNone(reanalyse.readable(dict(judge="gen3")), "a record that names no encoding is the loader's to judge")
        like = dict(schema=now[0], encoder_hash=now[1], mode="full")
        self.assertEqual(reanalyse.readable(dict(schema=now[0], encoder_hash=now[1], mode="decklist"), like),
                         "its mode is decklist, this generation's full")
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            corpus(root, 3, encoder_hash="0123456789abcdef")
            corpus(root, 4)
            dirs, left = reanalyse.corpus_dirs(root, 6, 8192, log=lambda *_: None)
            self.assertEqual(list(weights(dirs, root)), ["gen4/seed-1/train"])
            self.assertEqual(len(left), 1)
            self.assertIn("the deep corpus of gen3 is not trained on: its encoding is", left[0])
            self.assertEqual(reanalyse.heldout_dirs(root, 6), [root / reanalyse.CORPUS / "gen4" / "seed-1" / "heldout"])

    def test_the_held_out_slices_are_those_of_the_generations_trained_beside_whose_games_are_named(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            corpus(root, 3, keys=[11, 12])
            corpus(root, 4, keys=None)            # made before held-out games were named: no clean measure
            corpus(root, 5, keys=[13])
            self.assertEqual([d.parts[-3] for d in reanalyse.heldout_dirs(root, 5)], ["gen3"])
            self.assertEqual([d.parts[-3] for d in reanalyse.heldout_dirs(root, 6)], ["gen3", "gen5"])
            self.assertEqual(reanalyse.heldout_dirs(root, 3), [])
            # Every generation's named games are left out of training, this generation's own too (a retrain's).
            files = reanalyse.exclusions(root)
            self.assertEqual([f.parent.name for f in files], ["gen3", "gen5"])
            self.assertEqual(dataset.read_keys(files), frozenset({11, 12, 13}))
            self.assertIsNone(dataset.read_keys([]))

    def test_the_held_out_games_are_named_by_their_source_and_never_loaded(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            metas = [dict(seed=70 + k, decks=["red", "blue"], names=["a", "b"]) for k in range(4)]
            selfplay = root / "gen7" / "selfplay"
            selfplay.mkdir(parents=True)
            write_shard(selfplay / "s.shard.zst", "selfplay", [(m, [sample(1, 0, [(0, 1.0)]), sample(2, 1, [(1, 1.0)])]) for m in metas])
            # The corpus's held-out slice holds the deep targets of game 71: a reanalysed game keeps its source's meta.
            held = root / "corpus" / "deep" / "gen7" / "seed-1" / "heldout"
            held.mkdir(parents=True)
            write_shard(held / "mine.shard.zst", "deep", [(dict(metas[1], reanalysed=True, root_values=[[1, 0.2]]),
                                                          [sample(1, 0, [(0, 1.0)])])])
            keys = reanalyse.heldout_keys([held])
            self.assertEqual(keys, {dataset.source_key(metas[1])})
            (held / "bad.shard.zst").write_text("not a shard")
            with self.assertRaises(SystemExit):
                reanalyse.heldout_keys([held])        # a game left unnamed would be trained on
            # The window's self-play without it, wherever the game is; every other game as before.
            whole = dataset.load([str(selfplay)], log=lambda *_: None, workers=0)
            less = dataset.load([str(selfplay)], log=lambda *_: None, workers=0, exclude=frozenset(keys))
            self.assertEqual((len(whole), len(less)), (8, 6))
            self.assertEqual(len(np.unique(less.game)), 3)
            self.assertEqual(dataset._count_games(selfplay / "s.shard.zst", None, frozenset(keys)), 3)
            self.assertEqual(dataset._count_games(selfplay / "s.shard.zst", None), 4)

    def test_retention_never_touches_the_corpus(self):
        from cantrip_ai import retention  # noqa: PLC0415
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for g in range(1, 6):
                (root / f"gen{g}" / "selfplay").mkdir(parents=True)
                (root / f"gen{g}" / "reanalyse").mkdir()
                corpus(root, g)
            doomed = retention.retire(root, keep=2, log=lambda *_: None)
            self.assertTrue(doomed)
            self.assertFalse((root / "gen1" / "reanalyse").exists(), "the stage's working files go with the generation")
            for g in range(1, 6):
                self.assertTrue((root / reanalyse.CORPUS / f"gen{g}" / "seed-1" / "train" / "mine-0.shard.zst").exists())


class Driver(unittest.TestCase):
    """The stage in the driver: where it runs, what it is given, what its failure means, and the corpus in training."""

    def fake(self, seen: list):
        class Fake:
            def __init__(self, root, gen, settings, model, decks, explore, setup, call, engine=None, log=print, **_):
                seen.append(dict(gen=gen, settings=settings, model=model, local=setup.local, workers=setup.workers, engine=engine))

            def run(self):
                return dict(corpus="corpus/deep/gen2", judge="gen2", source="selfplay", simulations=131072, worlds=16, seeds=[1],
                            planned=300, positions=280, searched=280, disagree=61, kl_median=0.103, memory_bound=1,
                            mismatched=0, closed="the time budget (4 h) is spent", seconds_judging=14400.0,
                            seconds_a_position=9.5, directories={"seed-1": dict(train=250, heldout=30)}, lanes={"w": 24})
        return Fake

    def test_the_stage_runs_after_the_miner_and_is_recorded(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            recordings(root / "gen2" / "selfplay", [1, 2])
            seen = []
            with unittest.mock.patch.object(reanalyse, "Reanalysis", self.fake(seen)):
                quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine", "--selfplay-traces", "--reanalyse-hours", "4",
                         "--reanalyse-positions", "300", "--reanalyse-seeds", "1,2", "--reanalyse-memory-mb", "9000",
                         "--no-judge-local"], run=lambda *a, **k: "")
            self.assertEqual(len(seen), 1)
            s = seen[0]["settings"]
            self.assertEqual((s.positions, s.hours, s.simulations, s.worlds, s.seeds, s.memory_mb, s.source, s.mode),
                             (300, 4.0, 131072, 16, (1, 2), 9000, "auto", "full"))
            self.assertEqual(seen[0]["model"].name, "gen2", "the champion the gate left: the candidate was accepted")
            self.assertTrue(seen[0]["local"], "with no workers asked for this machine judges, whatever --no-judge-local says")
            record = json.loads((root / "gen2" / "generation.json").read_text())
            self.assertEqual(record["stages"]["reanalyse"]["positions"], 280)
            self.assertLess(list(record["stages"]).index("promote"), list(record["stages"]).index("reanalyse"))
            self.assertLess(list(record["stages"]).index("reanalyse"), list(record["stages"]).index("retire"))
            readme = (root / "gen2" / "README.md").read_text()
            self.assertIn("| reanalyse |", readme)
            self.assertIn("280 of 300 positions of selfplay games searched by gen2 at 131072 simulations over 16 worlds", readme)
            self.assertIn("stopped: the time budget (4 h) is spent", readme)
            self.assertIn("9.5 s a position", readme)
            # Run again: a stage done is not made again; and a retrain does not set it aside.
            with unittest.mock.patch.object(reanalyse, "Reanalysis", self.fake(seen)):
                quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine", "--selfplay-traces",
                         "--reanalyse-hours", "4"], run=lambda *a, **k: "")
            self.assertEqual(len(seen), 1)
            self.assertNotIn("reanalyse", generation.LATER)
            self.assertIn("reanalyse", generation.DATA_STAGES)
            self.assertEqual(generation.STAGES[generation.STAGES.index("mine") + 1], "reanalyse")

    def test_without_its_flags_there_is_no_stage(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine"], run=lambda *a, **k: "")
            record = json.loads((root / "gen2" / "generation.json").read_text())
            self.assertNotIn("reanalyse", record["stages"])
            self.assertIn("finished", record)

    def test_this_machine_left_out_is_no_fallback_when_no_worker_is_leased(self):
        class NoLeases:
            named: list = []

            def __init__(self, *a, **k):
                self.workers = []

            def candidates(self):
                return []

            def held(self):
                import contextlib  # noqa: PLC0415
                return contextlib.nullcontext([])

        for flags, local in (([], True), (["--no-judge-local"], False)):
            with tempfile.TemporaryDirectory() as tmp:
                root = promoted_root(Path(tmp))
                recordings(root / "gen2" / "selfplay", [1, 2])
                seen = []
                with unittest.mock.patch.object(reanalyse, "Reanalysis", self.fake(seen)):
                    with unittest.mock.patch.object(distributed, "Leases", NoLeases):
                        quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine", "--selfplay-traces", "--workers",
                                 "all", "--reanalyse-hours", "4", "--no-preflight", "--no-dist-duels", *flags],
                                run=lambda *a, **k: "")
                self.assertEqual((seen[0]["local"], seen[0]["workers"]), (local, []))

    def test_a_failed_reanalysis_stops_the_generation_and_says_what_is_lost(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            # Asked to draw from self-play, whose traces never came: nothing to stand in again.
            with self.assertRaises(SystemExit) as caught:
                quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine", "--selfplay-traces",
                         "--reanalyse-positions", "40"], run=lambda *a, **k: "")
            why = str(caught.exception.code)
            self.assertIn("stage reanalyse failed, and its output is training data", why)
            self.assertIn("WITHOUT gen2's deep targets", why)
            self.assertIn("nothing to reanalyse", why)
            self.assertEqual(status.read(root, 2)["stage"], "reanalyse")
            # Allowed: recorded, flagged, and the generation goes on.
            quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine", "--selfplay-traces",
                     "--reanalyse-positions", "40", "--allow-data-failure"], run=lambda *a, **k: "")
            readme = (root / "gen2" / "README.md").read_text()
            self.assertRegex(readme, r"stage reanalyse failed and was passed over, so the next generations train WITHOUT this "
                                     r"generation's deep targets")
            # A missing part of the corpus is no reason to refuse the next generation's training (the mined targets' is).
            started(root, 3)
            quietly(["--root", str(root), "--generation", "3", *BASE, "--stop-after", "build"], run=lambda *a, **k: "")

    def test_a_generation_with_nothing_to_stand_in_again_is_refused_at_the_preflight(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            started(root, 3)
            ran = []
            with self.assertRaises(SystemExit) as caught:
                quietly(["--root", str(root), "--generation", "3", *BASE, "--no-mine", "--reanalyse-hours", "2"],
                        run=lambda cmd, *a, **k: ran.append(cmd) or "")
            self.assertIn("preflight failed; nothing was run", str(caught.exception.code))
            self.assertIn("no games it could stand in again", str(caught.exception.code))
            self.assertEqual(ran, [])
            with self.assertRaises(SystemExit) as caught:
                quietly(["--root", str(root), "--generation", "3", *BASE, "--reanalyse-hours", "2", "--reanalyse-model", "gen77",
                         "--preflight"], run=lambda *a, **k: "")
            self.assertIn("--reanalyse-model gen77: no model there", str(caught.exception.code))

    def test_training_takes_the_corpus_when_asked_and_only_then(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            started(root, 3)
            corpus(root, 1, simulations=640, keys=[5])
            corpus(root, 2, simulations=640, keys=[6])
            corpus(root, 3, simulations=640, keys=[7])     # this generation's own (a retrain's): never its own training's
            ran = []
            spy = lambda cmd, *a, **k: ran.append([str(c) for c in cmd]) or ""  # noqa: E731
            for flags, expected in (([], 0), (["--corpus-train", "--corpus-decay", "1"], 2),
                                    (["--corpus-train", "--corpus-weight", "0.5", "--corpus-decay", "0.5"], 2)):
                del ran[:]
                quietly(["--root", str(root), "--generation", "3", *BASE, "--no-mine", "--simulations", "64", "--dry-run",
                         *flags], run=spy)
                train = next(cmd for cmd in ran if "cantrip_ai.train" in cmd)
                took = [w for w in train if "corpus" in w and "deep" in w and Path(w.rsplit(":", 1)[0]).name == "train"]
                self.assertEqual(len(took), expected, train)
                if len(flags) == 3:
                    self.assertTrue(all(w.endswith(":10") for w in took), took)       # 640 simulations over 64
                    self.assertTrue(all(Path(w.rsplit(":", 1)[0]).name == "train" for w in took))
                if len(flags) > 3:
                    by_gen = {Path(w.rsplit(":", 1)[0]).parts[-3]: float(w.rsplit(":", 1)[1]) for w in took}
                    self.assertEqual(by_gen, {"gen1": 2.5, "gen2": 5.0})
            # The evaluation then scores the networks on the corpus's held-out slices.
            del ran[:]
            quietly(["--root", str(root), "--generation", "3", *BASE, "--no-mine", "--dry-run"], run=spy)
            scored = [cmd for cmd in ran if "cantrip_ai.value_eval" in cmd]
            self.assertEqual(len(scored), 1, "no permanent held-out set here: the corpus's slice alone")
            deep = scored[-1]
            self.assertIn("--every-game", deep)
            self.assertIn("--policy", deep)
            self.assertEqual(sum(1 for w in deep if w.endswith("heldout") and "corpus" in w), 2)
            self.assertTrue(deep[deep.index("--out") + 1].endswith("corpus-eval.json"))

    def test_a_corpus_evaluation_that_fails_is_its_own_warning_and_the_held_out_results_stand(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            corpus(root, 1, keys=[5])
            held = root / "heldout"
            held.mkdir()
            (held / "a.shard.zst").write_text("x")
            (held / generation.HELDOUT_RECORD).write_text("{}")
            result = dict(name="gen2", games=3, positions=30, value_mse=0.5, ece=0.1, sign=0.6, policy_ce=1.2, top1=0.4,
                          top1_played=0.4, choices=20, seats=[], by_game=None)

            def run(cmd, log_path, *a, **k):
                cmd = [str(c) for c in cmd]
                if "cantrip_ai.value_eval" in cmd:
                    out = Path(cmd[cmd.index("--out") + 1])
                    if out.name == "corpus-eval.json":
                        raise SystemExit("failed (1): value_eval\nthe shards' encoder hash is not ai/encoding.json's")
                    out.write_text(json.dumps([result, dict(result, name="gen0")]))
                return ""

            quietly(["--root", str(root), "--generation", "2", *BASE, "--no-mine"], run=run)
            record = json.loads((root / "gen2" / "generation.json").read_text())
            scored = record["stages"]["evaluate"]
            self.assertTrue(scored["done"], "the stage stands")
            self.assertEqual(scored["networks"]["gen2"]["policy_ce"], 1.2)
            self.assertIn("encoder hash", scored["corpus_failed"])
            self.assertNotIn("corpus", scored)
            self.assertIn("finished", record)
            readme = (root / "gen2" / "README.md").read_text()
            self.assertIn("> - **evaluate: the deep corpus's held-out slice was not scored: failed (1): value_eval", readme)
            self.assertIn("| evaluate | ", readme)

    def test_the_held_out_games_are_left_out_of_every_training_and_the_corpus_is_capped(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = promoted_root(Path(tmp))
            started(root, 3)
            corpus(root, 1, simulations=640, keys=[5, 6])
            corpus(root, 2, simulations=640, keys=[7])
            ran = []
            spy = lambda cmd, *a, **k: ran.append([str(c) for c in cmd]) or ""  # noqa: E731
            # With or without the corpus in training, its held-out games are left out.
            for flags in ([], ["--corpus-train"]):
                del ran[:]
                quietly(["--root", str(root), "--generation", "3", *BASE, "--no-mine", "--simulations", "64", "--dry-run",
                         *flags], run=spy)
                train = next(cmd for cmd in ran if "cantrip_ai.train" in cmd)
                at = train.index("--exclude-games")
                self.assertEqual([Path(f).parent.name for f in train[at + 1:at + 3]], ["gen1", "gen2"])
                self.assertTrue(all(Path(f).name == reanalyse.KEYS for f in train[at + 1:at + 3]))
            # The window's policy targets counted by decision: 90 x 10 + 90 x 10 x 0.85 = 1,665 corpus targets weighted
            # against 3,000 of the window's is capped at a quarter of the whole (1,000).
            with unittest.mock.patch.object(reanalyse, "policy_targets", lambda d: 3000 if d.parent.name == "gen3" else 0):
                del ran[:]
                out = quietly(["--root", str(root), "--generation", "3", *BASE, "--no-mine", "--simulations", "64", "--dry-run",
                               "--corpus-train"], run=spy)
            train = next(cmd for cmd in ran if "cantrip_ai.train" in cmd)
            got = {Path(w.rsplit(":", 1)[0]).parts[-3]: float(w.rsplit(":", 1)[1]) for w in train if "corpus" in w and ":" in w[3:]
                   and Path(w.rsplit(":", 1)[0]).name == "train"}
            self.assertAlmostEqual(90 * got["gen1"] + 90 * got["gen2"], 1000.0, places=3)
            self.assertAlmostEqual(got["gen1"] / got["gen2"], 0.85)
            self.assertIn("capped at --corpus-share 0.25", out)
            self.assertIn("left out of every directory: the deep corpus's held-out games", out)

    def test_what_a_stage_flags_while_it_runs_is_in_the_warning_block(self):
        d = dict(generation=4, started="then", stages={}, flagged=["self-play: w1-a1's traces were not fetched from w1 (x)"])
        self.assertEqual(generation.warnings(d), d["flagged"])
        self.assertIn("> - **self-play: w1-a1's traces were not fetched", generation.readme(d))

    def test_a_stop_asked_for_while_a_data_stage_judges_is_a_stop_not_a_failure(self):
        for flags in (["--selfplay-traces", "--reanalyse-positions", "40", "--no-mine"], []):
            with tempfile.TemporaryDirectory() as tmp:
                root = promoted_root(Path(tmp))
                recordings(root / "gen2" / "selfplay", [1, 2])
                (root / "gen2" / "logs").mkdir(exist_ok=True)
                (root / "gen2" / "logs" / "gate-results.tsv").write_text("")

                def stopped(*a, **k):
                    raise SystemExit("the judgements stopped: run the driver again to go on (ledger.json)")

                class Stopped:
                    def __init__(self, *a, **k):
                        pass

                    run = staticmethod(stopped)

                with unittest.mock.patch.object(reanalyse, "Reanalysis", Stopped):
                    with unittest.mock.patch.object(generation.mining, "clear", stopped):
                        with self.assertRaises(SystemExit) as caught:
                            quietly(["--root", str(root), "--generation", "2", *BASE, *flags], run=lambda *a, **k: "")
                why = str(caught.exception.code)
                self.assertTrue(why.startswith("the judgements stopped"), why)
                self.assertNotIn("its output is training data", why)
                said = status.read(root, 2)
                self.assertEqual((said["state"], status.judge(said)["state"]), ("stopped", "stopped"))
                self.assertEqual(said["stage"], "reanalyse" if flags else "mine")

    def test_the_readme_says_what_the_corpus_slice_scored(self):
        d = dict(generation=4, started="then", champion="gen3", stages=dict(evaluate=dict(
            done=True, seconds=60.0, corpus=dict(directories=["a", "b"], games=30, choices=57, networks={
                "gen4": dict(policy_ce=0.61, top1=0.72, choices=57), "gen3": dict(policy_ce=0.70, top1=0.66, choices=57)},
                paired=dict(of="gen4", against="gen3", policy_ce=dict(diff=-0.09, half=0.04, games=30, n=57))))))
        text = generation.readme(d)
        self.assertIn("the deep corpus's held-out slice (57 deep targets of 30 games, 2 directories): gen4: policy CE 0.6100, "
                      "top-1 0.720; gen3: policy CE 0.7000, top-1 0.660; gen4 less gen3, paired by game: policy CE -0.0900 +/- 0.0400", text)

    def test_what_the_stage_flags(self):
        d = dict(generation=4, started="then", stages=dict(reanalyse=dict(done=True, seconds=1.0, searched=100, memory_bound=9,
                                                                         mismatched=3)))
        flags = generation.warnings(d)
        self.assertTrue(any("9 of 100 deep searches stopped at their memory bound" in f for f in flags))
        self.assertTrue(any("3 positions were stood in as another position" in f for f in flags))
        d["stages"]["reanalyse"].update(memory_bound=2, mismatched=0)
        self.assertEqual(generation.warnings(d), [], "a few searches at their bound are no flag")


if __name__ == "__main__":
    unittest.main()
