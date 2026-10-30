"""The belief over what an opponent holds, learned from play and measured against the prior.

The prior is exact: in Decklist mode an opponent's unseen cards are their list less what the seat has seen
(`pool_card` tokens with `from_pool`, `count` copies each; `pool_left` of them in all, on the opponent's player
token), and the hidden part of their hand is a draw of `H` of them without replacement, `H` their hand size less the
cards of it the seat knows. So the copies of a card in that part of the hand are hypergeometric, and so is any class
of cards.

The head reads a trained network's encoder (`model.Net.body`) and adds a learned correction to the prior's log
probabilities, never replaces them: the correction's last layer starts at zero, so an untrained head is the prior,
and a count the prior rules out (more copies than are unseen, or than the hidden hand holds) stays ruled out. Its
outputs:

- per opponent pool card, the copies of it in the hidden hand, 0 to 4 (`MAX_COPIES`);
- per position, classes of the hidden hand (`CLASSES`): a counterspell castable now, an instant-speed removal spell
  castable now, a combat trick castable now (each "castable" as mana value at most the opponent's untapped lands, a
  diagnostic that ignores colors), a sweeper at all, and its lands (0, 1, 2 or more). The classes' cards are named in
  a JSON file (`--threats`), a definition to review, not a rule.

The labels are the true hidden state the recorder keeps with every sample (`opponent_hand`), less the cards of the
hand the seat sees. A position the labels and the prior disagree on (the hidden part of the hand is not a draw from
the pool: a card seen and shuffled back, a pool cut short by the token cap) is counted and left out of the measure.

    python -m cantrip_ai.belief --data DIR [DIR ...] --checkpoint CKPT --threats FILE --out DIR [--unfreeze] [--steps N]

writes `report.json` (log-loss and Brier against the prior per turn, calibration of each class) and the head's
weights to OUT.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import torch
from torch import nn
from torch.nn import functional as F

from . import dataset, model
from .train import choose_device

LAYOUT = model.LAYOUT
KIND = {k: i for i, k in enumerate(LAYOUT["kinds"])}
ZONE = {z: i + 1 for i, z in enumerate(LAYOUT["zones"])}
FEAT = {k: {f["name"]: (f["offset"], f["scale"]) for f in v} for k, v in LAYOUT["features"].items()}
CAT = {c["name"]: c["slot"] for c in LAYOUT["categorical"]}
TYPES = FEAT["object"]["types"][0]
LAND_BIT = 4  # `CardType::Land`
OPPONENT = 2  # `cat::OWNER` and `cat::CONTROLLER`: 1 + the seat relative to the observer
MAX_COPIES = 4
CLASSES = ("counter", "removal", "trick", "sweeper")
NOW = ("counter", "removal", "trick")  # castable now; a sweeper at all
TURN_BANDS = ((0, 0), (1, 2), (3, 4), (5, 6), (7, 9), (10, 14), (15, 99))
ROOT = Path(__file__).resolve().parents[2]


def num(nums: np.ndarray, kind: str, name: str) -> np.ndarray:
    offset, scale = FEAT[kind][name]
    return nums[..., offset] * scale


def mana_value(cost: str) -> int:
    total = 0
    for sym in re.findall(r"\{([^}]*)\}", cost or ""):
        if sym.isdigit():
            total += int(sym)
        elif sym != "X":
            total += 1
    return total


def card_json(name: str) -> dict | None:
    slug = re.sub(r"[^a-z0-9]+", "-", name.lower().replace("'", "").replace(",", "")).strip("-")
    path = ROOT / "content" / "mtg" / "cards" / f"{slug}.json"
    if not path.exists():
        return None
    d = json.loads(path.read_text(encoding="utf-8"))
    return d["faces"][0] | {k: v for k, v in d.items() if k != "faces" and k not in d["faces"][0]} if d.get("faces") else d


def card_facts_by_name(threats: dict) -> dict:
    """By card name, for the cards the threats file names: mana value (`mv`, any card asked of it), and each class
    as a set of names."""
    class MV(dict):
        def __missing__(self, name):
            d = card_json(name)
            self[name] = mana_value(d.get("manaCost", "")) if d else 99
            return self[name]

        def get(self, name, default=None):
            return self[name]

    return dict(mv=MV(), **{c: set(threats.get(c, ())) for c in CLASSES})


def card_facts(names: list, threats: dict) -> dict[str, np.ndarray]:
    """Per training-index row (row 0 the unknown card): mana value, land, and membership of each class."""
    rows = len(names) + 1
    facts = {k: np.zeros(rows, np.float32) for k in ("mv", "land", *CLASSES)}
    for i, n in enumerate(names, 1):
        d = card_json(n)
        if d:
            facts["mv"][i] = mana_value(d.get("manaCost", ""))
            type_line = d.get("type", "")
            facts["land"][i] = float("Land" in type_line and "Creature" not in type_line)
        for c in CLASSES:
            facts[c][i] = float(n in threats.get(c, ()))
    return facts


@dataclass
class Decisions:
    """The positions of the first sub-step of each decision (or every `every`-th), with their labels."""
    blob: np.ndarray
    offset: np.ndarray
    lut_of: np.ndarray  # u16 [N]: which `luts` entry maps the position's rows onto the training index
    luts: list
    hand_offset: np.ndarray
    hand: np.ndarray  # training-index rows of the opponent's whole hand
    turn: np.ndarray
    game: np.ndarray
    length: np.ndarray
    games: int
    matchup: list  # per game, "deck/opponent deck" of the seat... per sample: see `pair`
    pair: np.ndarray  # u16 [N]: index into `matchup`

    def __len__(self):
        return len(self.turn)


def load(directories: list, base: list, max_games: int | None, every: int, log=print) -> tuple[Decisions, list]:
    names = list(base)
    row_of = {n: i + 1 for i, n in enumerate(names)}
    luts, key_of = [], {}
    blob, offset = bytearray(), [0]
    lut_of, hand, hand_offset, turn, game, length, pair = [], [], [0], [], [], [], []
    matchups: dict[str, int] = {}
    games = 0
    for directory in directories:
        taken = 0
        for path in dataset.shards.shards(directory):
            if max_games is not None and taken >= max_games:
                break
            header, gs = dataset.shards.read(path)
            if header["mode"] != "decklist":
                raise SystemExit(f"{path}: mode {header['mode']}; the belief is measured on Decklist-mode positions")
            theirs = dataset.shard_cards(path, header)
            key = dataset.index_hash(theirs)
            if key not in key_of:
                for n in theirs:
                    if n not in row_of:
                        names.append(n)
                        row_of[n] = len(names)
                key_of[key] = len(luts)
                luts.append(np.array([0] + [row_of[n] for n in theirs], np.uint32))
            which = key_of[key]
            lut = luts[which]
            for g in gs:
                if max_games is not None and taken >= max_games:
                    break
                taken += 1
                decks = g.meta.get("decks") or []
                for s in g.samples:
                    if s.step != 0 or s.decision % every:
                        continue
                    p = s.position
                    blob += p
                    offset.append(len(blob))
                    length.append(p[2])
                    lut_of.append(which)
                    hand.extend(int(lut[c]) if c < len(lut) else 0 for c in s.opponent_hand)
                    hand_offset.append(len(hand))
                    turn.append(s.turn)
                    game.append(games)
                    m = f"{decks[s.seat]}/{decks[1 - s.seat]}" if len(decks) == 2 else "?"
                    pair.append(matchups.setdefault(m, len(matchups)))
                games += 1
            log(f"  {path}: {games} games, {len(turn)} positions")
    data = Decisions(np.frombuffer(blob, np.uint8), np.array(offset, np.int64), np.array(lut_of, np.uint16), luts,
                     np.array(hand_offset, np.int64), np.array(hand, np.uint32), np.array(turn, np.int32),
                     np.array(game, np.int32), np.array(length, np.int32), games,
                     sorted(matchups, key=matchups.get), np.array(pair, np.uint16))
    return data, names


def log_choose(n: torch.Tensor, k: torch.Tensor) -> torch.Tensor:
    return torch.lgamma(n + 1) - torch.lgamma(k + 1) - torch.lgamma(n - k + 1)


def hypergeometric(n_total: torch.Tensor, draws: torch.Tensor, good: torch.Tensor, k: torch.Tensor) -> torch.Tensor:
    """log P(k of the `draws` drawn are among the `good` of `n_total`); -inf where impossible."""
    ok = (k >= 0) & (k <= good) & (k <= draws) & (draws - k <= n_total - good) & (draws <= n_total)
    kk, g, n, d = k.clamp(min=0), good.clamp(min=0), n_total.clamp(min=0), draws.clamp(min=0)
    lp = log_choose(g, kk.clamp(max=g)) + log_choose((n - g).clamp(min=0), (d - kk).clamp(min=0).clamp(max=(n - g).clamp(min=0))) - log_choose(n, d.clamp(max=n))
    return torch.where(ok, lp, torch.full_like(lp, -math.inf))


def batch_features(inputs: dict, rows: np.ndarray | None, data: Decisions | None, facts: dict, device) -> dict:
    """The prior and the labels of a decoded batch (its card rows on the training index); with no `data`, the prior
    alone (the labels left at zero)."""
    kind, card, cats, nums = inputs["kind"], inputs["card"], inputs["cats"], inputs["nums"]
    owner, zone, controller = cats[..., CAT["owner"]], cats[..., CAT["zone"]], cats[..., CAT["controller"]]
    is_pool = (kind == KIND["pool_card"]) & (owner == OPPONENT) & (num(nums, "pool_card", "from_pool") > 0.5)
    count = np.rint(num(nums, "pool_card", "count")) * is_pool
    opp_player = (kind == KIND["player"]) & (owner == OPPONENT)
    hand_size = np.rint((num(nums, "player", "hand") * opp_player).sum(1))
    pool_left = np.rint((num(nums, "player", "pool_left") * opp_player).sum(1))
    # Cards of the hand the seat knows: seen ones, and hidden ones it knows the identity of (a card it saw go back).
    known = (kind == KIND["card"]) | ((kind == KIND["hidden"]) & (num(nums, "object", "identity_known") > 0.5))
    known_in_hand = known & (owner == OPPONENT) & (zone == ZONE["hand"])
    hidden_hand = hand_size - known_in_hand.sum(1)
    lands = (kind == KIND["card"]) & (controller == OPPONENT) & (zone == ZONE["battlefield"]) & (nums[..., TYPES + LAND_BIT] > 0.5)
    untapped = (lands & (num(nums, "object", "tapped") < 0.5)).sum(1)

    B, T = kind.shape
    k_true = np.zeros((B, T), np.int64)
    usable = np.ones(B, bool)
    land_count = np.zeros(B, np.int64)
    has = {c: np.zeros(B, np.float32) for c in CLASSES}
    for i, r in enumerate(rows if data is not None else ()):
        whole = [int(c) for c in data.hand[data.hand_offset[r] : data.hand_offset[r + 1]]]
        for c in card[i][known_in_hand[i]].tolist():
            if c in whole:
                whole.remove(c)
        if len(whole) != hidden_hand[i] or pool_left[i] < hidden_hand[i]:
            usable[i] = False
        counts = {}
        for c in whole:
            counts[c] = counts.get(c, 0) + 1
        pool_rows = card[i][is_pool[i]].tolist()
        if count[i].sum() != pool_left[i] or any(counts.get(c, 0) > n for c, n in zip(pool_rows, count[i][is_pool[i]])) \
                or sum(counts.get(c, 0) for c in pool_rows) != len(whole):
            usable[i] = False
        for t in np.flatnonzero(is_pool[i]):
            k_true[i, t] = counts.get(int(card[i, t]), 0)
        land_count[i] = sum(facts["land"][c] > 0 for c in whole)
        for c in CLASSES:
            castable = facts["mv"][whole] <= untapped[i] if c in NOW else np.ones(len(whole), bool)
            has[c][i] = float(bool((facts[c][whole] * castable).sum() > 0)) if whole else 0.0

    t = lambda a, dt=torch.float32: torch.as_tensor(a, dtype=dt, device=device)  # noqa: E731
    N, H, cnt = t(pool_left)[:, None], t(hidden_hand)[:, None], t(count)
    ks = torch.arange(MAX_COPIES + 1, device=device, dtype=torch.float32)
    prior = hypergeometric(N[..., None], H[..., None], cnt[..., None], ks)  # [B,T,5]
    # Classes: hypergeometric over the class's unseen copies.
    pool_rows = torch.as_tensor(card, device=device)
    class_prior = {}
    for c in CLASSES:
        member = t(facts[c])[pool_rows]
        if c in NOW:
            member = member * (t(facts["mv"])[pool_rows] <= t(untapped)[:, None]).float()
        good = (member * cnt).sum(1)
        p0 = hypergeometric(N[:, 0], H[:, 0], good, torch.zeros_like(good)).exp()
        class_prior[c] = (1 - p0).clamp(1e-6, 1 - 1e-6)
    good = (t(facts["land"])[pool_rows] * cnt).sum(1)
    l0 = hypergeometric(N[:, 0], H[:, 0], good, torch.zeros_like(good)).exp()
    l1 = hypergeometric(N[:, 0], H[:, 0], good, torch.ones_like(good)).exp()
    land_prior = torch.stack([l0, l1, (1 - l0 - l1).clamp(min=0)], -1).clamp(min=1e-9)
    land_prior = land_prior / land_prior.sum(-1, keepdim=True)
    tokens_ok = t(is_pool, torch.bool) & t(usable, torch.bool)[:, None] & (t(k_true, torch.long) <= MAX_COPIES)
    tokens_ok &= torch.isfinite(prior.gather(-1, t(k_true, torch.long).clamp(max=MAX_COPIES)[..., None]).squeeze(-1))
    return dict(prior=prior, k=t(k_true, torch.long).clamp(max=MAX_COPIES), tokens=tokens_ok, usable=t(usable, torch.bool),
                class_prior=class_prior, has={c: t(has[c]) for c in CLASSES}, land_prior=land_prior,
                lands=t(np.minimum(land_count, 2), torch.long))


class Head(nn.Module):
    """A correction to the prior's log probabilities: per pool card over its copies in the hidden hand, and per
    position over the classes. Starts at zero (the prior)."""

    def __init__(self, width: int):
        super().__init__()
        self.card = nn.Sequential(nn.Linear(width, width), nn.GELU(), nn.Linear(width, MAX_COPIES + 1))
        self.classes = nn.Sequential(nn.Linear(2 * width, width), nn.GELU(), nn.Linear(width, len(CLASSES) + 3))
        for last in (self.card[-1], self.classes[-1]):
            nn.init.zeros_(last.weight)
            nn.init.zeros_(last.bias)

    def forward(self, x, summary, f):
        card = (f["prior"] + self.card(x)).log_softmax(-1)  # -inf stays -inf
        out = self.classes(summary)
        classes = {c: torch.logit(f["class_prior"][c]) + out[:, i] for i, c in enumerate(CLASSES)}
        lands = (f["land_prior"].log() + out[:, len(CLASSES):]).log_softmax(-1)
        return card, classes, lands


class Belief:
    """A trained head beside its network, for reading the belief at positions (`marginals`)."""

    def __init__(self, directory: str | Path, checkpoint: str | Path, threats: str | Path, device="auto"):
        self.device = choose_device(device)
        ck = torch.load(checkpoint, map_location="cpu", weights_only=False)
        saved = torch.load(Path(directory) / "belief.pt", map_location="cpu", weights_only=False)
        self.names = model.checkpoint_cards(ck, checkpoint)
        self.facts = card_facts(self.names, json.loads(Path(threats).read_text(encoding="utf-8")))
        config = model.Config(**ck["config"])
        self.net = model.build(config, ck["state"]["table"].numpy())
        self.net.load_state_dict(saved["body"] or ck["state"])
        self.net.to(self.device).eval()
        self.head = Head(config.width).to(self.device)
        self.head.load_state_dict(saved["head"])
        self.head.eval()
        self.row_of = {n: i + 1 for i, n in enumerate(self.names)}

    @torch.no_grad()
    def marginals(self, positions: list[bytes], names: list) -> list[dict]:
        """For each compact position (its card rows of the index `names`, row i + 1 being names[i]): by opponent pool
        card name, the prior's and the head's probabilities of 0 to 4 copies in the hidden hand."""
        lut = np.array([0] + [self.row_of.get(n, 0) for n in names], np.int64)
        out = []
        for start in range(0, len(positions), 64):
            chunk = positions[start : start + 64]
            blob = np.frombuffer(b"".join(chunk), np.uint8)
            offset = np.cumsum([0] + [len(p) for p in chunk]).astype(np.int64)
            x = dataset.decode(blob, offset, np.arange(len(chunk)))
            x["card"] = lut[x["card"]]
            f = batch_features(x, None, None, self.facts, self.device)
            tens = {k: torch.as_tensor(v, device=self.device) for k, v in x.items() if k != "legal"}
            body, summary = self.net.body(tens["kind"], tens["card"], tens["cats"], tens["ptrs"], tens["nums"])
            card, _, _ = self.head(body, summary, f)
            prior = f["prior"].log_softmax(-1).exp().cpu().numpy()
            head = card.exp().cpu().numpy()
            is_pool = ((x["kind"] == KIND["pool_card"]) & (x["cats"][..., CAT["owner"]] == OPPONENT)
                       & (num(x["nums"], "pool_card", "from_pool") > 0.5))
            for i in range(len(chunk)):
                m = {}
                for t in np.flatnonzero(is_pool[i]):
                    row = int(x["card"][i, t])
                    if row > 0:
                        m[self.names[row - 1]] = (prior[i, t].tolist(), head[i, t].tolist())
                out.append(m)
        return out


def losses(card, classes, lands, f):
    tok = f["tokens"]
    nll = -card.gather(-1, f["k"][..., None]).squeeze(-1)
    card_loss = (nll * tok).sum() / tok.sum().clamp(min=1)
    u = f["usable"].float()
    cls = sum((F.binary_cross_entropy_with_logits(classes[c], f["has"][c], reduction="none") * u).sum() for c in CLASSES)
    land = (F.nll_loss(lands, f["lands"], reduction="none") * u).sum()
    return card_loss + (cls + land) / u.sum().clamp(min=1) / (len(CLASSES) + 1)


def run(args):
    device = choose_device(args.device)
    started = time.time()
    checkpoint = torch.load(args.checkpoint, map_location="cpu", weights_only=False)
    base = model.checkpoint_cards(checkpoint, args.checkpoint)
    data, names = load(args.data, base, args.max_games, args.every, log=lambda *_: None)
    if len(names) != len(base):
        raise SystemExit(f"the data names {len(names) - len(base)} cards the checkpoint does not have")
    print(f"{data.games} games, {len(data)} positions ({time.time() - started:.0f} s)", flush=True)
    threats = json.loads(Path(args.threats).read_text(encoding="utf-8"))
    facts = card_facts(names, threats)
    config = model.Config(**checkpoint["config"])
    net = model.build(config, checkpoint["state"]["table"].numpy())
    net.load_state_dict(checkpoint["state"])
    net.to(device)
    head = Head(config.width).to(device)
    params = list(head.parameters()) + (list(net.parameters()) if args.unfreeze else [])
    if not args.unfreeze:
        net.requires_grad_(False)
    optimizer = torch.optim.AdamW([dict(params=list(head.parameters()), lr=args.lr),
                                   *([dict(params=[p for p in net.parameters() if p.requires_grad], lr=args.body_lr)] if args.unfreeze else [])],
                                  weight_decay=0.01)
    rng = np.random.default_rng(args.seed)
    held_games = rng.random(data.games) < args.held_out
    held = held_games[data.game]
    train_rows, test_rows = np.flatnonzero(~held), np.flatnonzero(held)

    class View:  # what `dataset.batches` reads
        length = data.length.astype(np.uint8) if data.length.max() < 256 else data.length
        legal = np.full(len(data), 2, np.uint8)
        repeat = np.ones(len(data), np.float32)

    def inputs_of(rows):
        x = dataset.decode(data.blob, data.offset, rows)
        out = np.empty_like(x["card"])
        which = data.lut_of[rows]
        for k in np.unique(which):
            at = which == k
            out[at] = data.luts[k][x["card"][at]]
        x["card"] = out
        return x

    def forward(rows, train: bool):
        x = inputs_of(rows)
        f = batch_features(x, rows, data, facts, device)
        tens = {k: torch.as_tensor(v, device=device) for k, v in x.items() if k != "legal"}
        with torch.set_grad_enabled(train and args.unfreeze):
            body, summary = net.body(tens["kind"], tens["card"], tens["cats"], tens["ptrs"], tens["nums"])
        with torch.set_grad_enabled(train):
            out = head(body, summary, f)
        return out, f

    total = args.steps
    step = 0
    print(f"training the head{' and the body' if args.unfreeze else ''} for {total} steps on {len(train_rows)} positions, "
          f"{len(test_rows)} held out ({held_games.sum()} games)", flush=True)
    while step < total:
        for rows in dataset.batches(View, train_rows, args.batch, rng):
            if step >= total:
                break
            lr_scale = min(1.0, (step + 1) / 200) * 0.5 * (1 + math.cos(math.pi * step / total))
            for g, base_lr in zip(optimizer.param_groups, (args.lr, args.body_lr)):
                g["lr"] = base_lr * lr_scale
            (card, classes, lands), f = forward(rows, True)
            loss = losses(card, classes, lands, f)
            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(params, 1.0)
            optimizer.step()
            step += 1
            if step % 200 == 0:
                print(f"step {step}/{total} loss {loss.item():.4f} ({time.time() - started:.0f} s)", flush=True)

    report = evaluate(forward, test_rows, data, View, args.batch)
    report["run"] = dict(vars(args), positions=len(data), games=data.games, seconds=time.time() - started)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "report.json").write_text(json.dumps(report, indent=1))
    torch.save(dict(head=head.state_dict(), body=net.state_dict() if args.unfreeze else None, config=checkpoint["config"],
                    args=vars(args)), out / "belief.pt")
    print(json.dumps(report["summary"], indent=1))


@torch.no_grad()
def evaluate(forward, rows, data, View, batch) -> dict:
    """Per turn band and matchup: the per-card log-loss and Brier score of the prior and of the head, over pool cards whose
    prior is uncertain; the classes' log-loss, Brier and calibration (ten bins) for both."""
    acc = {}

    def add(key, name, value, n=1):
        a = acc.setdefault(key, {})
        a[name] = a.get(name, 0.0) + value
        a[name + "_n"] = a.get(name + "_n", 0.0) + n

    bins = {who: {c: np.zeros((10, 3)) for c in (*CLASSES, "lands0")} for who in ("prior", "head")}
    unusable = 0
    for b in dataset.batches(View, rows, batch, None):
        (card, classes, lands), f = forward(b, False)
        prior = f["prior"].log_softmax(-1)
        tok = f["tokens"] & (prior.exp().max(-1).values < 0.999)
        onehot = F.one_hot(f["k"], MAX_COPIES + 1).float()
        u = f["usable"]
        unusable += int((~u).sum())
        turns = torch.as_tensor(data.turn[b], device=card.device)
        pairs = data.pair[b]
        for who, lp, cl, la in (("prior", prior, {c: torch.logit(f["class_prior"][c]) for c in CLASSES}, f["land_prior"].log()),
                                ("head", card, classes, lands)):
            nll = -lp.gather(-1, f["k"][..., None]).squeeze(-1)
            brier = ((lp.exp() - onehot) ** 2).sum(-1)
            for i in range(len(b)):
                if not bool(u[i]):
                    continue
                band = next(f"{lo}-{hi}" for lo, hi in TURN_BANDS if lo <= int(turns[i]) <= hi)
                m = tok[i]
                for key in ("all", f"turn {band}", f"matchup {data.matchup[pairs[i]]}"):
                    if bool(m.any()):
                        add(key, f"{who}_card_nll", float(nll[i][m].sum()), int(m.sum()))
                        add(key, f"{who}_card_brier", float(brier[i][m].sum()), int(m.sum()))
                    for c in CLASSES:
                        p = float(torch.sigmoid(cl[c][i]))
                        y = float(f["has"][c][i])
                        add(key, f"{who}_{c}_nll", -math.log(max(p if y else 1 - p, 1e-9)))
                        add(key, f"{who}_{c}_brier", (p - y) ** 2)
                        if key == "all":
                            bins[who][c][min(int(p * 10), 9)] += (1, p, y)
                    pl = la[i].exp()
                    add(key, f"{who}_lands_nll", -float(la[i][f["lands"][i]]))
                    if key == "all":
                        p0 = float(pl[0])
                        bins[who]["lands0"][min(int(p0 * 10), 9)] += (1, p0, float(f["lands"][i] == 0))
    table = {key: {k: v / a[k + "_n"] for k, v in a.items() if not k.endswith("_n") and a[k + "_n"]} | {"positions": a.get("prior_counter_nll_n", 0)}
             for key, a in acc.items()}
    calibration = {who: {c: [dict(n=int(r[0]), mean_p=r[1] / r[0], rate=r[2] / r[0]) for r in m if r[0]] for c, m in bm.items()} for who, bm in bins.items()}
    ece = {who: {c: sum(abs(r["mean_p"] - r["rate"]) * r["n"] for r in rows_) / max(1, sum(r["n"] for r in rows_)) for c, rows_ in cm.items()} for who, cm in calibration.items()}
    summary = {key: {k: round(v, 4) for k, v in val.items()} for key, val in sorted(table.items()) if not key.startswith("matchup")}
    return dict(summary=summary, by_key=table, calibration=calibration, ece=ece, unusable=unusable)


def main(argv=None):
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("--data", nargs="+", required=True)
    a.add_argument("--checkpoint", required=True)
    a.add_argument("--threats", required=True)
    a.add_argument("--out", required=True)
    a.add_argument("--unfreeze", action="store_true")
    a.add_argument("--steps", type=int, default=6000)
    a.add_argument("--batch", type=int, default=256)
    a.add_argument("--lr", type=float, default=1e-3)
    a.add_argument("--body-lr", type=float, default=1e-4)
    a.add_argument("--every", type=int, default=1, help="keep every n-th decision")
    a.add_argument("--max-games", type=int)
    a.add_argument("--held-out", type=float, default=0.05)
    a.add_argument("--seed", type=int, default=0)
    a.add_argument("--device", default="auto")
    run(a.parse_args(argv))


if __name__ == "__main__":
    sys.exit(main())
