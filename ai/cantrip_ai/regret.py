"""The regret measure, read: positions sampled from traced games, valued by `cantrip-regret value`
(crates/cantrip-ai-host/src/regret.rs), tagged with the situation categories, and the
regret of each scored answer reported overall, per category, per role, per matchup and per turn band, with the
value's calibration beside it.

    python -m cantrip_ai.regret sample --traces DIR --out FILE [--count N] [--keep N] [--seed N]
    python -m cantrip_ai.regret report --values FILE [FILE ...] --threats FILE --out DIR [--decks FILE]

`sample` draws positions from the searched decisions of the traces (`cantrip_ai::trace`): stratified by turn (each
turn 1 to 15 alike, the rest as one more) and by request kind (`GROUP`, `SHARE`: priority 40%, attackers, blockers,
targets, mulligans and the rest 15% each, a group short of its share giving it to the others), and marks the
first `--keep` of a seeded order as the permanent set every later champion re-scores (`keep` in each line). It also
writes, beside FILE, `decks.json`: each deck's mean turn of the games it won, the matchup prior of the role tag.

`report` reads the valued positions. A candidate's value V(a) is its mean over the worlds; the scored answers are:
`search` (the answer the traced game's player gave, the PIMC search's), `network` (the network's best answer),
`uniform` (a candidate drawn uniformly: the expected regret of a blind choice among them, a floor every player should
beat). Regret is max V less V(answer), on the +-1 scale, so 0.2 is ten points of win probability: a blunder. Because
the best is chosen on the same worlds it is scored on, the plug-in regret is biased upward by the noise; `cross` is
the cross-fitted estimate (the best chosen on half the worlds, both scored on the other half, both ways), which is
not. The noise itself is reported: the standard error of V(best) - V(answer) over the worlds.

The categories, each a predicate over the seat's observation, the reference values and the world
deals (never the hidden state), every one reported per role and per matchup:

- role: the clock comparison weighted by the matchup. A side's clock is the turns its creatures' power on the board
  takes to kill the other (opponent's life over the power, rounded up; none with no power, held at 20). The score is
  the opponent's clock less ours, plus the matchup prior (+0.5 for the deck whose wins come at least half a turn
  sooner in the sampled games' run, -0.5 for the other, 0 between them): beatdown above 0, control below, even at 0.
- mode switch: the seat's role differs from its role at its last decision of an earlier turn.
- ahead (best value's win probability above 0.7) and behind (below 0.3), with the outs: the share of worlds the
  answer loses (V < 0), and the spread of its values over the worlds.
- can't play around X: for a card X the worlds deal the opponent in 25-90% of them (at least 4 worlds with and
  without), the best answer over all worlds is not the best over the worlds with X, and X matters there (by 0.1 or
  more). Read: does the seat accept X (take the overall best) or guard (take X's best).
- counter open: a priority in our main phase with a spell to cast, and the opponent's untapped lands reach the
  cheapest counterspell left unseen in their list; bait: the same with two or more spells to cast.
- removal open: declaring attackers, and their untapped lands reach the cheapest instant-speed removal left unseen.
- sweeper likely: a sweeper left unseen in their list within their lands plus one, and three or more creatures of ours.
- lethal now: our turn before blockers, and our untapped, unsick creatures' power reaches their life (on the board,
  blockers aside); lethal threat: their creatures' power reaches our life.
- combat: declaring attackers or blockers with two or more creatures that may.
- stack: anything on the stack.
- end-of-turn instant: the opponent's end step, our priority, a spell to cast.
- mulligan.
- mana constrained: our main phase, two or more spells to cast whose mana values add up past our untapped lands.

The classes of cards (counterspell, removal, sweeper) are the threats file (`--threats`, as `belief.py` reads it);
mana values from content/mtg/cards.
"""

from __future__ import annotations

import argparse
import json
import math
import random
import sys
from collections import defaultdict
from pathlib import Path

import numpy as np

from .belief import card_facts_by_name

CREATURE, LAND = 1 << 1, 1 << 4
MAIN = (3, 9)
TURN_BANDS = ((0, 0), (1, 2), (3, 4), (5, 6), (7, 9), (10, 14), (15, 99))
BLUNDER = 0.2


# The request kinds positions are stratified by, and each group's share of a turn's positions.
GROUP = {"Priority": "priority", "DeclareAttackers": "attackers", "DeclareBlockers": "blockers", "Mulligan": "mulligan",
         "ChooseTarget": "targets", "ChooseCards": "targets", "ChooseDividedTargets": "targets", "ChooseModes": "targets",
         "SearchLibrary": "targets"}
SHARE = {"priority": 0.4, "attackers": 0.15, "blockers": 0.15, "targets": 0.15, "mulligan": 0.15, "other": 0.15}


# ---- sampling ---------------------------------------------------------------------------------------------------

def load_traces(directory: str) -> dict:
    out = {}
    for f in sorted(Path(directory).glob("*.jsonl")):
        for line in f.read_text(encoding="utf-8").splitlines():
            if line.strip():
                t = json.loads(line)
                out[t["seed"]] = t
    return out


def sample(args):
    traces = load_traces(args.traces)
    rng = random.Random(args.seed)
    strata = defaultdict(list)
    for seed, t in traces.items():
        for i, s in enumerate(t["steps"]):
            if s.get("searched") is None:
                continue
            turn = s["turn"] if s["turn"] <= 15 else 16
            strata[turn].append((seed, i, s.get("kind", "?")))
    turns = sorted(strata)
    per_turn = args.count // max(1, len(turns))
    chosen = []
    for turn in turns:
        by_group = defaultdict(list)
        for p in strata[turn]:
            by_group[GROUP.get(p[2], "other")].append(p)
        for v in by_group.values():
            rng.shuffle(v)
        # Each group its share of the turn, a group short of its share giving the rest to the others in proportion.
        want = {g: SHARE[g] for g in by_group}
        picked = []
        left = per_turn
        while left > 0 and any(by_group.values()):
            total = sum(w for g, w in want.items() if by_group[g])
            quota = {g: max(1, round(left * w / total)) for g, w in want.items() if by_group[g]}
            for g, q in quota.items():
                take = by_group[g][: min(q, left)]
                del by_group[g][: len(take)]
                picked.extend(take)
                left -= len(take)
        chosen.extend(picked)
    rng.shuffle(chosen)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w") as f:
        for i, (seed, at, kind) in enumerate(chosen):
            f.write(json.dumps(dict(id=i, seed=seed, at=at, kind=kind, keep=i < args.keep)) + "\n")
    # The matchup prior of the role tag: each deck's mean turn of the games it won.
    won = defaultdict(list)
    for t in traces.values():
        for seat, r in enumerate(t.get("outcome") or []):
            if r == 1 and t["steps"]:
                won[t["decks"][seat]].append(t["steps"][-1]["turn"])
    decks = {d: dict(wins=len(v), mean_win_turn=float(np.mean(v))) for d, v in won.items()}
    (out.parent / "decks.json").write_text(json.dumps(decks, indent=1))
    print(f"{len(chosen)} positions from {len(traces)} traces ({sum(len(v) for v in strata.values())} searched decisions), "
          f"{min(args.keep, len(chosen))} kept as the permanent set; per turn: " +
          ", ".join(f"{t}:{min(per_turn, len(strata[t]))}" for t in turns))


# ---- features of an observation ---------------------------------------------------------------------------------

def visible(obs):
    for i, o in enumerate(obs["objects"]):
        if "Visible" in o:
            yield i, o["Visible"]


def board(obs, p):
    out = [v for _, v in visible(obs) if v["place"]["zone"] == "Battlefield" and v["controller"] == p]
    creatures = [v for v in out if v["chars"]["types"] & CREATURE]
    lands = [v for v in out if v["chars"]["types"] & LAND]
    return creatures, lands


def power(creatures, ready=False):
    return sum(max(v["chars"].get("power") or 0, 0) for v in creatures if not ready or (not v["tapped"] and not v["sick"]))


def clock(life, pw):
    return 20 if pw <= 0 else min(20, math.ceil(max(life, 1) / pw))


def role_of(obs, decks, matchup_prior):
    me = obs["seat"]
    opp = 1 - me
    mine, _ = board(obs, me)
    theirs, _ = board(obs, opp)
    life = [p["life"] for p in obs["players"]]
    score = clock(life[me], power(theirs)) - clock(life[opp], power(mine)) + matchup_prior
    return "beatdown" if score > 0 else "control" if score < 0 else "even"


def options(obs):
    ask = (obs.get("request") or {}).get("ask") or {}
    kind = ask.get("kind") or {}
    name = next(iter(kind), "") if isinstance(kind, dict) else str(kind)
    body = kind.get(name, {}) if isinstance(kind, dict) else {}
    return name, body


def spells_to_cast(obs, body):
    out = []
    for o in body.get("options", []):
        if o["kind"] == 1 and isinstance(o["source"], dict) and "Object" in o["source"]:
            src = obs["objects"][o["source"]["Object"]]
            mv = src["Visible"]["chars"]["mana_value"] if "Visible" in src else 0
            out.append((o["source"]["Object"], mv))
    return out


def tags(pos, facts, deck_turns):
    """The categories a valued position falls in, and the measures that go with them."""
    obs = pos["observation"]
    me, opp = obs["seat"], 1 - obs["seat"]
    decks = pos["decks"]
    my_deck, their_deck = decks[me], decks[opp]
    a, b = deck_turns.get(my_deck), deck_turns.get(their_deck)
    prior = 0.0
    if a is not None and b is not None and my_deck != their_deck:
        prior = 0.5 if a <= b - 0.5 else -0.5 if a >= b + 0.5 else 0.0
    out = {"role": role_of(obs, decks, prior), "matchup": f"{my_deck}/{their_deck}"}
    prev = pos.get("previous_observation")
    out["mode_switch"] = prev is not None and role_of(prev, decks, prior) != out["role"]
    ask_name, body = options(obs)
    turn = obs["turn"]
    active, step = turn["active"] == me, turn["step"]
    mine, my_lands = board(obs, me)
    theirs, their_lands = board(obs, opp)
    their_untapped = sum(not v["tapped"] for v in their_lands)
    my_untapped = sum(not v["tapped"] for v in my_lands)
    life = [p["life"] for p in obs["players"]]
    pool = {c["name"]: c["count"] for c in (obs["pools"][opp] if len(obs["pools"]) > opp else [])}

    def cheapest(cls):
        mvs = [facts["mv"].get(n, 99) for n, c in pool.items() if c > 0 and n in facts[cls]]
        return min(mvs) if mvs else None

    spells = spells_to_cast(obs, body)
    counter = cheapest("counter")
    out["counter_open"] = ask_name == "Priority" and active and step in MAIN and bool(spells) and counter is not None and their_untapped >= counter
    out["bait"] = out["counter_open"] and len({s for s, _ in spells}) >= 2
    removal = cheapest("removal")
    out["removal_open"] = ask_name == "DeclareAttackers" and removal is not None and their_untapped >= removal
    sweeper = cheapest("sweeper")
    out["sweeper_likely"] = sweeper is not None and len(their_lands) + 1 >= sweeper and len(mine) >= 3
    out["lethal_now"] = active and step <= 5 and power(mine, ready=True) >= life[opp] > 0
    out["lethal_threat"] = power(theirs) >= life[me] > 0
    out["combat"] = (ask_name == "DeclareAttackers" and len(body.get("eligible", [])) >= 2) or \
        (ask_name == "DeclareBlockers" and (len(body.get("eligible", [])) >= 2 or len(body.get("attackers", [])) >= 2))
    out["stack"] = any(("Visible" in o and o["Visible"]["place"]["zone"] == "Stack") or "Ability" in o for o in obs["objects"])
    out["eot_instant"] = (not active) and step == 10 and ask_name == "Priority" and bool(spells) and my_untapped > 0
    out["mulligan"] = ask_name == "Mulligan"
    out["mana_constrained"] = ask_name == "Priority" and active and step in MAIN and len(spells) >= 2 and \
        sum(mv for _, mv in {s: mv for s, mv in spells}.items()) > my_untapped
    return out


# ---- the reference --------------------------------------------------------------------------------------------

def matrix(pos):
    """[candidates, worlds] of values (NaN where a line failed)."""
    return np.array([[np.nan if v is None else v for v in c["values"]] for c in pos["candidates"]], np.float64)


def scored(pos, rng, weights=None):
    """Per scored answer: plug-in regret, cross-fitted regret, the noise of best less answer. `weights`: the worlds'
    belief weights (uniform without), each candidate's value their weighted mean."""
    m = matrix(pos)
    ok = ~np.isnan(m).any(0)
    m = m[:, ok]
    w = m.shape[1]
    if w < 4 or m.shape[0] < 1:
        return None
    b = np.ones(w) if weights is None else np.asarray(weights, np.float64)[ok]
    b = b / b.sum()

    def mean(rows, cols):
        bb = b[cols] / b[cols].sum()
        return (m[rows][:, cols] * bb).sum(-1)

    allw = np.arange(w)
    v = mean(slice(None), allw)
    best = int(np.argmax(v))
    cands = pos["candidates"]
    who = {"search": next((i for i, c in enumerate(cands) if c["given"]), None),
           "network": next((i for i, c in enumerate(cands) if c["best"]), None)}
    for name, p in (pos.get("players") or {}).items():
        if "candidate" in p:
            who[name] = p["candidate"]
    perm = rng.permutation(w)
    h1, h2 = perm[: w // 2], perm[w // 2 :]
    v1, v2 = mean(slice(None), h1), mean(slice(None), h2)
    b1, b2 = int(np.argmax(v1)), int(np.argmax(v2))

    def cross(i):
        return 0.5 * ((v2[b1] - v2[i]) + (v1[b2] - v1[i]))

    out = {}
    for name, i in who.items():
        if i is None:
            continue
        diff = m[best] - m[i]
        se = math.sqrt(float((b * (diff - (b * diff).sum()) ** 2).sum()) * float((b ** 2).sum())) if w > 1 else 0.0
        out[name] = dict(regret=float(v[best] - v[i]), cross=float(cross(i)), se=se)
    out["uniform"] = dict(regret=float(v[best] - v.mean()), cross=float(np.mean([cross(i) for i in range(m.shape[0])])), se=0.0)
    return dict(values=v, best=best, scored=out, worlds=w, m=m, ess=float(1.0 / (b ** 2).sum()))


def around(pos, ref):
    """Can't play around X (see the module): the cards X it fires on, and whether the answer given accepts or guards."""
    m = ref["m"]
    hands = [w["opponent_hand"] for w in pos["worlds"]]
    ok = ~np.isnan(matrix(pos)).any(0)
    hands = [h for h, k in zip(hands, ok) if k]
    given = next((i for i, c in enumerate(pos["candidates"]) if c["given"]), None)
    found = []
    for x in sorted({n for h in hands for n in h}):
        has = np.array([x in h for h in hands])
        k = has.sum()
        if not (0.25 <= k / len(hands) <= 0.9 and k >= 4 and len(hands) - k >= 4):
            continue
        a_all = ref["best"]
        a_x = int(np.argmax(m[:, has].mean(1)))
        if a_x == a_all or m[a_x, has].mean() - m[a_all, has].mean() < 0.1:
            continue
        found.append(dict(card=x, p=float(k / len(hands)), accept=given == a_all, guard=given == a_x))
    return found


def calibration(pairs, bins=10):
    """Reliability of a value in [-1, 1] as the probability (v + 1) / 2 against the outcome (+1 win): bins and ECE."""
    if not pairs:
        return None
    p = np.array([(v + 1) / 2 for v, _ in pairs])
    y = np.array([o == 1 for _, o in pairs], np.float64)
    keep = np.array([o != 0 for _, o in pairs])
    p, y = p[keep], y[keep]
    if not len(p):
        return None
    idx = np.minimum((p * bins).astype(int), bins - 1)
    rows = []
    ece = 0.0
    for b in range(bins):
        at = idx == b
        if at.any():
            rows.append(dict(bin=b, n=int(at.sum()), mean_p=float(p[at].mean()), rate=float(y[at].mean())))
            ece += at.sum() * abs(p[at].mean() - y[at].mean())
    return dict(n=int(len(p)), ece=float(ece / len(p)), brier=float(((p - y) ** 2).mean()), bins=rows)


def summarize(rows):
    out = {}
    names = sorted({n for x in rows for n in x["scored"]}, key=lambda n: (n not in ("search", "network", "uniform"), n))
    for name in names:
        r = [x["scored"][name] for x in rows if name in x["scored"]]
        if not r:
            continue
        reg = np.array([x["regret"] for x in r])
        cross = np.array([x["cross"] for x in r])
        se = np.array([x["se"] for x in r])
        out[name] = dict(n=len(r), mean=float(reg.mean()), cross=float(cross.mean()),
                         ci95=float(1.96 * cross.std(ddof=1) / math.sqrt(len(r))) if len(r) > 1 else None,
                         blunder=float((reg > BLUNDER).mean()), cross_blunder=float((cross > BLUNDER).mean()),
                         p50=float(np.median(reg)), p90=float(np.quantile(reg, 0.9)), noise=float(np.median(se)) if len(se) else None)
    return out


def band(turn):
    return next(f"{lo}-{hi}" for lo, hi in TURN_BANDS if lo <= turn <= hi)


CATEGORIES = ("mode_switch", "ahead", "behind", "around", "counter_open", "bait", "removal_open", "sweeper_likely",
              "lethal_now", "lethal_threat", "combat", "stack", "eot_instant", "mulligan", "mana_constrained")
RANGE = ("counter_open", "bait", "sweeper_likely", "behind", "around")
TACTICAL = ("lethal_now", "lethal_threat", "combat", "stack")


def report(args):
    rng = np.random.default_rng(args.seed)
    threats = json.loads(Path(args.threats).read_text(encoding="utf-8"))
    facts = card_facts_by_name(threats)
    deck_turns = {}
    if args.decks and Path(args.decks).exists():
        deck_turns = {d: v["mean_win_turn"] for d, v in json.loads(Path(args.decks).read_text()).items()}
    rows, failed = [], 0
    belief_of = {}
    if args.belief:
        from .belief import Belief
        reader = Belief(args.belief, args.belief_checkpoint, args.threats)
        names = json.loads(Path(args.names).read_text(encoding="utf-8"))
        names = names["names"] if isinstance(names, dict) else names
        positions = []
        for path in args.values:
            for line in Path(path).read_text(encoding="utf-8").splitlines():
                if line.strip():
                    pos = json.loads(line)
                    if "position" in pos:
                        positions.append((pos["id"], bytes.fromhex(pos["position"])))
        marg = reader.marginals([p for _, p in positions], names)
        belief_of = {i: m for (i, _), m in zip(positions, marg)}
    for path in args.values:
        for line in Path(path).read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            pos = json.loads(line)
            if "error" in pos:
                failed += 1
                continue
            weights = None
            if pos["id"] in belief_of:
                weights = world_weights(belief_of[pos["id"]], pos["worlds"])
            ref = scored(pos, rng, weights)
            if ref is None:
                failed += 1
                continue
            t = tags(pos, facts, deck_turns)
            p_best = (ref["values"][ref["best"]] + 1) / 2
            t["ahead"], t["behind"] = p_best > 0.7, p_best < 0.3
            found = around(pos, ref)
            t["around"] = bool(found)
            given = next((i for i, c in enumerate(pos["candidates"]) if c["given"]), None)
            m = ref["m"]
            outs = None
            if given is not None:
                outs = dict(lose_given=float((m[given] < 0).mean()), lose_best=float((m[ref["best"]] < 0).mean()),
                            spread_given=float(m[given].std()), spread_best=float(m[ref["best"]].std()))
            rows.append(dict(id=pos["id"], keep=pos.get("keep"), turn=pos["turn"], kind=pos["kind"], tags=t, scored=ref["scored"],
                             around=found, outs=outs, net_value=pos["net_value"], outcome=pos.get("outcome"),
                             root=(pos.get("searched") or [None])[0], v_given=float(ref["values"][given]) if given is not None else None,
                             worlds=ref["worlds"], ess=ref["ess"], candidates=len(pos["candidates"]),
                             players={n: p.get("report") for n, p in (pos.get("players") or {}).items()}))
    groups = defaultdict(list)
    for r in rows:
        t = r["tags"]
        keys = ["all", f"role {t['role']}", f"matchup {t['matchup']}", f"turn {band(r['turn'])}", f"kind {r['kind']}"]
        for c in CATEGORIES:
            if t.get(c):
                keys += [f"cat {c}", f"cat {c} / role {t['role']}"]
        if any(t.get(c) for c in RANGE):
            keys.append("range-reasoning")
        if any(t.get(c) for c in TACTICAL):
            keys.append("tactical")
        for k in keys:
            groups[k].append(r)
    table = {k: summarize(v) for k, v in sorted(groups.items())}
    calib = {}
    for k, v in sorted(groups.items()):
        if k.startswith("kind") or "/ role" in k:
            continue
        calib[k] = dict(network=calibration([(r["net_value"], r["outcome"]) for r in v if r["outcome"] is not None]),
                        reference=calibration([(r["v_given"], r["outcome"]) for r in v if r["outcome"] is not None and r["v_given"] is not None]),
                        search=calibration([(r["root"], r["outcome"]) for r in v if r["outcome"] is not None and r["root"] is not None]))
    around_rows = [x for r in rows for x in r["around"]]
    labels = None
    if args.luck:
        luck = {}
        for line in Path(args.luck).read_text(encoding="utf-8").splitlines():
            if line.strip():
                x = json.loads(line)
                if "error" not in x:
                    luck[x["id"]] = x
        labels = label_comparison([r for r in rows if r["id"] in luck], luck)
    counts = {k: len(v) for k, v in groups.items()}
    roles = defaultdict(lambda: defaultdict(int))
    for r in rows:
        for c in CATEGORIES:
            if r["tags"].get(c):
                roles[c][r["tags"]["role"]] += 1
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    result = dict(positions=len(rows), failed=failed, regret=table, calibration=calib, counts=counts,
                  category_roles={k: dict(v) for k, v in roles.items()},
                  around=dict(n=len(around_rows), accept=sum(x["accept"] for x in around_rows), guard=sum(x["guard"] for x in around_rows)),
                  outs={k: _outs(groups.get(f"cat {k}", [])) for k in ("ahead", "behind")}, labels=labels)
    (out / "report.json").write_text(json.dumps(result, indent=1, default=_plain))
    (out / "rows.jsonl").write_text("\n".join(json.dumps(r, default=_plain) for r in rows) + "\n")
    print_summary(result)


def label_comparison(rows, luck):
    """The value labels against the reference's V(answer given): the outcome, the outcome less the
    draws' luck (`cantrip-regret luck`), the search's root value, the network's value: mean squared error and its
    share of the outcome's, and each label's correlation with the reference."""
    ref = np.array([r["v_given"] for r in rows], np.float64)
    out = np.array([r["outcome"] for r in rows], np.float64)
    adj = np.array([r["outcome"] - luck[r["id"]]["correction"] for r in rows], np.float64)
    root = np.array([np.nan if r["root"] is None else r["root"] for r in rows], np.float64)
    net = np.array([r["net_value"] for r in rows], np.float64)
    result = dict(n=len(rows), draws_corrected=float(np.mean([luck[r["id"]]["corrected"] for r in rows])))
    base = None
    for name, x in (("outcome", out), ("luck_adjusted", adj), ("root", root), ("network", net)):
        ok = ~np.isnan(x) & ~np.isnan(ref)
        mse = float(((x[ok] - ref[ok]) ** 2).mean())
        base = base or mse
        result[name] = dict(n=int(ok.sum()), mse=mse, of_outcome=mse / base, corr=float(np.corrcoef(x[ok], ref[ok])[0, 1]),
                             mean=float(x[ok].mean()))
    return result


def world_weights(marginals, worlds):
    """A world's weight under the belief over its prior: the head's probability of each pool card's copies in the
    world's opponent hand over the prior's, multiplied over the cards (the per-card marginals, their correlation
    dropped)."""
    out = []
    for w in worlds:
        hand = {}
        for n in w["opponent_hand"]:
            hand[n] = hand.get(n, 0) + 1
        lw = 0.0
        for name, (prior, head) in marginals.items():
            k = min(hand.get(name, 0), len(prior) - 1)
            if prior[k] > 0 and head[k] > 0:
                lw += math.log(head[k]) - math.log(prior[k])
        out.append(lw)
    out = np.array(out)
    return np.exp(out - out.max())


def _plain(o):
    """numpy's scalars as JSON's."""
    return o.item() if hasattr(o, "item") else str(o)


def _outs(rows):
    o = [r["outs"] for r in rows if r["outs"]]
    if not o:
        return None
    return {k: float(np.mean([x[k] for x in o])) for k in o[0]}


def print_summary(result):
    print(f"{result['positions']} positions ({result['failed']} failed)")
    for k, v in result["regret"].items():
        if k.startswith(("kind", "matchup")) or "/ role" in k:
            continue
        line = "  ".join(f"{n} {s['cross']:+.3f}+-{(s['ci95'] or 0):.3f} (plug {s['mean']:.3f}, blunder {s['cross_blunder']:.1%})" for n, s in v.items())
        print(f"{k:28s} n={v.get('search', {}).get('n', 0):5d}  {line}")
    print("around:", result["around"], "outs:", result["outs"])
    if result.get("labels"):
        print("labels against the reference:", json.dumps(result["labels"]))
    c = result["calibration"].get("all", {})
    for who in ("network", "reference", "search"):
        if c.get(who):
            print(f"calibration {who}: ECE {c[who]['ece']:.3f}, Brier {c[who]['brier']:.3f}, n {c[who]['n']}")


def main(argv=None):
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = a.add_subparsers(dest="command", required=True)
    s = sub.add_parser("sample")
    s.add_argument("--traces", required=True)
    s.add_argument("--out", required=True)
    s.add_argument("--count", type=int, default=5000)
    s.add_argument("--keep", type=int, default=1000)
    s.add_argument("--seed", type=int, default=0)
    r = sub.add_parser("report")
    r.add_argument("--values", nargs="+", required=True)
    r.add_argument("--threats", required=True)
    r.add_argument("--decks")
    r.add_argument("--out", required=True)
    r.add_argument("--seed", type=int, default=0)
    r.add_argument("--belief", help="a belief head's directory (belief.py's --out): worlds weighted by it")
    r.add_argument("--belief-checkpoint", help="the network checkpoint the head reads")
    r.add_argument("--names", help="the card index the positions' rows are of: a cards.json or a JSON list")
    r.add_argument("--luck", help="cantrip-regret luck's output for these positions: the label comparison")
    args = a.parse_args(argv)
    {"sample": sample, "report": report}[args.command](args)


if __name__ == "__main__":
    sys.exit(main())
