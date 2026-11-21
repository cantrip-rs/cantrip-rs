"""What a generation's record says beyond its summary numbers (docs/ai-training.md, "Reading a generation"): a paired
duel split by matchup, and the held-out set's value by deck seat and its paired differences. Both are read from what
the stages already write (a duel's results file, `value_eval`'s output) and kept in the stage's record in
`generation.json`; the README shows them as tables, and anything flagged at its top.

A candidate can lose its gate by a point or two with every summary number level, because one deck's seat went wrong:
the gate's share is then far below a half in that deck's matchups (its mirror most of all) and level elsewhere, and
the network's mean value for that seat is far from how often the seat wins. Neither shows in a total.

No torch and no numpy: the driver imports this.
"""

from __future__ import annotations

import math
from pathlib import Path

from . import duels

# A matchup is flagged when its paired share is more than this many standard errors below a half.
MATCHUP_Z = 3.0
# With no variance at all (every pair the same) there is no standard error: flagged from this many pairs, below a half.
MATCHUP_SAME_PAIRS = 10
# A seat is flagged when a network's mean value is further than this from the seat's mean outcome (both in -1 .. 1).
SEAT_GAP = 0.15


# --- A duel by matchup --------------------------------------------------------------------------------------------


def paired(totals: list[float]) -> dict:
    """Pairs summarised: `totals` are A's points in each pair (0, 0.5, 1, 1.5 or 2). The share is A's points over the
    pairs' two each; its standard error is the pairs' own spread (a pair is one observation: its two games share a
    deal); `half` the 95% half-width; `z` the share against a half in standard errors (None with no spread)."""
    n = len(totals)
    share = sum(totals) / (2 * n)
    var = sum((t / 2 - share) ** 2 for t in totals) / (n - 1) if n > 1 else 0.0
    se = math.sqrt(var / n)
    z = (share - 0.5) / se if se > 0 else None
    flagged = z < -MATCHUP_Z if z is not None else share < 0.5 and n >= MATCHUP_SAME_PAIRS
    return dict(pairs=n, lost_both=sum(t == 0 for t in totals), split=sum(0 < t < 2 for t in totals),
                won_both=sum(t == 2 for t in totals), share=round(share, 4), half=round(1.96 * se, 4),
                z=None if z is None else round(z, 2), flagged=bool(flagged))


def matchups(paths) -> dict | None:
    """A paired duel by matchup, from its results files (`duels.COLUMNS`: BotLab's `--results`, a distributed duel's
    merged file, or its hosts' own files, with or without their `#` header line; a game named in more than one file
    counts once, and a pair's two games may be in different files). A matchup is the two decks of a pair's deal,
    whichever seat holds which (A plays both sides of it, one game each); a mirror is a deck against itself.

    `rows` has the whole duel (`all`), the mirrors and the non-mirrors together, then each matchup, each as `paired`
    gives it; `incomplete` counts the pairs without two finished games, which are in no row. None with no whole pair."""
    games: dict[int, list[str]] = {}
    for path in paths:
        for line in duels.read_rows(Path(path))[1]:
            if duels.finished(line):
                games.setdefault(duels.game_of(line), line.split("\t"))
    by_pair: dict[int, list] = {}
    for g in sorted(games):
        by_pair.setdefault(int(games[g][1]), []).append(games[g])
    by: dict[tuple, list[float]] = {}
    for pair in by_pair.values():
        if len(pair) == 2:
            by.setdefault(tuple(sorted(pair[0][4:6])), []).append(sum(float(g[7]) for g in pair))
    if not by:
        return None
    rows = [dict(matchup="all", **paired([t for v in by.values() for t in v]))]
    for name, mirror in (("mirrors", True), ("non-mirrors", False)):
        totals = [t for k, v in by.items() if (k[0] == k[1]) == mirror for t in v]
        if totals and len(by) > 1:
            rows.append(dict(matchup=name, **paired(totals)))
    rows += [dict(matchup=f"{a} vs {b}", **paired(by[a, b])) for a, b in sorted(by)]
    return dict(rows=rows, incomplete=sum(len(p) != 2 for p in by_pair.values()))


def matchup_flags(table: dict | None) -> list[str]:
    """The matchups of a duel far below a half, a line each (the whole duel's own share is the gate's to judge)."""
    return [f"{r['matchup']}: {r['share']:.1%} +/- {r['half']:.1%} by pair over {r['pairs']} pairs"
            + (f" (z {r['z']:+.1f})" if r.get("z") is not None else " (every pair alike)")
            for r in (table or {}).get("rows", [])[1:] if r.get("flagged")]


def matchup_table(table: dict) -> list[str]:
    """A duel by matchup as a Markdown table: A's paired share in each."""
    lines = ["| matchup | pairs | paired share | z | lost both / split / won both | |", "|---|---|---|---|---|---|"]
    for r in table["rows"]:
        lines.append(f"| {r['matchup']} | {r['pairs']} | {r['share']:.1%} +/- {r['half']:.1%} | "
                     f"{'-' if r.get('z') is None else format(r['z'], '+.1f')} | "
                     f"{r['lost_both']} / {r['split']} / {r['won_both']} | {'**LOW**' if r.get('flagged') else ''} |")
    if table.get("incomplete"):
        lines += ["", f"{table['incomplete']} pair(s) without two finished games are in no row."]
    return lines


# --- The held-out set by seat, and by game ------------------------------------------------------------------------


def ratio_difference(a: dict, b: dict, total: str, count: str) -> dict | None:
    """A's mean less B's over the same games, paired by game: each is `value_eval`'s `by_game` (its `games`, and per
    game a `total` and a `count`: the mean is the totals' sum over the counts'). The 95% half-width is from the games'
    own differences (a game's positions are not independent: the game is the observation), the ratio's linearised.
    None where the two were not scored on the same games and counts, or on fewer than two."""
    if not a or not b or a.get(count) is None or a.get("games") != b.get("games") or a[count] != b.get(count):
        return None
    counts = a[count]
    n, games = sum(counts), sum(1 for c in counts if c)
    if games < 2 or n == 0:
        return None
    d = [x - y for x, y in zip(a[total], b[total])]
    diff = sum(d) / n
    var = games / (games - 1) * sum((x - diff * c) ** 2 for x, c in zip(d, counts)) / n ** 2
    return dict(diff=diff, half=1.96 * math.sqrt(var), games=games, n=n)


def heldout_pairs(results: list[dict]) -> dict | None:
    """The first network (the candidate) less the second (the champion) on the held-out set, paired by game: value MSE
    over every position, policy CE over the full-search targets. None without two networks scored by game."""
    if len(results) < 2:
        return None
    a, b = results[0].get("by_game"), results[1].get("by_game")
    out = dict(value_mse=ratio_difference(a, b, "squared_error", "positions"),
               policy_ce=ratio_difference(a, b, "policy_ce", "choices"))
    if all(v is None for v in out.values()):
        return None
    return dict(out, of=results[0]["name"], against=results[1]["name"])


def heldout_seats(results: list[dict]) -> list[dict]:
    """The held-out set by the seat's deck: its positions and mean outcome, and each network's mean value there (with
    a heuristic head, `model.Config.residual`, that head's mean log-odds and the residual's); `flagged` names the
    networks whose mean value is more than `SEAT_GAP` from the mean outcome."""
    out: dict[str, dict] = {}
    for r in results:
        for s in r.get("seats") or []:
            row = out.setdefault(s["deck"], dict(deck=s["deck"], positions=s["positions"], games=s.get("games"),
                                                 outcome=s["outcome"], networks={}, flagged=[]))
            row["networks"][r["name"]] = {k: s.get(k) for k in ("value", "heuristic", "residual")}
            if abs(s["value"] - s["outcome"]) > SEAT_GAP:
                row["flagged"].append(r["name"])
    return [out[k] for k in sorted(out)]


def seat_flags(seats: list[dict]) -> list[str]:
    """The seats a network misvalues, a line each."""
    return [f"{s['deck']}'s seat: {n} values it {s['networks'][n]['value']:+.2f} on average where its outcomes average "
            f"{s['outcome']:+.2f} ({s['positions']} positions)" for s in seats or [] for n in s.get("flagged", [])]


def _signed(v, digits=2):
    return "-" if v is None else format(v, f"+.{digits}f")


def seat_table(seats: list[dict]) -> list[str]:
    """The held-out set by seat as a Markdown table: mean value (heuristic head's log-odds / residual's) a network."""
    names = list(dict.fromkeys(n for s in seats for n in s["networks"]))
    parts = any(s["networks"][n].get("heuristic") is not None for s in seats for n in s["networks"])
    lines = ["| seat's deck | positions | mean outcome | " + " | ".join(
        f"{n}: mean value" + (" (heuristic head / residual, log-odds)" if parts else "") for n in names) + " | |",
             "|---|---|---|" + "---|" * len(names) + "---|"]
    for s in seats:
        cells = []
        for n in names:
            e = s["networks"].get(n)
            if e is None:
                cells.append("-")
            else:
                cells.append(_signed(e["value"]) + (f" ({_signed(e['heuristic'])} / {_signed(e['residual'])})"
                                                    if e.get("heuristic") is not None else ""))
        flag = "**OFF: " + ", ".join(s["flagged"]) + "**" if s.get("flagged") else ""
        lines.append(f"| {s['deck']} | {s['positions']} | {_signed(s['outcome'])} | " + " | ".join(cells) + f" | {flag} |")
    return lines


def pairs_text(p: dict | None) -> str:
    """The held-out set's paired differences in a clause, or nothing."""
    if not p:
        return ""
    parts = [f"{label} {d['diff']:+.4f} +/- {d['half']:.4f}" for label, key in (("value MSE", "value_mse"), ("policy CE", "policy_ce"))
             if (d := p.get(key))]
    games = next((d["games"] for d in (p.get("value_mse"), p.get("policy_ce")) if d), 0)
    return f"{p['of']} less {p['against']}, paired by game ({games} games; below zero is better): " + ", ".join(parts)
