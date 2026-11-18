"""The pair miner's driver (`cantrip_ai.mine`): which games' ids the range rule knows, the pairs chosen, the replay check,
the loser's decisions before its largest fall, the plan's held-out split by pair, and the report's tallies."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from cantrip_ai import mine  # noqa: E402


def row(game, a_points, winner="1", turns="9", starter="1"):
    return {"game": str(game), "pair": str(game // 2), "shuffle": str(100 + game // 2), "a_seat": "1" if game % 2 == 0 else "2",
            "deck1": "x", "deck2": "y", "winner": winner, "a_points": str(a_points), "starter": starter, "turns": turns}


# Two ranges of four games (0-3, 4-7); the results in the order they finished. Range 1 is done at place 6 (game 7), so
# games finishing at places 0-5 have known ids: 0, 4, 1, 5, 2, 6.
ORDER = [0, 4, 1, 5, 2, 6, 7, 3]
POINTS = {0: 0, 1: 0, 2: 1, 3: 0, 4: 1, 5: 1, 6: 1, 7: 0}


def results():
    return [row(g, POINTS[g]) for g in ORDER]


class RangeRule(unittest.TestCase):
    def test_ids_are_the_place_in_the_range_until_a_range_is_done(self):
        ids, done = mine.range_ids(results(), 2)
        self.assertEqual(done, 6)
        self.assertEqual(ids, {0: 1, 1: 2, 2: 3, 4: 1, 5: 2, 6: 3})

    def test_a_run_not_split_so_is_refused(self):
        rows = [row(g, POINTS[g]) for g in [1, 4, 0, 5, 2, 6, 7, 3]]
        with self.assertRaises(ValueError):
            mine.range_ids(rows, 2)
        with self.assertRaises(ValueError):
            mine.range_ids(results(), 3)

    def test_pairs_with_both_ids_known_are_chosen(self):
        sel = mine.select(results(), 2, control=5, seed=1)
        # Pair 0 (games 0, 1): A lost both. Pair 1 (2, 3): game 3's id is not known. Pair 2 (4, 5): A won both. Pair 3
        # (6, 7): game 7 not known.
        self.assertEqual(sel["decisive"], [0, 2])
        self.assertEqual(sel["control"], [])
        self.assertEqual(sel["decisive_unknown_ids"], [])
        self.assertEqual(sel["counts"]["a_lost_both"], 1)
        self.assertEqual(sel["counts"]["a_won_both"], 1)
        self.assertEqual(sel["ids"], {0: 1, 1: 2, 4: 1, 5: 2})


class Verify(unittest.TestCase):
    def test_every_field_must_match(self):
        gate = results()
        same = [dict(r) for r in gate[:3]]
        other = dict(gate[3], turns="10")
        res = mine.verify(gate, same + [other])
        self.assertEqual(res["same"], 3)
        self.assertEqual([d["game"] for d in res["differ"]], [int(other["game"])])


class Plan(unittest.TestCase):
    def test_the_decisions_before_the_largest_fall(self):
        values = [(1, 1, 0.1), (3, 1, 0.2), (5, 2, 0.15), (7, 2, -0.6), (9, 3, -0.5)]
        self.assertEqual(mine.largest_fall(values), [0, 1, 2])
        self.assertEqual(mine.largest_fall([(1, 1, 0.1), (2, 1, 0.3)]), [])
        self.assertEqual(mine.largest_fall([(1, 1, 0.1)]), [])

    def test_forks_and_losers_planned_held_out_by_pair(self):
        sel = mine.select(results(), 2, control=5, seed=1)
        forks = [{"seed": 100, "status": "fork", "decision": 12, "answers": ["Pass", {"Act": {}}]},
                 {"seed": 102, "status": "identical"}]
        values = [{"seed": 100, "seat": 0, "dir": "first", "values": [[1, 1, 0.5], [4, 1, 0.4], [6, 2, -0.5]]},
                  {"seed": 100, "seat": 1, "dir": "second", "values": [[2, 1, 0.0], [12, 2, -0.9]]}]
        positions = mine.plan(sel, forks, values, heldout=0.0, before=3, seed=1)
        keys = [(p["dir"], p["decision"], tuple(p["why"])) for p in positions]
        self.assertEqual(keys, [("first", 1, ("loser",)), ("first", 4, ("loser",)), ("first", 12, ("fork",)), ("second", 2, ("loser",))])
        self.assertTrue(all(p["split"] == "train" for p in positions))
        held = mine.plan(sel, forks, values, heldout=1.0, before=3, seed=1)
        self.assertTrue(all(p["split"] == "heldout" for p in held))
        # Held out by pair: every position of a pair in one split.
        for share in (0.15, 0.5):
            for seed in range(20):
                splits = {p["pair"]: set() for p in mine.plan(sel, forks, values, share, 3, seed)}
                for p in mine.plan(sel, forks, values, share, 3, seed):
                    splits[p["pair"]].add(p["split"])
                self.assertTrue(all(len(s) == 1 for s in splits.values()))

    def test_the_losers_seat_by_game(self):
        sel = mine.select(results(), 2, control=5, seed=1)
        games = mine.loser_games(sel, "a-model", "b-model")
        # Pair 0: A lost both: seat 1 (index 0) in the even game, seat 2 in the odd. Pair 2: B lost both.
        self.assertIn({"seed": 100, "seat": 0, "model": "a-model", "game": 0}, games["first"])
        self.assertIn({"seed": 100, "seat": 1, "model": "a-model", "game": 1}, games["second"])
        self.assertIn({"seed": 102, "seat": 1, "model": "b-model", "game": 4}, games["first"])
        self.assertIn({"seed": 102, "seat": 0, "model": "b-model", "game": 5}, games["second"])


def world(qa, qb):
    return {"qa": qa, "qb": qb}


class Judgement(unittest.TestCase):
    def test_runs_are_pooled_world_by_world(self):
        one = {"verdict": "undecided", "worlds": [world(0.2, 0.1), world(0.1, 0.05), world(0.0, 0.1), world(0.3, 0.1)]}
        two = {"verdict": "first", "worlds": [world(0.2, 0.0), world(0.15, 0.05), world(0.1, 0.0), world(0.2, 0.1), world(None, 0.1)]}
        p = mine.pooled([one, two])
        self.assertEqual(p["paired"], 8, "a world with one value is no pair")
        self.assertEqual(p["runs"], ["undecided", "first"])
        self.assertAlmostEqual(p["difference"]["mean"], (0.1 + 0.05 - 0.1 + 0.2 + 0.2 + 0.1 + 0.1 + 0.1) / 8)
        self.assertEqual(p["verdict"], "first")
        self.assertEqual(mine.pooled([{"verdict": "unjudged", "error": "no search"}])["verdict"], "unjudged")
        flat = mine.pooled([{"verdict": "undecided", "worlds": [world(0.1, 0.0), world(0.0, 0.1)]}])
        self.assertEqual(flat["verdict"], "undecided")

    def test_the_interval_is_students_then_the_normal(self):
        m, lo, hi = mine.mean_interval([1.0, 2.0, 3.0])
        self.assertAlmostEqual(m, 2.0)
        self.assertAlmostEqual(hi - m, 4.303 * 1.0 / 3 ** 0.5, places=6)
        m, lo, hi = mine.mean_interval([float(i % 2) for i in range(100)])
        self.assertAlmostEqual(hi - m, 1.96 * (sum((i % 2 - 0.5) ** 2 for i in range(100)) / 99) ** 0.5 / 10, places=6)


class Report(unittest.TestCase):
    def setUp(self):
        self.sel = mine.select(results(), 2, control=5, seed=1)
        self.forks = [{"seed": 100, "status": "fork", "decision": 12, "seat": 1, "turn": 3, "kind": "Priority",
                       "labels": ["cast it", "pass"], "answers": [1, 2]},
                      {"seed": 102, "status": "fork", "decision": 5, "seat": 0, "turn": 1, "kind": "Priority",
                       "labels": ["Play Island", "Play Island"], "answers": [3, 4]}]
        better_second = {"verdict": "second", "tokens": [["a", "b"], ["a", "c"]], "parted_at": 1, "labels": ["cast it", "pass"],
                         "worlds": [world(0.1, 0.3), world(0.0, 0.25), world(0.05, 0.2)], "q": [0.05, 0.25],
                         "difference": {"mean": -0.2, "low": -0.3, "high": -0.1}, "paired": 3}
        same = {"verdict": "undecided", "tokens": [["x"], ["y"]], "parted_at": 0, "labels": ["Play Island", "Play Island"],
                "worlds": [world(0.1, 0.1), world(0.2, 0.2)]}
        self.run = {"p00000-first-0012": {"id": "p00000-first-0012", "judgement": better_second,
                                          "target": {"chosen_tokens": ["a", "c", "d"], "chosen": "pass", "root_value": 0.3,
                                                     "net_value": 0.2, "kl": 0.1}},
                    "p00002-first-0005": {"id": "p00002-first-0005", "judgement": same}}

    def test_who_answered_and_who_was_right(self):
        rows = mine.merge(self.sel, self.forks, [self.run], [])
        # Pair 0, seat 2 of the even game is B's; the second answer (the odd game's) is A's, and it was the better.
        self.assertEqual(rows[0]["who"], ("B", "A"))
        self.assertEqual(rows[0]["better"], "A")
        self.assertEqual(rows[0]["agree"], {"B": False, "A": True})
        self.assertTrue(rows[1]["equivalent"])
        text = mine.report(self.sel, rows, [], {"A": "gen5", "B": "gen4", "cross": "gen5"}, None, {}, None, self.forks)
        self.assertIn("The 10 most decisive forks", text)
        self.assertIn("gen5 played: pass", text)

    def test_decided_forks_and_far_loser_targets_are_kept(self):
        rows = mine.merge(self.sel, self.forks, [self.run], [])
        plan = [{"id": "p00000-first-0012", "dir": "first", "seed": 100, "decision": 12, "split": "train"},
                {"id": "p00002-first-0005", "dir": "first", "seed": 102, "decision": 5, "split": "train"},
                {"id": "p00000-second-0004", "dir": "second", "seed": 100, "decision": 4, "split": "heldout"},
                {"id": "p00000-second-0007", "dir": "second", "seed": 100, "decision": 7, "split": "heldout"}]
        losers = [{"id": "p00000-second-0004", "target": {"kl": 0.5}}, {"id": "p00000-second-0007", "target": {"kl": 0.01}},
                  {"id": "p00000-first-0012", "target": {"kl": 9.0}}]
        kept = mine.keep(rows, losers, plan, 0.1)
        self.assertEqual([x["id"] for x in kept["train"]], ["p00000-first-0012"], "the decided fork, not the equivalent one")
        self.assertEqual([x["id"] for x in kept["heldout"]], ["p00000-second-0004"], "the loser decision far from the prior")


if __name__ == "__main__":
    unittest.main()
