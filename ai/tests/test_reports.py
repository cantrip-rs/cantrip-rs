"""What a generation's record says beyond its totals (`reports.py`): a paired duel by matchup, from its results files,
and the held-out set by the seat's deck and paired by game (`value_eval`).

    nix-shell ai/shell.nix --run "python -m unittest discover -s ai/tests"
"""

from __future__ import annotations

import contextlib
import gc
import io
import json
import math
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np

TESTS = Path(__file__).resolve().parent
sys.path.insert(0, str(TESTS.parent))
sys.path.insert(0, str(TESTS))
from cantrip_ai import duels, reports, train, value_eval  # noqa: E402
from test_dataset import sample, write_shard  # noqa: E402


def game(number: int, decks: tuple, points, seat: int = 1) -> str:
    """A results line (`duels.COLUMNS`): game `number` of pair `number // 2`; `points` None for one that did not finish."""
    done = points is not None
    return "\t".join(map(str, (number, number // 2, 7, seat, *decks, 1 if done else "-", points if done else "-", 1, 9)))


class MatchupTest(unittest.TestCase):
    def test_a_duel_by_matchup_over_its_hosts_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            a, b = Path(tmp) / "a.results.tsv", Path(tmp) / "b.results.tsv"
            header = "# " + json.dumps(dict(seed=5, games=24))
            # The red mirror: 12 pairs, 10 lost outright and 2 split. Pair 0's two games are in different files, and
            # game 2 is in both (counted once).
            mirror = [game(2 * k + j, ("red", "red"), 0 if k < 10 else j) for k in range(12) for j in (0, 1)]
            # Red against blue, the decks either way round: 3 pairs, won 2, 1.5 and 1; one more pair has a game that
            # did not finish, and one only a single game.
            cross = [game(24, ("red", "blue"), 1), game(25, ("red", "blue"), 1),
                     game(26, ("blue", "red"), 1), game(27, ("blue", "red"), 0.5),
                     game(28, ("red", "blue"), 1), game(29, ("red", "blue"), 0),
                     game(30, ("red", "blue"), 1), game(31, ("red", "blue"), None), game(32, ("blue", "red"), 1)]
            a.write_text("\n".join([header, duels.COLUMNS, mirror[0], *mirror[2:14], *cross[:4]]) + "\n", encoding="utf-8")
            b.write_text("\n".join([header, mirror[1], mirror[2], *mirror[14:], *cross[4:]]) + "\n", encoding="utf-8")
            table = reports.matchups([a, b])
        by = {r["matchup"]: r for r in table["rows"]}
        self.assertEqual(list(by), ["all", "mirrors", "non-mirrors", "blue vs red", "red vs red"])
        self.assertEqual(table["incomplete"], 2)
        red = by["red vs red"]
        self.assertEqual((red["pairs"], red["lost_both"], red["split"], red["won_both"]), (12, 10, 2, 0))
        share = 2 / 24
        se = math.sqrt((10 * share ** 2 + 2 * (0.5 - share) ** 2) / 11 / 12)
        self.assertAlmostEqual(red["share"], share, places=4)
        self.assertAlmostEqual(red["half"], 1.96 * se, places=4)
        self.assertAlmostEqual(red["z"], (share - 0.5) / se, places=2)
        self.assertTrue(red["flagged"])
        self.assertEqual(by["mirrors"], dict(red, matchup="mirrors"))
        cross = by["blue vs red"]
        self.assertEqual((cross["pairs"], cross["lost_both"], cross["split"], cross["won_both"], cross["share"]), (3, 0, 2, 1, 0.75))
        self.assertFalse(cross["flagged"])
        self.assertEqual(by["all"]["pairs"], 15)
        self.assertEqual(reports.matchup_flags(table),
                         [f"mirrors: 8.3% +/- {red['half']:.1%} by pair over 12 pairs (z {red['z']:+.1f})",
                          f"red vs red: 8.3% +/- {red['half']:.1%} by pair over 12 pairs (z {red['z']:+.1f})"])
        text = "\n".join(reports.matchup_table(table))
        self.assertIn(f"| red vs red | 12 | 8.3% +/- {red['half']:.1%} | {red['z']:+.1f} | 10 / 2 / 0 | **LOW** |", text)
        self.assertIn("2 pair(s) without two finished games are in no row.", text)

    def test_a_matchup_is_flagged_only_far_below_a_half(self):
        # Level, above, and a little below: not flagged. Three standard errors below: flagged.
        self.assertFalse(reports.paired([0, 2] * 50)["flagged"])
        self.assertFalse(reports.paired([2] * 30 + [1] * 5)["flagged"])
        self.assertFalse(reports.paired([0] * 12 + [2] * 10 + [1] * 20)["flagged"])
        low = reports.paired([0] * 30 + [2] * 10 + [1] * 60)
        self.assertEqual((low["share"], low["flagged"]), (0.4, True))
        self.assertLess(low["z"], -3)
        # Every pair alike has no spread to judge by: flagged from ten pairs lost, never when split.
        self.assertEqual((reports.paired([0] * 10)["z"], reports.paired([0] * 10)["flagged"]), (None, True))
        self.assertFalse(reports.paired([0] * 9)["flagged"])
        self.assertFalse(reports.paired([1] * 40)["flagged"])
        with tempfile.TemporaryDirectory() as tmp:
            empty = Path(tmp) / "r.tsv"
            empty.write_text(duels.COLUMNS + "\n" + game(0, ("red", "red"), 1) + "\n", encoding="utf-8")
            self.assertIsNone(reports.matchups([empty]), "no whole pair")


class HeldOutTest(unittest.TestCase):
    def test_seats_and_games_of_scored_positions(self):
        decks = [["red", "blue"], ["blue", "blue"], ["red", "blue"]]
        g = np.array([0, 0, 0, 1, 1, 2, 2, 2])
        seat = np.array([0, 1, 0, 0, 1, 0, 0, 1])
        outcome = np.array([1, -1, 1, 1, -1, -1, -1, 1], np.float32)
        value = np.array([-0.9, 0.1, -0.7, 0.2, -0.2, -0.8, -1.0, 0.3], np.float32)
        h, r = value * 4, value * 0 + 0.5
        rows = value_eval.seats(decks, g, seat, outcome, value, h, r)
        self.assertEqual([(s["deck"], s["positions"], s["games"]) for s in rows], [("blue", 4, 3), ("red", 4, 2)])
        red = rows[1]
        self.assertAlmostEqual(red["outcome"], 0.0)
        self.assertAlmostEqual(red["value"], -0.85, places=5)
        self.assertAlmostEqual(red["heuristic"], -3.4, places=5)
        self.assertAlmostEqual(red["residual"], 0.5)
        self.assertNotIn("heuristic", value_eval.seats(decks, g, seat, outcome, value)[0])
        # The other network values red's seat as its games end; the table flags the one that does not.
        other = value_eval.seats(decks, g, seat, outcome, value * 0 + 0.05)
        table = reports.heldout_seats([dict(name="gen3", seats=rows), dict(name="gen2", seats=other)])
        self.assertEqual([(s["deck"], s["flagged"]) for s in table], [("blue", []), ("red", ["gen3"])])
        self.assertEqual(table[1]["networks"]["gen2"], dict(value=other[1]["value"], heuristic=None, residual=None))
        self.assertEqual(reports.seat_flags(table),
                         ["red's seat: gen3 values it -0.85 on average where its outcomes average +0.00 (4 positions)"])
        text = "\n".join(reports.seat_table(table))
        self.assertIn("| red | 4 | +0.00 | -0.85 (-3.40 / +0.50) | +0.05 | **OFF: gen3** |", text)

        ce = np.arange(8, dtype=np.float64)
        choice = np.array([1, 0, 1, 1, 1, 0, 0, 1], bool)
        by = value_eval.by_game(g, (value - outcome) ** 2, ce, choice)
        self.assertEqual((by["games"], by["positions"], by["choices"]), ([0, 1, 2], [3, 2, 3], [2, 2, 1]))
        self.assertEqual(by["policy_ce"], [2.0, 7.0, 7.0])
        self.assertAlmostEqual(sum(by["squared_error"]), float(((value - outcome) ** 2).sum()), places=5)
        self.assertNotIn("policy_ce", value_eval.by_game(g, (value - outcome) ** 2, None, choice))

    def test_two_networks_are_paired_by_game(self):
        games = list(range(6))
        a = dict(games=games, positions=[10, 20, 10, 30, 10, 20], squared_error=[5.0, 12.0, 4.0, 18.0, 6.0, 9.0])
        b = dict(games=games, positions=a["positions"], squared_error=[4.0, 10.0, 4.5, 15.0, 5.0, 8.0])
        d = reports.ratio_difference(a, b, "squared_error", "positions")
        diffs = [x - y for x, y in zip(a["squared_error"], b["squared_error"])]
        mean = sum(diffs) / 100
        self.assertAlmostEqual(d["diff"], mean)
        self.assertAlmostEqual(d["diff"], sum(a["squared_error"]) / 100 - sum(b["squared_error"]) / 100, msg="the totals' difference")
        var = 6 / 5 * sum((x - mean * n) ** 2 for x, n in zip(diffs, a["positions"])) / 100 ** 2
        self.assertAlmostEqual(d["half"], 1.96 * math.sqrt(var))
        self.assertEqual((d["games"], d["n"]), (6, 100))
        # The same network against itself differs by nothing; other games, or no policy scored, pair nothing.
        self.assertEqual(reports.ratio_difference(a, a, "squared_error", "positions")["half"], 0.0)
        self.assertIsNone(reports.ratio_difference(a, dict(b, games=games[::-1]), "squared_error", "positions"))
        self.assertIsNone(reports.ratio_difference(a, b, "policy_ce", "choices"))
        pairs = reports.heldout_pairs([dict(name="gen3", by_game=a), dict(name="gen2", by_game=b)])
        self.assertEqual((pairs["of"], pairs["against"], pairs["policy_ce"]), ("gen3", "gen2", None))
        self.assertEqual(reports.pairs_text(pairs), f"gen3 less gen2, paired by game (6 games; below zero is better): "
                                                    f"value MSE {d['diff']:+.4f} +/- {d['half']:.4f}")
        self.assertIsNone(reports.heldout_pairs([dict(name="gen3", by_game=a)]))
        self.assertEqual(reports.pairs_text(None), "")


class ScoredTest(unittest.TestCase):
    """A tiny residual network scored on a tiny set: the seats and the games add up to the totals."""

    def test_a_scored_set_by_seat_and_by_game(self):
        with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
            root = Path(tmp)
            (root / "data").mkdir()
            games = []
            for g in range(16):
                samples = []
                for d in range(4):
                    s = bytearray(sample(d, (g + d) % 3, [((g + d) % 3, 0.75), ((g + d + 1) % 3, 0.25)], cards=(1, 2)))
                    s[0] = d % 2                        # the answering seat
                    s[9] = 1 if (g + d) % 2 == 0 else 255  # the outcome for it: won or lost
                    samples.append(bytes(s))
                decks = ["red", "blue"] if g % 2 else ["blue", "blue"]
                games.append((dict(seed=g, decks=decks, root_values=[[d, 0.1] for d in range(4)]), samples))
            write_shard(root / "data" / "a.shard.zst", "p", games, cards=["Opt", "Shock"])
            out = root / "out"
            with contextlib.redirect_stdout(io.StringIO()):
                train.main(["--data", str(root / "data"), "--out", str(out), "--steps", "2", "--batch", "4", "--width", "16",
                            "--layers", "1", "--heads", "1", "--ffn", "32", "--threads", "1", "--workers", "0", "--device", "cpu",
                            "--progress", "off", "--warmup", "1", "--held-out", "0.2", "--checkpoint-every", "0", "--residual"])
            gc.collect()
            import torch  # noqa: PLC0415

            r = value_eval.evaluate(f"net={out / 'checkpoint.pt'}", [str(root / "data")], 100, 0, 0.02, torch.device("cpu"),
                                    every_game=True, policy=True)
            gc.collect()
        self.assertEqual((r["games"], r["positions"]), (16, 64))
        seats = {s["deck"]: s for s in r["seats"]}
        self.assertEqual({k: (s["positions"], s["games"]) for k, s in seats.items()}, {"blue": (48, 16), "red": (16, 8)})
        for s in r["seats"]:
            self.assertTrue(-1 <= s["value"] <= 1 and math.isfinite(s["heuristic"]) and math.isfinite(s["residual"]))
        self.assertAlmostEqual(seats["red"]["outcome"], -1.0)  # red is seat 0 of the odd games, which it loses
        by = r["by_game"]
        self.assertEqual((len(by["games"]), sum(by["positions"]), sum(by["choices"])), (16, 64, r["choices"]))
        self.assertAlmostEqual(sum(by["squared_error"]) / 64, r["value_mse"], places=5)
        self.assertAlmostEqual(sum(by["policy_ce"]) / r["choices"], r["policy_ce"], places=4,
                               msg="each position's cross-entropy is train.py's")
        json.dumps(r)
        same = reports.heldout_pairs([r, dict(r, name="other")])
        self.assertEqual((same["value_mse"]["diff"], same["policy_ce"]["diff"]), (0.0, 0.0))


if __name__ == "__main__":
    unittest.main()
