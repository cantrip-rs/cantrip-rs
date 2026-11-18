"""The pair miner: training targets from a duel played in pairs, where a deep search says which move is better.

A duel plays games in pairs (BotLab: one deal, the players exchanging seats, each deck staying with its seat). A pair one
side wins both games of points at decisions, not luck. The targets come from a deep search with hidden information
(`cantrip-explore mine-judge`, crates/cantrip-ai-host/src/mine.rs), never from game outcomes: outcomes only choose which
games are looked at.

    python -m cantrip_ai.mine select --results FILE --out DIR [--ranges 48] [--control 200] [--seed N]
    python -m cantrip_ai.mine verify --gate FILE --replayed FILE [FILE ...]
    python -m cantrip_ai.mine plan --selected DIR --forks FILE --values FILE [FILE ...] --out FILE
        [--heldout 0.15] [--before 3] [--seed N]
    python -m cantrip_ai.mine keep --selected DIR --forks FILE --plan FILE --runs DIR [DIR ...] --losers DIR --out DIR
        [--kl-min 0.1]
    python -m cantrip_ai.mine report --selected DIR --forks FILE --runs DIR [DIR ...] [--cross DIR ...] [--losers DIR]
        [--replay FILE] [--settings FILE] [--kept FILE] --out FILE

`select` reads a duel's `--results` (BotLab's, a line a game in the order the games finished) and chooses the pairs to
mine: every decisive pair (one side won both games) and a random sample of split pairs as a control. A neural seat's
search seed is made from its table's game id, which BotLab's lanes number from 1 in the order each deals its games, and
lanes are reused game after game: a game is replayed exactly only with the id it had. BotLab's `Parallel.For` splits
the run's games into `--ranges` contiguous ranges, each worker playing its own range in order with its own lane until
it runs out and takes work from another range; so until the first worker finishes its range, the game `j` places into
a range was its lane's game `j + 1`. Only games that finished before then (by the results' order) are chosen: a
game that finished later may have started after a worker crossed ranges, and its id is not known. The game ids go to
`ids.txt` (BotLab's `--game-ids`), the games to replay to `first.txt` (each pair's even game: A in seat 1) and
`second.txt` (its odd game).

`verify` compares replayed games' results with the duel's: winner, A's points, starter, turns, decks, seat and shuffle
must all be the same.

`plan` turns forks (`mine-forks`) and the losers' network values (`mine-values`) into positions for `mine-judge`: each
pair's fork with its two candidate answers, and in each game of a decisive pair the loser's `--before` decisions before
its largest fall of value (as `cantrip-reanalyse` chooses the loser's decisions from root values). A share `--heldout`
of the pairs (by a hash of the pair and `--seed`) is held out: its positions are written to the held-out split.

A fork is judged by `mine-judge` runs on independent seeds (`--runs`; the first also searched each position without a
floor, the target): their worlds' paired differences are pooled into one sample (`pooled`), and the verdict is that
sample's 95% interval. One run's interval alone is often on the boundary at 16 to 32 worlds, and a second seed moves a
near fork between decided and undecided.

`keep` names the positions written as targets (`mine-filter --keep`): every fork the pooled judgement decided, and
every loser decision whose deep target is further than `--kl-min` nats from the network's prior (KL(target || prior)).

`report` writes the deep-dive (REPORT.md): forks found and decided, who was right by pair type, request kinds and
turns, matchups, the most decisive forks, and each network's agreement with the deep search.
"""

from __future__ import annotations

import argparse
import collections
import csv
import hashlib
import json
import math
import random
from pathlib import Path

# A pair's A points: 0 A lost both, 2 A won both, 1 split.
DECISIVE = (0.0, 2.0)

KINDS = {
    "Priority": "priority (cast, activate, pass)",
    "DeclareAttackers": "attacks",
    "DeclareBlockers": "blocks",
    "ChooseTarget": "targets",
    "ChooseDividedTargets": "targets",
    "PayCosts": "payments",
    "UnlessPays": "payments",
    "Mulligan": "mulligans",
    "ChooseStartingPlayer": "play or draw",
    "ChooseCards": "card choices",
    "SearchLibrary": "card choices",
    "DiscardToHandSize": "card choices",
}


def read_results(path: Path) -> list[dict]:
    """BotLab's `--results`: a dict a game, in the file's order (the order the games finished)."""
    with open(path, newline="", encoding="utf-8") as f:
        return list(csv.DictReader(f, delimiter="\t"))


def pairs_of(rows: list[dict]) -> dict[int, dict]:
    """The pairs of a duel by number: their games (even first), shuffle, decks and A's points."""
    out: dict[int, dict] = {}
    for r in rows:
        p = out.setdefault(int(r["pair"]), {"pair": int(r["pair"]), "games": {}, "shuffle": int(r["shuffle"]),
                                             "decks": (r["deck1"], r["deck2"])})
        p["games"][int(r["game"])] = r
    for p in out.values():
        p["points"] = sum(float(g["a_points"]) for g in p["games"].values()) if len(p["games"]) == 2 else None
    return out


def range_ids(rows: list[dict], ranges: int) -> tuple[dict[int, int], int]:
    """Each game's table id under the range rule (see the module) for the games that finished before the first range
    was played out, and that place in the results; the run's games must split into `ranges` ranges of one size."""
    games = len(rows)
    if games % ranges:
        raise ValueError(f"{games} games do not split into {ranges} ranges")
    size = games // ranges
    place = {int(r["game"]): i for i, r in enumerate(rows)}
    if sorted(place) != list(range(games)):
        raise ValueError("the results are not every game of the run once")
    first_done = min(max(place[g] for g in range(k * size, (k + 1) * size)) for k in range(ranges))
    # The rule's own evidence: before the first range is played out, each range's games finish in order from its first
    # (one worker playing it game after game); otherwise the run was not split as the rule says.
    for k in range(ranges):
        done = [g for g in range(k * size, (k + 1) * size) if place[g] < first_done]
        if done != list(range(k * size, k * size + len(done))) or sorted(place[g] for g in done) != [place[g] for g in done]:
            raise ValueError(f"range {k}'s games did not finish in order from its first before the first range was done")
    ids = {g: g % size + 1 for g, i in place.items() if i < first_done}
    return ids, first_done


def select(rows: list[dict], ranges: int, control: int, seed: int) -> dict:
    """The pairs to mine (see the module): decisive pairs and a control of split pairs, both games' ids known. `ranges`
    0: the duel's searches are seeded by the game's number (BotLab since 2026-11-16, `NeuralPlay.SearchSeedFor`), so
    every game replays from its number alone and every pair is known; no ids are needed."""
    if ranges > 0:
        ids, first_done = range_ids(rows, ranges)
    else:
        ids, first_done = {int(r["game"]): None for r in rows}, None
    pairs = pairs_of(rows)
    known = {p for p, v in pairs.items() if v["points"] is not None and all(g in ids for g in v["games"])}
    decisive = sorted(p for p in known if pairs[p]["points"] in DECISIVE)
    split = sorted(p for p in known if pairs[p]["points"] == 1.0)
    rng = random.Random(seed)
    controls = sorted(rng.sample(split, min(control, len(split))))
    unknown = sorted(p for p, v in pairs.items() if v["points"] in DECISIVE and p not in known)
    return {
        "ranges": ranges,
        "first_range_done": first_done,
        "decisive": decisive,
        "control": controls,
        "decisive_unknown_ids": unknown,
        "ids": {g: ids[g] for p in decisive + controls for g in pairs[p]["games"]},
        "pairs": {p: {"shuffle": pairs[p]["shuffle"], "decks": pairs[p]["decks"], "points": pairs[p]["points"],
                      "games": sorted(pairs[p]["games"])} for p in decisive + controls},
        "counts": {"pairs": len(pairs), "known": len(known),
                   "decisive_all": sum(1 for v in pairs.values() if v["points"] in DECISIVE),
                   "a_lost_both": sum(1 for p in decisive if pairs[p]["points"] == 0.0),
                   "a_won_both": sum(1 for p in decisive if pairs[p]["points"] == 2.0)},
    }


def write_selection(sel: dict, out: Path):
    out.mkdir(parents=True, exist_ok=True)
    (out / "selection.json").write_text(json.dumps(sel, indent=1), encoding="utf-8")
    games = [g for p in sel["pairs"].values() for g in p["games"]]
    if sel["ranges"] > 0:
        (out / "ids.txt").write_text("".join(f"{g} {sel['ids'][g]}\n" for g in sorted(games)), encoding="utf-8")
    (out / "first.txt").write_text("".join(f"{g}\n" for g in sorted(games) if g % 2 == 0), encoding="utf-8")
    (out / "second.txt").write_text("".join(f"{g}\n" for g in sorted(games) if g % 2 == 1), encoding="utf-8")


def load_selection(out: Path) -> dict:
    sel = json.loads((out / "selection.json").read_text(encoding="utf-8"))
    sel["ids"] = {int(k): v for k, v in sel["ids"].items()}
    sel["pairs"] = {int(k): v for k, v in sel["pairs"].items()}
    return sel


FIELDS = ("pair", "shuffle", "a_seat", "deck1", "deck2", "winner", "a_points", "starter", "turns")


def verify(gate: list[dict], replayed: list[dict]) -> dict:
    """Each replayed game against the duel's line for it: the games whose every field matches, and those that do not."""
    by = {r["game"]: r for r in gate}
    same, differ = [], []
    for r in replayed:
        g = by.get(r["game"])
        (same if g is not None and all(g[k] == r[k] for k in FIELDS) else differ).append(
            {"game": int(r["game"]), "gate": {k: g[k] for k in FIELDS} if g else None, "replayed": {k: r[k] for k in FIELDS}})
    return {"games": len(replayed), "same": len(same), "differ": differ}


# ---------------------------------------------------------------------------------------------------------------------
# The plan

def models_of(game: int) -> tuple[str, str]:
    """Which player sat in each seat of game `game`: A in seat 1 on even games (BotLab's seating)."""
    return ("A", "B") if game % 2 == 0 else ("B", "A")


def held_out(pair: int, share: float, seed: int) -> bool:
    h = hashlib.sha256(f"{seed}:{pair}".encode()).digest()
    return int.from_bytes(h[:8], "little") / 2**64 < share


def largest_fall(values: list) -> list[int]:
    """Indices of the decisions before the largest fall between consecutive values (`[(decision, turn, value)]`), as
    `cantrip-reanalyse` takes the loser's: the decisions up to (not including) the one the value fell at; none if the
    values never fall."""
    if len(values) < 2:
        return []
    k = max(range(1, len(values)), key=lambda i: (values[i - 1][2] - values[i][2], -i))
    return [] if values[k - 1][2] <= values[k][2] else list(range(k))


def plan(sel: dict, forks: list[dict], values: list[dict], heldout: float, before: int, seed: int) -> list[dict]:
    """The positions for `mine-judge` (see the module), each with its pair and why, in a stable order."""
    by_seed = {p["shuffle"]: (pn, p) for pn, p in sel["pairs"].items()}
    positions: dict[tuple, dict] = {}

    def add(dir_, shuffle, decision, pair, why, candidates=None):
        key = (dir_, shuffle, decision)
        split = "heldout" if held_out(pair, heldout, seed) else "train"
        p = positions.setdefault(key, {"id": f"p{pair:05d}-{dir_}-{decision:04d}", "dir": dir_, "seed": shuffle,
                                       "decision": decision, "split": split, "pair": pair, "why": []})
        p["why"].append(why)
        if candidates is not None:
            p["candidates"] = candidates

    for f in forks:
        if f.get("status") != "fork":
            continue
        pair, _ = by_seed[f["seed"]]
        add("first", f["seed"], f["decision"], pair, "fork", f["answers"])
    for v in values:
        if "values" not in v:
            continue
        pair, p = by_seed[v["seed"]]
        if p["points"] not in DECISIVE:
            continue
        dir_ = v["dir"]
        chosen = largest_fall(v["values"])[-before:]
        for k in chosen:
            add(dir_, v["seed"], v["values"][k][0], pair, "loser")
    return sorted(positions.values(), key=lambda p: p["id"])


def loser_games(sel: dict, a_model: str, b_model: str) -> dict[str, list[dict]]:
    """For `mine-values`: in each game of each decisive pair, the loser's seat and model, by directory."""
    out: dict[str, list[dict]] = {"first": [], "second": []}
    for p in sel["pairs"].values():
        if p["points"] not in DECISIVE:
            continue
        loser = "A" if p["points"] == 0.0 else "B"
        for g in p["games"]:
            seats = models_of(g)
            seat = seats.index(loser)
            out["first" if g % 2 == 0 else "second"].append(
                {"seed": p["shuffle"], "seat": seat, "model": a_model if loser == "A" else b_model, "game": g})
    return out


# ---------------------------------------------------------------------------------------------------------------------
# The report

def kind_group(kind: str) -> str:
    return KINDS.get(kind, kind)


def agrees(deep: list[str] | None, cand: list[str], parted: int) -> bool | None:
    """Whether the deep search's answer takes the candidate's tokens up to and through the one where the candidates part."""
    if deep is None:
        return None
    return deep[: parted + 1] == cand[: parted + 1]


def turn_band(t: int) -> str:
    return "0-1" if t <= 1 else "2-3" if t <= 3 else "4-6" if t <= 6 else "7-10" if t <= 10 else "11+"


def pct(k: int, n: int) -> str:
    return f"{100 * k / n:.1f}%" if n else "-"


def wilson(k: int, n: int) -> str:
    if n == 0:
        return "-"
    z, p = 1.96, k / n
    c = (p + z * z / (2 * n)) / (1 + z * z / n)
    h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / (1 + z * z / n)
    return f"[{100 * (c - h):.0f}, {100 * (c + h):.0f}]"


T975 = [12.706, 4.303, 3.182, 2.776, 2.571, 2.447, 2.365, 2.306, 2.262, 2.228, 2.201, 2.179, 2.160, 2.145, 2.131, 2.120,
        2.110, 2.101, 2.093, 2.086, 2.080, 2.074, 2.069, 2.064, 2.060, 2.056, 2.052, 2.048, 2.045, 2.042]


def mean_interval(xs: list[float]) -> tuple[float, float, float]:
    """The mean of `xs` and its 95% interval (Student's t; the normal's past 30 degrees of freedom), as
    `explore::mean_interval`."""
    n = len(xs)
    if n == 0:
        return math.nan, math.nan, math.nan
    m = sum(xs) / n
    if n < 2:
        return m, math.nan, math.nan
    sd = math.sqrt(sum((x - m) ** 2 for x in xs) / (n - 1))
    t = T975[n - 2] if n - 1 <= 30 else 1.96
    h = t * sd / math.sqrt(n)
    return m, m - h, m + h


def pooled(judgements: list[dict]) -> dict:
    """Several judgements of one fork (independent seeds: other worlds, other searches) pooled: every world's paired
    difference of the two candidates' values, one sample, with its 95% interval and verdict ("first", "second",
    "undecided"; "unjudged" where none was judged)."""
    diffs = [w["qa"] - w["qb"] for j in judgements for w in j.get("worlds", []) if w.get("qa") is not None and w.get("qb") is not None]
    if not any(j.get("verdict") not in (None, "unjudged") for j in judgements):
        return {"verdict": "unjudged", "error": next((j.get("error") for j in judgements if j.get("error")), None), "paired": 0}
    m, lo, hi = mean_interval(diffs)
    verdict = "undecided" if len(diffs) < 2 else "first" if lo > 0 else "second" if hi < 0 else "undecided"
    return {"verdict": verdict, "paired": len(diffs), "difference": {"mean": m, "low": lo, "high": hi},
            "runs": [j.get("verdict") for j in judgements]}


def merge(sel: dict, forks: list[dict], runs: list[dict[str, dict]], cross: list[dict[str, dict]]) -> list[dict]:
    """Each fork with its pair, who answered what (A or B), its judgements pooled over `runs` (the deep search's network
    over independent seeds; the first run also searched the position without a floor: the target) and over `cross`
    (the other network's), and whether each player's answer is the unfloored deep search's at the sub-step where the
    two part."""
    by_seed = {p["shuffle"]: (pn, p) for pn, p in sel["pairs"].items()}
    rows = []
    for f in forks:
        if f.get("status") != "fork":
            continue
        pair, p = by_seed[f["seed"]]
        seat = f["seat"]
        # The first directory holds each pair's even game, where A sat in seat 1.
        who_first = models_of(min(p["games"]))[seat]
        who = (who_first, "B" if who_first == "A" else "A")
        key = f"p{pair:05d}-first-{f['decision']:04d}"
        js = [r.get(key, {}).get("judgement", {}) for r in runs]
        jd = pooled(js)
        cj = pooled([r.get(key, {}).get("judgement", {}) for r in cross]) if cross else None
        first = js[0] if js else {}
        target = runs[0].get(key, {}).get("target") if runs else None
        tokens = first.get("tokens") or [None, None]
        parted = first.get("parted_at")
        deep = (target or {}).get("chosen_tokens")
        labels = first.get("labels") or f.get("labels")
        rows.append({
            "id": key, "pair": pair, "seed": f["seed"], "decision": f["decision"], "seat": seat, "turn": f["turn"],
            "kind": f["kind"], "decks": tuple(p["decks"]), "points": p["points"], "who": who, "labels": labels,
            "equivalent": bool(labels) and labels[0] == labels[1],
            "verdict": jd["verdict"], "better": {"first": who[0], "second": who[1]}.get(jd["verdict"]), "jd": jd,
            "first": first, "cross": cj, "target": target, "split": None,
            "agree": (None if parted is None or tokens[0] is None or deep is None else
                      {who[0]: agrees(deep, tokens[0], parted), who[1]: agrees(deep, tokens[1], parted)}),
        })
    return rows


def keep(rows: list[dict], losers: list[dict], plan_positions: list[dict], kl_min: float) -> dict[str, list[dict]]:
    """The positions written as targets, by split: each fork the pooled judgement decided (not two copies of one card),
    and each loser decision whose deep target is further from the network's prior than `kl_min` (nats)."""
    where = {p["id"]: p for p in plan_positions}
    out: dict[str, list[dict]] = {"train": [], "heldout": []}
    for r in rows:
        if r["verdict"] in ("first", "second") and not r["equivalent"]:
            p = where[r["id"]]
            out[p["split"]].append({"dir": p["dir"], "seed": p["seed"], "decision": p["decision"], "id": p["id"], "why": "fork"})
    forks = {r["id"] for r in rows}
    for j in losers:
        if j["id"] in forks or "target" not in j or j["target"].get("kl") is None or j["target"]["kl"] < kl_min:
            continue
        p = where[j["id"]]
        out[p["split"]].append({"dir": p["dir"], "seed": p["seed"], "decision": p["decision"], "id": p["id"], "why": "loser"})
    return out


def report(sel: dict, rows: list[dict], losers: list[dict], names: dict[str, str], replay: dict | None, settings: dict,
           kept: dict | None, forks: list[dict]) -> str:
    A, B = names["A"], names["B"]
    out: list[str] = []
    w = out.append
    nm = lambda who: names[who]  # noqa: E731
    w("# Pair miner: generation 5's gate (gen5 against gen4)\n")
    w("Local planning data, not for GitHub. Made by `ai/cantrip_ai/mine.py` and `cantrip-explore mine-*` (branch "
      "`ai-mine`). A pair is one deal played twice with the players exchanged (decks stay with seats); a pair one side "
      "won both is decisive. Outcomes only choose the pairs; every verdict here is the deep search's.\n")
    c = sel["counts"]
    w("## Pairs and replays\n")
    w(f"- {c['pairs']} pairs in the gate, {c['decisive_all']} decisive. A gate game's search seed is made from its "
      f"lane's game count, which the range rule recovers for {c['known']} pairs (games that finished before the first "
      f"of BotLab's {sel['ranges']} ranges was played out, at place {sel['first_range_done']} of 6,000).")
    w(f"- Mined: {len(sel['decisive'])} decisive pairs ({c['a_lost_both']} where {A} lost both, {c['a_won_both']} where "
      f"{A} won both) and {len(sel['control'])} split pairs as a control; {len(sel['decisive_unknown_ids'])} decisive "
      "pairs left out for an unknown game id.")
    if replay:
        w(f"- Replays: {replay['same']} of {replay['games']} games the same as the gate (winner, points, starter, turns, "
          f"decks, seats, shuffle); {len(replay['differ'])} differ.")
    st = collections.Counter(f.get("status") for f in forks)
    w(f"- Pairs walked in step: {dict(st)}.\n")

    judged = [r for r in rows if r["verdict"] != "unjudged"]
    w("## Forks and how many the deep search decides\n")
    w(f"Judgement: {settings.get('fork', '?')}. Decided: the pooled interval of the paired difference excludes 0. "
      "Unjudged: the deep search cannot stand at the position (a mulligan after a redraw: the engine cannot fork a run "
      "that drew before the request, so the network answers alone; search.rs `random_before`). Equivalent: both "
      "players played the same card (two copies), shown apart only for completeness.\n")
    w(f"| pairs | forks | unjudged | equivalent | judged | decided [95%] | {A} better | {B} better |")
    w("|---|---|---|---|---|---|---|---|")

    def line(rs, label):
        n = len(rs)
        unj = sum(1 for r in rs if r["verdict"] == "unjudged")
        eq = sum(1 for r in rs if r["equivalent"] and r["verdict"] != "unjudged")
        jj = [r for r in rs if r["verdict"] != "unjudged" and not r["equivalent"]]
        dec = [r for r in jj if r["verdict"] in ("first", "second")]
        a = sum(1 for r in dec if r["better"] == "A")
        w(f"| {label} | {n} | {unj} | {eq} | {len(jj)} | {len(dec)} ({pct(len(dec), len(jj))}) {wilson(len(dec), len(jj))} "
          f"| {a} | {len(dec) - a} |")

    line([r for r in rows if r["points"] == 0.0], f"{A} lost both")
    line([r for r in rows if r["points"] == 2.0], f"{A} won both")
    line([r for r in rows if r["points"] in DECISIVE], "decisive, all")
    line([r for r in rows if r["points"] == 1.0], "split (control)")
    line(rows, "all")
    w("")
    for pts, loser in ((0.0, "A"), (2.0, "B")):
        rs = [r for r in rows if r["points"] == pts and r["verdict"] in ("first", "second") and not r["equivalent"]]
        lw = sum(1 for r in rs if r["better"] == loser)
        winner = "B" if loser == "A" else "A"
        w(f"- {nm(loser)}'s double losses: of {len(rs)} decided forks, {nm(winner)}'s answer was the better in "
          f"{len(rs) - lw} ({pct(len(rs) - lw, len(rs))}) {wilson(len(rs) - lw, len(rs))}, {nm(loser)}'s in {lw}.")
    rs = [r for r in rows if r["points"] == 1.0 and r["verdict"] in ("first", "second") and not r["equivalent"]]
    a = sum(1 for r in rs if r["better"] == "A")
    w(f"- Split pairs: of {len(rs)} decided forks, {A}'s answer the better in {a} ({pct(a, len(rs))}).")
    # Whose answer, the winner's or the loser's, the deep search prefers in decisive pairs, both ways pooled.
    rs = [r for r in rows if r["points"] in DECISIVE and r["verdict"] in ("first", "second") and not r["equivalent"]]
    win = sum(1 for r in rs if r["better"] == ("B" if r["points"] == 0.0 else "A"))
    w(f"- Decisive pairs, both ways: the pair's winner's answer the better in {win} of {len(rs)} "
      f"({pct(win, len(rs))}) {wilson(win, len(rs))}.\n")
    runs_agree = [r for r in judged if len(r["jd"].get("runs", [])) >= 2 and not r["equivalent"]]
    if runs_agree:
        flips = sum(1 for r in runs_agree if {"first", "second"} <= set(r["jd"]["runs"]))
        both = sum(1 for r in runs_agree if r["jd"]["runs"][0] == r["jd"]["runs"][1])
        w(f"Seed to seed: of {len(runs_agree)} judged forks, the two runs' own verdicts are the same in {both} "
          f"({pct(both, len(runs_agree))}); opposite verdicts (one run first, the other second) in {flips}.\n")
    cr = [r for r in judged if r["cross"] and r["cross"]["verdict"] != "unjudged" and not r["equivalent"]]
    if cr:
        same = sum(1 for r in cr if r["cross"]["verdict"] == r["verdict"])
        opp = sum(1 for r in cr if {r["cross"]["verdict"], r["verdict"]} == {"first", "second"})
        dec_c = [r for r in cr if r["cross"]["verdict"] in ("first", "second")]
        a_c = sum(1 for r in dec_c if {"first": r["who"][0], "second": r["who"][1]}[r["cross"]["verdict"]] == "A")
        w(f"Cross-check with {names.get('cross', '?')}'s network as the deep search's ({settings.get('cross', '?')}): "
          f"{same} of {len(cr)} verdicts the same, {opp} opposite; it decides {len(dec_c)}, {A}'s answer the better in "
          f"{a_c} ({pct(a_c, len(dec_c))}).\n")

    w("## Each network against the deep search\n")
    w(f"At each fork the deep search ({settings.get('target', '?')}, no floor) answers the position itself; a network "
      "agrees when that answer takes its choice at the sub-step where the two answers part. Both cannot agree; both can "
      "disagree (the search plays a third move).\n")
    w(f"| forks | n | {A} agrees | {B} agrees | neither |")
    w("|---|---|---|---|---|")
    for label, sub in (("all", rows), (f"{A} lost both", [r for r in rows if r["points"] == 0.0]),
                       (f"{A} won both", [r for r in rows if r["points"] == 2.0]),
                       ("split", [r for r in rows if r["points"] == 1.0])):
        ok = [r for r in sub if r["agree"] and None not in r["agree"].values() and not r["equivalent"]]
        a = sum(1 for r in ok if r["agree"]["A"])
        b = sum(1 for r in ok if r["agree"]["B"])
        w(f"| {label} | {len(ok)} | {a} ({pct(a, len(ok))}) {wilson(a, len(ok))} | {b} ({pct(b, len(ok))}) "
          f"{wilson(b, len(ok))} | {len(ok) - a - b} |")
    w("")
    w("## Where the forks are\n")
    w(f"| request | forks | judged | decided | {A} better | {B} better |")
    w("|---|---|---|---|---|---|")
    for k, n in collections.Counter(kind_group(r["kind"]) for r in rows).most_common():
        rs = [r for r in rows if kind_group(r["kind"]) == k]
        jj = [r for r in rs if r["verdict"] != "unjudged" and not r["equivalent"]]
        d = [r for r in jj if r["verdict"] in ("first", "second")]
        a = sum(1 for r in d if r["better"] == "A")
        w(f"| {k} | {n} | {len(jj)} | {len(d)} ({pct(len(d), len(jj))}) | {a} | {len(d) - a} |")
    w("")
    w(f"| turn | forks | judged | decided | {A} better | {B} better |")
    w("|---|---|---|---|---|---|")
    for band in ("0-1", "2-3", "4-6", "7-10", "11+"):
        rs = [r for r in rows if turn_band(r["turn"]) == band]
        jj = [r for r in rs if r["verdict"] != "unjudged" and not r["equivalent"]]
        d = [r for r in jj if r["verdict"] in ("first", "second")]
        a = sum(1 for r in d if r["better"] == "A")
        w(f"| {band} | {len(rs)} | {len(jj)} | {len(d)} ({pct(len(d), len(jj))}) | {a} | {len(d) - a} |")
    w("")
    w("## Matchups\n")
    w(f"| seat 1 vs seat 2 | pairs | {A} lost both | {A} won both | split | forks decided | {A} better | {B} better |")
    w("|---|---|---|---|---|---|---|---|")
    by_m = collections.defaultdict(list)
    for p in sel["pairs"].values():
        by_m[tuple(p["decks"])].append(p)
    for m, ps in sorted(by_m.items()):
        rs = [r for r in rows if r["decks"] == m and not r["equivalent"]]
        d = [r for r in rs if r["verdict"] in ("first", "second")]
        a = sum(1 for r in d if r["better"] == "A")
        w(f"| {m[0]} vs {m[1]} | {len(ps)} | {sum(1 for p in ps if p['points'] == 0.0)} | "
          f"{sum(1 for p in ps if p['points'] == 2.0)} | {sum(1 for p in ps if p['points'] == 1.0)} | {len(d)} | {a} | "
          f"{len(d) - a} |")
    w("")
    w("## The 10 most decisive forks\n")
    w("Decisive pairs, by the pooled interval's bound nearest zero (the most certain difference).\n")

    def strength(r):
        d = r["jd"].get("difference", {})
        return d.get("low", 0) if r["verdict"] == "first" else -d.get("high", 0)

    top = sorted([r for r in rows if r["verdict"] in ("first", "second") and r["points"] in DECISIVE and not r["equivalent"]],
                 key=lambda r: -strength(r))[:10]
    for i, r in enumerate(top, 1):
        d = r["jd"]["difference"]
        q = r["first"].get("q") or [None, None]
        who = r["who"]
        pts = {0.0: f"{A} lost both", 2.0: f"{A} won both"}[r["points"]]
        w(f"### {i}. Pair {r['pair']}: {r['decks'][0]} vs {r['decks'][1]}, {pts}; turn {r['turn']}, seat {r['seat'] + 1} "
          f"({nm(who[0])} in the first game, {nm(who[1])} in the second), {kind_group(r['kind'])}\n")
        for k in (0, 1):
            w(f"- {nm(who[k])} played: {r['labels'][k]}" + (f" (Q {q[k]:+.3f} in the first run)" if q[k] is not None else ""))
        w(f"- Q difference ({nm(who[0])}'s less {nm(who[1])}'s): {d['mean']:+.3f}, 95% [{d['low']:+.3f}, {d['high']:+.3f}] "
          f"over {r['jd']['paired']} worlds; the better: {nm(r['better'])}'s; runs {r['jd'].get('runs')}"
          + (f"; {names.get('cross', '?')}'s search: {r['cross']['verdict']}" if r["cross"] else ""))
        t = r["target"] or {}
        if t:
            w(f"- The deep search's own answer: {t.get('chosen')} (root value {t.get('root_value', math.nan):+.3f}, the "
              f"network's {t.get('net_value', math.nan):+.3f}; KL from the prior {t.get('kl', math.nan):.3f})")
        w(f"- Position: shuffle {r['seed']}, decision {r['decision']} (`{r['id']}`).\n")
    w("## The losers' decisions before their largest value drop\n")
    lj = [j for j in losers if isinstance(j.get("target"), dict) and j["target"].get("kl") is not None]
    if lj:
        kls = sorted(j["target"]["kl"] for j in lj)
        q = lambda p: kls[min(len(kls) - 1, int(p * len(kls)))]  # noqa: E731
        top_agree = sum(1 for j in lj if j["target"].get("top_agree"))
        played = [j["target"].get("played_agree_first_step") for j in lj]
        pa, pn = sum(1 for x in played if x), sum(1 for x in played if x is not None)
        kinds = collections.Counter(kind_group(j["kind"]) for j in lj)
        w(f"- {len(lj)} positions ({settings.get('loser', '?')}); by request: {dict(kinds.most_common())}.")
        w(f"- KL(deep target || network prior), nats: median {q(0.5):.3f}, p75 {q(0.75):.3f}, p90 {q(0.9):.3f}; above "
          f"{settings.get('kl_min', '?')}: {sum(1 for x in kls if x >= settings.get('kl_min', 0))}.")
        w(f"- The deep target's top move is the prior's in {top_agree} ({pct(top_agree, len(lj))}); the deep search's "
          f"first-step choice is the loser's own in {pa} of {pn} ({pct(pa, pn)}).")
    else:
        w("- None searched.")
    if kept:
        w("\n## Targets\n")
        for k, v in kept.items():
            w(f"- {k}: {v}")
    return "\n".join(out) + "\n"


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in Path(path).read_text(encoding="utf-8").splitlines() if line.strip()]


def by_id(path: Path) -> dict[str, dict]:
    return {j["id"]: j for j in read_jsonl(Path(path) / "judged.jsonl")}


def main(argv=None):
    a = argparse.ArgumentParser(prog="python -m cantrip_ai.mine", description=__doc__.split("\n\n")[0])
    sub = a.add_subparsers(dest="command", required=True)
    s = sub.add_parser("select")
    s.add_argument("--results", required=True, type=Path)
    s.add_argument("--out", required=True, type=Path)
    s.add_argument("--ranges", type=int, default=0,
                   help="BotLab's ranges, for a duel played before searches were seeded by the game's number "
                        "(2026-11-16): only the games whose lane ids the range rule recovers; 0 (a duel since), every game")
    s.add_argument("--control", type=int, default=200)
    s.add_argument("--seed", type=int, default=5)
    v = sub.add_parser("verify")
    v.add_argument("--gate", required=True, type=Path)
    v.add_argument("--replayed", required=True, type=Path, nargs="+")
    v.add_argument("--out", type=Path)
    lo = sub.add_parser("losers")
    lo.add_argument("--selected", required=True, type=Path)
    lo.add_argument("--a-model", required=True)
    lo.add_argument("--b-model", required=True)
    p = sub.add_parser("plan")
    p.add_argument("--selected", required=True, type=Path)
    p.add_argument("--forks", required=True, type=Path)
    p.add_argument("--values", type=Path, nargs="*", default=[])
    p.add_argument("--out", required=True, type=Path)
    p.add_argument("--heldout", type=float, default=0.15)
    p.add_argument("--before", type=int, default=3)
    p.add_argument("--seed", type=int, default=5)
    k = sub.add_parser("keep")
    k.add_argument("--selected", required=True, type=Path)
    k.add_argument("--forks", required=True, type=Path)
    k.add_argument("--plan", required=True, type=Path)
    k.add_argument("--runs", required=True, type=Path, nargs="+", help="the fork judgements (the first also the targets)")
    k.add_argument("--losers", required=True, type=Path, nargs="+")
    k.add_argument("--kl-min", type=float, default=0.1)
    k.add_argument("--out", required=True, type=Path)
    r = sub.add_parser("report")
    r.add_argument("--selected", required=True, type=Path)
    r.add_argument("--forks", required=True, type=Path)
    r.add_argument("--runs", required=True, type=Path, nargs="+")
    r.add_argument("--cross", type=Path, nargs="*", default=[])
    r.add_argument("--losers", type=Path, nargs="*", default=[])
    r.add_argument("--replay", type=Path)
    r.add_argument("--settings", type=Path, help="a JSON object: the searches' settings in words, and kl_min")
    r.add_argument("--kept", type=Path)
    r.add_argument("--a-name", default="gen5")
    r.add_argument("--b-name", default="gen4")
    r.add_argument("--cross-name", default="gen5")
    r.add_argument("--out", required=True, type=Path)
    args = a.parse_args(argv)
    if args.command == "select":
        sel = select(read_results(args.results), args.ranges, args.control, args.seed)
        write_selection(sel, args.out)
        print(json.dumps({k: sel[k] for k in ("counts", "first_range_done")} | {
            "decisive": len(sel["decisive"]), "control": len(sel["control"]),
            "decisive_unknown_ids": len(sel["decisive_unknown_ids"])}))
    elif args.command == "verify":
        res = verify(read_results(args.gate), [x for f in args.replayed for x in read_results(f)])
        if args.out:
            args.out.write_text(json.dumps(res, indent=1), encoding="utf-8")
        print(json.dumps({"games": res["games"], "same": res["same"], "differ": len(res["differ"])}))
        for d in res["differ"][:20]:
            print("  differs:", json.dumps(d))
    elif args.command == "losers":
        sel = load_selection(args.selected)
        for name, games in loser_games(sel, args.a_model, args.b_model).items():
            (args.selected / f"losers-{name}.jsonl").write_text("".join(json.dumps(g) + "\n" for g in games), encoding="utf-8")
            print(name, len(games))
    elif args.command == "plan":
        sel = load_selection(args.selected)
        values = []
        for f in args.values:
            name = "first" if "first" in Path(f).name else "second"
            values += [dict(x, dir=name) for x in read_jsonl(f)]
        positions = plan(sel, read_jsonl(args.forks), values, args.heldout, args.before, args.seed)
        args.out.write_text("".join(json.dumps(x) + "\n" for x in positions), encoding="utf-8")
        c = collections.Counter((x["split"], "fork" if "candidates" in x else "loser") for x in positions)
        print(json.dumps({f"{k[0]}/{k[1]}": n for k, n in sorted(c.items())}))
    elif args.command == "keep":
        sel = load_selection(args.selected)
        rows = merge(sel, read_jsonl(args.forks), [by_id(p) for p in args.runs], [])
        losers = [j for d in args.losers for j in read_jsonl(d / "judged.jsonl")]
        kept = keep(rows, losers, read_jsonl(args.plan), args.kl_min)
        args.out.mkdir(parents=True, exist_ok=True)
        for split, lines in kept.items():
            (args.out / f"keep-{split}.jsonl").write_text("".join(json.dumps(x) + "\n" for x in lines), encoding="utf-8")
        print(json.dumps({s: dict(collections.Counter(x["why"] for x in v)) for s, v in kept.items()}))
    elif args.command == "report":
        sel = load_selection(args.selected)
        forks = read_jsonl(args.forks)
        rows = merge(sel, forks, [by_id(p) for p in args.runs], [by_id(p) for p in args.cross])
        losers = [j for d in args.losers for j in read_jsonl(d / "judged.jsonl")]
        replay = json.loads(args.replay.read_text(encoding="utf-8")) if args.replay else None
        settings = json.loads(args.settings.read_text(encoding="utf-8")) if args.settings else {}
        kept = json.loads(args.kept.read_text(encoding="utf-8")) if args.kept else None
        names = {"A": args.a_name, "B": args.b_name, "cross": args.cross_name}
        args.out.write_text(report(sel, rows, losers, names, replay, settings, kept, forks), encoding="utf-8")


if __name__ == "__main__":
    main()
