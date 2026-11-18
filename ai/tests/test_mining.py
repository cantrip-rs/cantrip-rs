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

    def test_the_window_takes_the_last_generations_targets_each_older_at_half(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            for g in (3, 5, 6):
                d = root / f"gen{g}" / "mine" / "targets" / "forks-a"
                d.mkdir(parents=True)
                (d / "x.shard.zst").write_bytes(b"")
            got = mining.window_dirs(root, 7, 2, 8.0)
            self.assertEqual([Path(x.rsplit(":", 1)[0]).parts[-4] for x in got], ["gen6", "gen5"])
            self.assertEqual([x.rsplit(":", 1)[1] for x in got], ["8", "4"])
            self.assertEqual(mining.window_dirs(root, 7, 0, 8.0), [])

    def test_judge_runs_are_the_forks_twice_and_the_losers_once(self):
        runs = mining.judge_runs(mining.Paths(Path("g")), mining.Settings(), "model", "decks", Path("here"), "judge")
        self.assertEqual([r["name"] for r in runs], ["forks-a", "forks-b", "losers"])
        self.assertIn("--targets", runs[0]["args"])
        self.assertEqual(runs[0]["args"][runs[0]["args"].index("--targets") + 1], "true")
        self.assertEqual(runs[1]["args"][runs[1]["args"].index("--targets") + 1], "false")


if __name__ == "__main__":
    unittest.main()
