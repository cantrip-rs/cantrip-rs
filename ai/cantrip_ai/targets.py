"""Alternative value-training targets: computed once from a loaded
training set, per sample, for `train.py`.

- `td(data, lam)`: the seat's lambda-return over its later searched root values: the target of a decision is
  `(1 - lam) * R + lam * G` folded backward from the outcome, `R` the root value of the seat's next searched decision
  after it and `G` that decision's own return; a decision with no later searched decision of its seat has the outcome.
  The objective is still the win probability: the root values are the search's estimates of it, the outcome its
  realisation.
- `future(data)`: training-only auxiliary targets, never read by the search: from the sample's seat's side, both life
  totals at the end of the next turn and of the turn after, and at the end of the next turn the power of the creatures
  each controls and the cards in each hand. Read from the game's own later positions: the last sample of a turn is that
  turn's end as the seat answering it saw it (its own and its opponent's life, hand size and battlefield are public).
  Past the game's last turn, its last position. Scaled to about one: life and power by 20, cards by 7.
"""

from __future__ import annotations

import numpy as np

from . import dataset, model

LAYOUT = model.LAYOUT
KIND = {k: i for i, k in enumerate(LAYOUT["kinds"])}
ZONE = {z: i + 1 for i, z in enumerate(LAYOUT["zones"])}
FEAT = {k: {f["name"]: (f["offset"], f["scale"]) for f in v} for k, v in LAYOUT["features"].items()}
CAT = {c["name"]: c["slot"] for c in LAYOUT["categorical"]}
TYPES = FEAT["object"]["types"][0]
CREATURE_BIT = 1  # `CardType::Creature`
SELF, OPPONENT = 1, 2  # `cat::OWNER` and `cat::CONTROLLER`: 1 + the seat relative to the observer
FUTURE = 8  # life own/opp next turn, life own/opp the turn after, power own/opp next turn, hand own/opp next turn


def num(nums: np.ndarray, kind: str, name: str) -> np.ndarray:
    offset, scale = FEAT[kind][name]
    return nums[..., offset] * scale


def td(data: dataset.Data, lam: float) -> np.ndarray:
    out = np.array(data.outcome, np.float32)
    game, seat, decision, root = data.game, data.seat, data.decision, data.root
    n = len(out)
    start = 0
    while start < n:
        end = start
        while end < n and game[end] == game[start]:
            end += 1
        for s in np.unique(seat[start:end]):
            rows = np.arange(start, end)[seat[start:end] == s]
            # Each decision of the seat, in order, with its root value (the first finite one among its sub-steps).
            decisions = []
            for r in rows:
                if decisions and decisions[-1][0] == decision[r]:
                    decisions[-1][1].append(r)
                    if not np.isfinite(decisions[-1][2]) and np.isfinite(root[r]):
                        decisions[-1][2] = float(root[r])
                else:
                    decisions.append([decision[r], [r], float(root[r])])
            g = float(data.outcome[rows[0]])
            for _, members, value in reversed(decisions):
                for r in members:
                    out[r] = g
                if np.isfinite(value):
                    g = (1.0 - lam) * value + lam * g
        start = end
    return out


def future(data: dataset.Data, log=print) -> np.ndarray:
    n = len(data.outcome)
    game, turn, seat = data.game, data.turn, data.seat
    # Each game's turn ends: the last sample of each turn.
    ends: dict[tuple[int, int], int] = {}
    last_of_game: dict[int, int] = {}
    for r in range(n):
        ends[(int(game[r]), int(turn[r]))] = r
        last_of_game[int(game[r])] = r
    snaps = np.array(sorted(set(ends.values())), np.int64)
    facts = {}
    for k in range(0, len(snaps), 4096):
        rows = snaps[k : k + 4096]
        x = dataset.decode(data.blob, data.offset, rows)
        kind, cats, nums = x["kind"], x["cats"], x["nums"]
        owner, controller, zone = cats[..., CAT["owner"]], cats[..., CAT["controller"]], cats[..., CAT["zone"]]
        player = kind == KIND["player"]
        life = num(nums, "player", "life")
        hand = num(nums, "player", "hand")
        creature = ((kind == KIND["card"]) | (kind == KIND["pooled"])) & (zone == ZONE["battlefield"]) \
            & (nums[..., TYPES + CREATURE_BIT] > 0.5)
        power = np.clip(num(nums, "object", "power"), 0, None) * np.maximum(1.0, np.rint(num(nums, "object", "count")) * (kind == KIND["pooled"]))
        for i, r in enumerate(rows):
            own, opp = player[i] & (owner[i] == SELF), player[i] & (owner[i] == OPPONENT)
            facts[int(r)] = (
                float((life[i] * own).sum()), float((life[i] * opp).sum()),
                float((power[i] * (creature[i] & (controller[i] == SELF))).sum()),
                float((power[i] * (creature[i] & (controller[i] == OPPONENT))).sum()),
                float((hand[i] * own).sum()), float((hand[i] * opp).sum()),
                int(seat[r]),
            )
    log(f"future targets: {len(snaps)} turn ends decoded")
    out = np.zeros((n, FUTURE), np.float32)
    for r in range(n):
        g, t, s = int(game[r]), int(turn[r]), int(seat[r])

        def at(tt: int):
            snap = ends.get((g, tt), last_of_game[g])
            life_own, life_opp, pow_own, pow_opp, hand_own, hand_opp, seen_by = facts[snap]
            if seen_by != s:
                life_own, life_opp = life_opp, life_own
                pow_own, pow_opp = pow_opp, pow_own
                hand_own, hand_opp = hand_opp, hand_own
            return life_own, life_opp, pow_own, pow_opp, hand_own, hand_opp

        a, b = at(t + 1), at(t + 2)
        out[r] = (a[0] / 20, a[1] / 20, b[0] / 20, b[1] / 20, a[2] / 20, a[3] / 20, a[4] / 7, a[5] / 7)
    return out
