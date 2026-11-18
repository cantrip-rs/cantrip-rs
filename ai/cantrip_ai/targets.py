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
- `fates(data)`: training-only per-token targets (review 2026-11-13, item 11), from the sample's seat's own position
  `FATE_TURNS` turns on (its last answer at or before the end of turn t + 2; past the game's end, its last): for a
  permanent on the battlefield, the share of the permanents of its card and controller now that are still there then;
  for a card in the seat's own hand, the share of its copies there now that have left the hand by then. The encoding
  gives an object no identity a later position could find it by, so a permanent is known by its card and controller
  and a card in hand by its card: for a card with one copy in play these are exactly "is this permanent still on the
  battlefield" and "has this card left the hand" (cast, played, discarded: leaving the hand is all a later position
  can tell, not how). Both zones are seen alike by the seat in both positions (the battlefield is public, the hand its
  own), so no hidden card is a target. `fate_labels` makes a batch's targets and their mask.
- `game_cap(data, cap)`: per-sample value-loss weights, at most `cap` positions' worth a game an epoch
  (`train.py --value-games-cap`).
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
FATES = 2  # a permanent's like still on the battlefield, a card in hand's like gone from the hand
FATE_TURNS = 2
# A fate key: the card row, then the zone's code (`FATE_BATTLEFIELD`, `FATE_HAND`), then the controller's categorical.
FATE_BATTLEFIELD, FATE_HAND = 1, 2


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


def _fate_tokens(kind, card, cats, nums):
    """Each token's fate key and count [.., T] (key 0: no fate), from decoded inputs with training-index card rows: a
    known card on the battlefield (any controller) or in the observer's own hand, a pooled token counting its copies."""
    zone, owner, controller = cats[..., CAT["zone"]], cats[..., CAT["owner"]], cats[..., CAT["controller"]]
    objects = ((kind == KIND["card"]) | (kind == KIND["pooled"])) & (card > 0)
    field = objects & (zone == ZONE["battlefield"])
    hand = objects & (zone == ZONE["hand"]) & (owner == SELF)
    code = np.where(field, FATE_BATTLEFIELD, np.where(hand, FATE_HAND, 0))
    key = np.where(code > 0, card.astype(np.int64) * 64 + code * 16 + controller, 0)
    count = np.where(kind == KIND["pooled"], np.maximum(1.0, np.rint(num(nums, "object", "count"))), 1.0)
    return key, count.astype(np.float32)


def _totals(key: np.ndarray, count: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """The keys of one position (0 left out), each once, sorted, with their summed counts."""
    keep = key > 0
    keys, inverse = np.unique(key[keep], return_inverse=True)
    return keys, np.bincount(inverse, weights=count[keep], minlength=len(keys)).astype(np.float32)


def fates(data: dataset.Data, log=print):
    """Sets `data.fate_snap` (i64 [N], each sample's later position as an index into the snapshots) and the snapshots'
    fate keys and counts, ragged (`fate_offset`, `fate_key` u32, `fate_count` f32), for `fate_labels`."""
    n = len(data.outcome)
    game, seat, turn = data.game.astype(np.int64), data.seat.astype(np.int64), data.turn.astype(np.int64)
    key = (game << 24) | (seat << 20) | turn
    order = np.lexsort((np.arange(n), key))
    sorted_key = key[order]
    later = (game << 24) | (seat << 20) | np.minimum(turn + FATE_TURNS, (1 << 20) - 1)
    # The seat's last sample at or before the end of turn t + 2: never before the sample itself, which is one of them.
    snap_rows = order[np.searchsorted(sorted_key, later, side="right") - 1]
    snaps, which = np.unique(snap_rows, return_inverse=True)
    keys, counts, sizes = [], [], np.zeros(len(snaps), np.int64)
    for k in range(0, len(snaps), 4096):
        rows = snaps[k : k + 4096]
        x = dataset.decode(data.blob, data.offset, rows)
        card = dataset.cards(data, rows, x["card"])
        token_key, token_count = _fate_tokens(x["kind"], card, x["cats"], x["nums"])
        for i in range(len(rows)):
            ks, cs = _totals(token_key[i], token_count[i])
            keys.append(ks)
            counts.append(cs)
            sizes[k + i] = len(ks)
    data.fate_snap = which.astype(np.int64).reshape(-1)
    data.fate_offset = np.concatenate([np.zeros(1, np.int64), np.cumsum(sizes)])
    data.fate_key = (np.concatenate(keys) if keys else np.zeros(0)).astype(np.uint32)
    data.fate_count = (np.concatenate(counts) if counts else np.zeros(0)).astype(np.float32)
    log(f"fate targets: {len(snaps)} later positions decoded")


def fate_labels(data: dataset.Data, rows: np.ndarray, inputs: dict) -> dict:
    """A batch's fate targets `fates` [B, T, FATES] and their mask `fate_mask` (the same shape): channel 0 on the
    battlefield's tokens, channel 1 on the own hand's (`fates`)."""
    token_key, token_count = _fate_tokens(inputs["kind"], inputs["card"], inputs["cats"], inputs["nums"])
    shape = inputs["kind"].shape
    target = np.zeros((*shape, FATES), np.float32)
    mask = np.zeros((*shape, FATES), np.float32)
    for i, r in enumerate(rows):
        now_keys, now_counts = _totals(token_key[i], token_count[i])
        if not len(now_keys):
            continue
        s = data.fate_snap[r]
        a, b = data.fate_offset[s], data.fate_offset[s + 1]
        later_keys, later_counts = data.fate_key[a:b].astype(np.int64), data.fate_count[a:b]
        then = np.zeros(len(now_keys), np.float32)
        if len(later_keys):
            at = np.minimum(np.searchsorted(later_keys, now_keys), len(later_keys) - 1)
            then = np.where(later_keys[at] == now_keys, later_counts[at], 0.0)
        kept = np.minimum(1.0, then / now_counts)
        tokens = np.flatnonzero(token_key[i] > 0)
        token_keys = token_key[i][tokens]
        share = kept[np.searchsorted(now_keys, token_keys)]
        field = (token_keys // 16) % 4 == FATE_BATTLEFIELD
        target[i, tokens[field], 0] = share[field]
        mask[i, tokens[field], 0] = 1.0
        target[i, tokens[~field], 1] = 1.0 - share[~field]
        mask[i, tokens[~field], 1] = 1.0
    return dict(fates=target, fate_mask=mask)


def game_cap(data: dataset.Data, cap: float, trivial: float = 1.0) -> np.ndarray:
    """Per sample, min(1, cap / the game's samples dealt an epoch): a game's positions count in the value loss as at
    most `cap` positions do, however long the game (a game of 480 positions shares one outcome, and a network that
    sees them all learns the game). Its samples dealt an epoch: those with a choice, those without at the rate
    `trivial` (as `dataset.batches` keeps them), each times its directory's repeat."""
    dealt = np.where(data.legal > 1, 1.0, trivial) * data.repeat
    per_game = np.bincount(data.game, weights=dealt, minlength=data.games)
    return np.minimum(1.0, cap / np.maximum(per_game[data.game], 1e-9)).astype(np.float32)
