"""`mining.py`: the pair miner as a stage of every generation (the parts that need no binaries)."""

import json
import tempfile
import unittest
from pathlib import Path

from cantrip_ai import mine, mining


def results(rows):
    head = "game\tpair\tshuffle\ta_seat\tdeck1\tdeck2\twinner\ta_points\tstarter\tturns\n"
    return head + "".join("\t".join(str(x) for x in r) + "\n" for r in rows)


class Mining(unittest.TestCase):
    def test_a_seed_fixed_duel_selects_every_decisive_pair_without_ids(self):
        rows = []
        for pair, points in enumerate([(1, 1), (0, 0), (1, 0), (0, 1)]):
            for k, a in enumerate(points):
                rows.append(dict(game=2 * pair + k, pair=pair, shuffle=100 + pair, a_seat=1 + k, deck1="x", deck2="y",
                                 winner=1, a_points=a, starter=1, turns=8))
        sel = mine.select(rows, 0, control=5, seed=1)
        self.assertEqual(sel["decisive"], [0, 1])
        self.assertEqual(sel["control"], [2, 3])
        self.assertEqual(sel["decisive_unknown_ids"], [])

    def test_a_pair_whose_replay_differs_or_is_missing_is_left_out(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = mining.Paths(Path(tmp))
            rows = [(g, g // 2, 100 + g // 2, 1 + g % 2, "x", "y", 1, 1 if g < 2 else 0, 1, 8) for g in range(6)]
            gate = Path(tmp) / "gate.tsv"
            gate.write_text(results(rows), encoding="utf-8")
            sel = mine.select(mine.read_results(gate), 0, control=0, seed=1)
            mine.write_selection(sel, p.selected)
            self.assertEqual(sel["decisive"], [0, 1, 2])
            p.replay.mkdir(parents=True)
            # Game 3 replayed differently, game 5 not replayed.
            changed = [r if r[0] != 3 else (3, 1, 101, 2, "x", "y", 2, 1, 1, 8) for r in rows if r[0] != 5]
            (p.replay / "first-results.tsv").write_text(results(changed), encoding="utf-8")
            verified = mine.verify(mine.read_results(gate), mine.read_results(p.replay / "first-results.tsv"))
            self.assertEqual(mining.prune_selection(p, verified), 2)
            self.assertEqual(mining.selection(p)["decisive"], [0])

    def test_the_plan_splits_into_forks_and_losers(self):
        with tempfile.TemporaryDirectory() as tmp:
            p = mining.Paths(Path(tmp))
            p.root.mkdir(parents=True)
            p.plan.write_text(json.dumps({"id": "a", "candidates": [1, 2]}) + "\n" + json.dumps({"id": "b", "dir": "first"}) + "\n",
                              encoding="utf-8")
            self.assertEqual(mining.split_plan(p), dict(forks=1, losers=1))
            self.assertIn('"candidates"', p.plan_forks.read_text(encoding="utf-8"))

    def test_mined_targets_are_weighted_by_effort_and_scaled_to_the_floor(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            fake = {}
            for g, name, sims in ((6, "forks-deep", 32768), (6, "losers", 8192), (5, "losers", 8192), (3, "losers", 8192)):
                d = root / f"gen{g}" / "mine" / "targets" / name
                d.mkdir(parents=True)
                (d / "x.shard.zst").write_bytes(b"")
                mining.write_effort(d, sims, 16)
                fake[str(d)] = 100
            real = mining.target_samples
            mining.target_samples = lambda d: fake[str(d)]
            try:
                # Effort alone: deep forks 8x, losers 2x, a generation older at half; gen3 is out of the window.
                got = mining.weighted_dirs(root, 7, 2, 4096, 0.0, 1_000_000, log=lambda *_: None)
                weights = {Path(x.rsplit(":", 1)[0]).parts[-4] + "/" + Path(x.rsplit(":", 1)[0]).name: float(x.rsplit(":", 1)[1]) for x in got}
                self.assertEqual(weights, {"gen6/forks-deep": 8.0, "gen6/losers": 2.0, "gen5/losers": 1.0})
                # A floor of 10%: mined weight 100 x (8 + 2 + 1) = 1,100 against 1,000,000 is scaled to 1/9 of it.
                got = mining.weighted_dirs(root, 7, 2, 4096, 0.10, 1_000_000, log=lambda *_: None)
                mined = sum(float(x.rsplit(":", 1)[1]) * 100 for x in got)
                self.assertAlmostEqual(mined / (mined + 1_000_000), 0.10, places=6)
                self.assertAlmostEqual(float(got[0].rsplit(":", 1)[1]) / float(got[1].rsplit(":", 1)[1]), 4.0)
            finally:
                mining.target_samples = real

    def test_judge_runs_are_the_forks_twice_and_the_losers_once(self):
        runs = mining.judge_runs(mining.Paths(Path("g")), mining.Settings(), "model", "decks", Path("here"), "judge")
        self.assertEqual([r["name"] for r in runs], ["forks-a", "forks-b", "forks-deep", "losers"])
        deep = runs[2]["args"]
        self.assertEqual((deep[deep.index("--simulations") + 1], deep[deep.index("--targets") + 1]), ("32768", "true"))
        self.assertIn("--targets", runs[0]["args"])
        self.assertEqual(runs[0]["args"][runs[0]["args"].index("--targets") + 1], "true")
        self.assertEqual(runs[1]["args"][runs[1]["args"].index("--targets") + 1], "false")


class FakeJudgeHost:
    """A host whose judgement parts (`mining.JUDGE`) judge their grant at once: a `ATTEMPT.judged` directory with its
    items, a line a planned position (an inference failure in `fail_once`'s games, once) and a target shard a game."""

    def __init__(self, name, base, plan, header, down=False):
        from cantrip_ai.distributed import Poll  # noqa: PLC0415
        self.Poll, self.name, self.remote, self.plan, self.header = Poll, name, True, plan, header
        self.dir = base / name
        self.dir.mkdir(parents=True)
        self.games = mining.plan_games(plan)
        self.runs, self.judged, self.fail_once = {}, [], set()

    def launch(self, a, grant):
        from cantrip_ai import distributed  # noqa: PLC0415
        items = distributed.from_ranges([[int(x.split("..")[0]), int(x.split("..")[1]) if ".." in x else int(x) + 1]
                                         for x in grant.split() if x != "end"])
        out = self.dir / f"{a['id']}.judged"
        (out / "train" / "first").mkdir(parents=True)
        lines = []
        for i in items:
            d, seed = self.games[i]
            bad = i in self.fail_once
            self.fail_once.discard(i)
            for pos in [json.loads(x) for x in self.plan.read_text().splitlines() if x.strip()]:
                if (pos["dir"], pos["seed"]) == (d, seed):
                    lines.append({"id": pos["id"], "judgement": {"verdict": "first"} if not bad else
                                  {"error": "the inference server closed"}})
            (out / "train" / "first" / f"mine-l0-{i}.shard.zst").write_text(str(i))
            if not bad:
                self.judged.append(i)
        (out / "items.txt").write_text("".join(f"{i}\n" for i in items))
        (out / "judged.jsonl").write_text("".join(json.dumps(x) + "\n" for x in lines))
        (out / "judge-report.json").write_text(json.dumps(dict(Simulations=int(self.header["simulations"]),
                                                               Worlds=int(self.header["worlds"]), Seed=int(self.header["seed"]),
                                                               Targets=self.header["targets"])))
        self.runs[a["id"]] = out.name
        return {}

    def grant(self, a, text):
        pass

    def poll(self, a):
        return self.Poll(status=None, exit=0, alive=False, shards=[self.runs[a["id"]]], started=True)

    def fetch(self, a, name, dest):
        import shutil  # noqa: PLC0415
        shutil.copytree(self.dir / name, dest)

    def fetch_logs(self, a, dest):
        pass

    def delete(self, a, names):
        pass

    def stop(self, a, now):
        pass

    def kill(self, a):
        pass


class DistributedJudgements(unittest.TestCase):
    def plan(self, tmp: Path) -> Path:
        lines = []
        for k, (d, seed) in enumerate([("second", 5), ("first", 9), ("first", 3), ("first", 9), ("second", 1)]):
            lines.append(json.dumps(dict(id=f"p{k:02}", dir=d, seed=seed, decision=k, split="train")))
        path = tmp / "plan-forks.jsonl"
        path.write_text("\n".join(lines) + "\n")
        return path

    def test_a_plans_games_are_numbered_as_mine_judge_numbers_them(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertEqual(mining.plan_games(self.plan(Path(tmp))), [("first", 3), ("first", 9), ("second", 1), ("second", 5)])

    def test_a_runs_games_are_judged_once_across_hosts_and_merged_as_one_machine_writes_them(self):
        from cantrip_ai import distributed  # noqa: PLC0415
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            plan = self.plan(tmp)
            games = len(mining.plan_games(plan))
            header = mining.judge_header(games, ["mine-judge", "--simulations", "64", "--worlds", "2", "--seed", "1",
                                                 "--targets", "true"])
            a = FakeJudgeHost("a", tmp, plan, header)
            b = FakeJudgeHost("b", tmp, plan, header)
            a.fail_once = {0, 1, 2, 3}  # its first part loses its positions to its server: refused, judged again
            led = distributed.Ledger(tmp / "ledger.json", games, 0)
            led.header = header
            out = tmp / "results"
            out.mkdir()
            logs = []
            c = distributed.Coordinator(led, [a, b], out, tmp / "dist", read=lambda path: mining.read_judged(path, games),
                                        log=logs.append, sleep=lambda s: None, kind=mining.JUDGE, chunk=1, initial=1,
                                        lookahead=0.0, max_failures=5)
            self.assertEqual(c.run(), "done")
            self.assertEqual(sorted(set(a.judged + b.judged)), list(range(games)))
            self.assertTrue(led.rejected and "inference server" in led.rejected[0]["why"])
            dest = tmp / "judge" / "forks-a"
            mining.merge_judged(led, out, dest)
            lines = [json.loads(x) for x in (dest / "judged.jsonl").read_text().splitlines()]
            self.assertEqual([x["id"] for x in lines], [f"p{k:02}" for k in range(5)])
            self.assertTrue(all(x["judgement"]["verdict"] == "first" for x in lines))
            self.assertEqual(len(list((dest / "train" / "first").glob("*.shard.zst"))), games)
            self.assertEqual(mining.check_judged(tmp / "judge", ["forks-a"])["forks-a"]["judged"], 5)

    def test_a_part_of_another_search_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            plan = self.plan(tmp)
            header = mining.judge_header(4, ["mine-judge", "--simulations", "64", "--worlds", "2", "--seed", "1", "--targets", "true"])
            host = FakeJudgeHost("a", tmp, plan, dict(header, simulations="128"))
            from cantrip_ai.distributed import Ledger  # noqa: PLC0415
            host.launch(dict(id="a-a1"), "0..4\n")
            got, items, _ = mining.read_judged(host.dir / "a-a1.judged", 4)
            self.assertEqual(items, [0, 1, 2, 3])
            led = Ledger(None, 4, 0)
            led.header = header
            self.assertIn("simulations", led.check(got))


if __name__ == "__main__":
    unittest.main()
