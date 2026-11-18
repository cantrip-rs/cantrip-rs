"""Train a network by imitation on recorded shards (docs/ai-player-plan.md, "Training").

    python -m cantrip_ai.train --data DIR [DIR ...] --out DIR [--epochs N] [--hours H] [--max-games N]

The loss is the cross-entropy of the chosen option (positions with more than one legal option), the squared error
of the value against the game's outcome, and small weights on the auxiliary targets (the life swing over the turn,
smooth L1 in tens of life; dead by the next turn, binary cross-entropy). The belief head is trained (binary
cross-entropy on whether each pool card is in an opponent's hand) only when the data's mode hides the hands: in Full
mode the hands are in the input, so the belief loss is off.

AdamW, linear warm-up then a cosine to zero over the run. Games, not samples, are held out. Each epoch and the end
write `checkpoint.pt` (config, weights, the data's header, metrics) and `metrics.json`; `--hours` stops the run early
at a wall-clock limit (choose `--epochs` so the cosine finishes inside it).

Expert iteration (Phase 4): the policy target is each sample's visit distribution (the search's root visits where the
data was self-played; one-hot on the chosen option in imitation data, where this is the cross-entropy above), so the
loss is the soft cross-entropy `-sum(visits * log softmax(policy))`. `--value-mix M` moves the value's target toward
the search's root value where a sample has one: `(1 - M) * outcome + M * root`. `--init CHECKPOINT` fine-tunes a
trained network (its config and weights) instead of building a fresh one. `--resume` continues from
`--out/checkpoint.pt`, with the optimizer, step and epoch it holds. `--data` takes `DIR:WEIGHT` (see `dataset`): the
window of generations, the newest weighted most.

The held-out set is games held out of the data loaded, so a chunked run's changes with each chunk. `--eval-data DIR
[DIR ...]` is a fixed evaluation set beside it: at most `--eval-data-games` games from each directory, loaded once at
the start (never unloaded with a chunk), and `--eval-data-samples` of its positions chosen by `--seed` alone, scored at
every `--eval-every` and at the end as `fixed_eval` beside `held_out` in the history, the same positions through a run,
its resumptions and its chunks, and in every run given the same set and seed: curves of different runs and sizes are
on the same positions. Its card names join the training index after the training data's, and a fresh network's table
is made for the training data's names as it would be without the set, the set's own names then added as a fine-tuned
network's new cards are (projected into that basis): the set never shapes the network. `--steps 0` trains nothing
and evaluates `--init`'s network (held out and fixed) as it is.

The card-text table (`embed`) is made here, never checked in. A fresh network's is fitted to the names its shards
name; a fine-tuned one keeps its checkpoint's rows and basis, and each card new to it is projected into that basis.
The checkpoint keeps the names (`cards`) and the basis (`embed`), and export writes the names as the model's
`cards.json`.
"""

from __future__ import annotations

import argparse
import contextlib
import datetime
import json
import math
import os
import sys
import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import torch
import torch.nn.functional as F

from . import dataset, embed, model

AUX_WEIGHT = 0.1
BELIEF_WEIGHT = 0.1


class collate:
    """A batch's tensors from its rows. A class at module level rather than a closure, so a DataLoader worker can be
    handed it by pickling (Windows and macOS start workers by spawning, which pickles the collate function)."""

    def __init__(self, data: dataset.Data):
        self.data = data

    def __call__(self, rows):
        data = self.data
        rows, fresh = dataset.unmark(data, rows)
        inputs = dataset.decode(data.blob, data.offset, rows)
        inputs["card"] = dataset.cards(data, rows, inputs["card"])
        target = dataset.labels(data, rows, inputs)
        if not fresh.all():
            # Policy-only copies of search targets (`dataset.batches`' search_repeat): the other losses skip them.
            target["fresh"] = fresh.astype(np.float32)
        return {k: torch.from_numpy(np.ascontiguousarray(v)) for k, v in {**inputs, **target}.items()}


def loader(data, batches, workers, pin=False):
    return torch.utils.data.DataLoader(batches, batch_size=None, shuffle=False, collate_fn=collate(data),
                                       num_workers=workers, persistent_workers=False, pin_memory=pin,
                                       prefetch_factor=4 if workers else None)


def choose_device(name: str) -> torch.device:
    """`auto` picks CUDA, which is HIP on a ROCm build of PyTorch (torch.version.hip is set)."""
    if name == "auto":
        return torch.device("cuda" if torch.cuda.is_available() else "cpu")
    return torch.device(name)


def to_device(b: dict, device: torch.device) -> dict:
    return {k: v.to(device, non_blocking=True) for k, v in b.items()}


def elapsed(seconds: float) -> str:
    seconds = max(0, int(seconds))
    return f"{seconds // 3600}:{seconds // 60 % 60:02d}:{seconds % 60:02d}" if seconds >= 3600 else f"{seconds // 60}:{seconds % 60:02d}"


class Progress:
    """A one-line step bar on stderr; off when stderr is not a terminal (as under the generation driver's log)."""

    WIDTH = 28

    def __init__(self, enabled: bool):
        self.enabled = enabled
        self.last = ""

    def update(self, step: int, total: int, seen: int, ce: float, top1: float, start: float):
        if not self.enabled:
            return
        now = time.time()
        seconds = max(now - start, 1e-9)
        fraction = min(1.0, step / max(total, 1))
        filled = int(self.WIDTH * fraction)
        rate = seen / seconds
        eta = (total - step) / max(step / seconds, 1e-9)
        line = (f"  {step}/{total} [{'#' * filled}{'-' * (self.WIDTH - filled)}] {fraction:5.1%}  "
                f"{rate:,.0f} samples/s  ce {ce:.4f}  top1 {top1:.4f}  {elapsed(seconds)} / eta {elapsed(eta)}")
        sys.stderr.write("\r" + line + "\x1b[K")
        sys.stderr.flush()
        self.last = line

    def clear(self):
        if self.enabled:
            sys.stderr.write("\r\x1b[K")
            sys.stderr.flush()

    def close(self):
        if self.enabled:
            sys.stderr.write("\n")
            sys.stderr.flush()
            self.enabled = False


class Surprise:
    """KataGo's policy surprise weighting's normaliser (`--surprise-weight`): a running mean of the search targets'
    KL(target || prior), so a target's weight, (1 - S) + S * min(KL / mean, `CAP`), averages about one whatever its
    batch holds (a batch holds a dozen search targets, too few to normalise by alone). The mean is an exponential
    average by batch of the targets' sum over their count, so a batch counts by the targets it holds, and it is not
    used until it is over `SEED` targets (until then `update` returns 0 and the targets weigh 1): a first batch's few
    targets would set every weight after it. A target's ratio to the mean is capped at `CAP`, so one surprising
    position cannot take a batch's whole policy loss."""

    CAP = 10.0
    SEED = 64

    def __init__(self, decay: float = 0.99):
        self.decay = decay
        self.total = 0.0  # the decayed sum of the targets' KL
        self.count = 0.0  # the decayed count of the targets
        self.seen = 0  # the targets seen
        self.mean = None

    def update(self, kl: torch.Tensor) -> float:
        n = kl.numel()
        if n:
            self.total = self.decay * self.total + float(kl.sum())
            self.count = self.decay * self.count + n
            self.seen += n
            if self.seen >= self.SEED:
                self.mean = self.total / self.count
        return self.mean or 0.0

    def weight(self, kl: torch.Tensor, s: float, mean: float) -> torch.Tensor:
        """A target's weight: (1 - S) + S * its KL over the mean, that ratio capped at `CAP`."""
        return (1 - s) + s * torch.clamp(kl / mean, max=self.CAP)


@dataclass
class LossOptions:
    """The training options of the review of 2026-11-13 (docs/ai-training.md, "Training options"). At its defaults the
    loss is what it was without them."""

    value_mix_full: float | None = None  # the root value's share of a full search's value target (None: value_mix)
    value_mix_fast: float | None = None  # ... of a fast search's
    value_loss: str = "mse"  # or "bce": a binary cross-entropy on the value's win logit
    imitation_policy_weight: float = 1.0  # a recorded game's policy targets' weight (imitation: no search in the game)
    surprise_weight: float = 0.0  # S of KataGo's policy surprise weighting, over the search targets
    anchor_kl: float = 0.0  # KL(anchor || network) on self-played positions with a choice that are no search target
    policy_smoothing: float = 0.0  # the uniform share over the legal options mixed into each policy target
    entropy_weight: float = 0.0  # the policy's entropy, rewarded, on the policy targets
    fate_weight: float = 0.0  # the per-token fates (targets.fates)
    q_weight: float = 0.0  # the per-option Q head, towards the search's per-option root values
    distill_share: float = 1.0  # with a teacher, the teacher's share of each policy and value target
    anchor: "Frozen | None" = None  # the frozen policy of `--anchor`
    teacher: "Frozen | None" = None  # the frozen network of `--distill-from`
    surprise: Surprise | None = None


def _rows_mean(per: torch.Tensor, weight: torch.Tensor | None) -> torch.Tensor:
    """A per-row loss's mean over the batch's rows, each weighted (a policy-only copy at 0, a long game's position at
    its cap's share): the weights scale what a row adds, the batch's size still divides."""
    return (per * weight).sum() / per.shape[0]


def losses(net, b, belief: bool, value_mix: float = 0.0, future_weight: float = 0.0, heuristic_weight: float = 0.0,
           residual_l2: float = 0.0, detach_heuristic: bool = False, policy_weight: float = 1.0, distill_mix: float = 0.0,
           opts: LossOptions | None = None):
    o = opts if opts is not None else LossOptions()
    out = net.train_heads(b["kind"], b["card"], b["cats"], b["ptrs"], b["nums"], b["legal"])
    value, policy, swing, dead, belief_logits, future, parts = (
        out[k] for k in ("value", "policy", "swing", "dead", "belief", "future", "parts"))
    legal = b["legal"]
    choice = b["choice"]
    recorded = visits = b["visits"]
    searched = b.get("searched")
    # The search targets (`dataset.search_targets`), and the self-played positions with a choice that are none (the
    # network's own answers and the fast searches'): where `--anchor-kl` holds the policy to the champion's.
    if searched is not None:
        search_rows = choice & (searched == dataset.SEARCHED)
        anchor_rows = (legal.sum(-1) > 1) & (searched != dataset.RECORDED) & ~search_rows
    else:
        search_rows = anchor_rows = torch.zeros_like(choice)
    # 1 for a sample itself, 0 for a policy-only copy of a search target (`dataset.batches`); None: every row is.
    fresh = b.get("fresh")
    n = len(choice)

    anchor_policy = None
    if o.anchor is not None and (o.anchor_kl > 0 or o.surprise_weight > 0):
        wanted = (anchor_rows if o.anchor_kl > 0 else torch.zeros_like(choice)) | (
            search_rows if o.surprise_weight > 0 else torch.zeros_like(choice))
        if bool(wanted.any()):
            anchor_policy = policy.detach().float().clone()
            anchor_policy[wanted] = o.anchor(b, wanted)[1]
    teacher_value = None
    if o.teacher is not None:
        # Distillation: every position with a choice is a policy target, towards the teacher's distribution (mixed with
        # the recorded target where there is one), and the value's target moves to the teacher's value.
        teacher_value, teacher_policy = o.teacher(b)
        taught = torch.softmax(teacher_policy, -1)
        s = o.distill_share
        visits = torch.where(choice.unsqueeze(-1), (1 - s) * visits + s * taught, taught)
        choice = legal.sum(-1) > 1

    target = visits
    if o.policy_smoothing > 0:
        spread = legal.to(visits.dtype)
        target = (1 - o.policy_smoothing) * visits + o.policy_smoothing * spread / spread.sum(-1, keepdim=True).clamp(min=1)
    logp = F.log_softmax(policy, -1)
    ce = -torch.where(target > 0, target * logp, torch.zeros_like(logp)).sum(-1)
    weight = choice
    if o.imitation_policy_weight != 1.0 and searched is not None:
        weight = weight * torch.where(searched == dataset.RECORDED, o.imitation_policy_weight, 1.0)
    # The policy's mean is over its rows as the imitation weight counts them (at 0, the batch's self-played rows alone;
    # at 1, every choice, as before), taken before surprise weighting, which moves weight among the search targets.
    policy_rows = weight.sum().clamp(min=1)
    surprise = 0.0
    if o.surprise_weight > 0 and bool(search_rows.any()):
        # KL(the search's visits || the prior): the champion's (`--anchor`, the network that played the games, as
        # KataGo weights by the prior its search had), else the network's own, as it is now.
        prior = F.log_softmax(anchor_policy, -1) if anchor_policy is not None else logp.detach().float()
        kl = torch.where(recorded > 0, recorded * (torch.log(recorded.clamp(min=1e-12)) - prior),
                         torch.zeros_like(prior)).sum(-1)
        if o.surprise is None:
            o.surprise = Surprise()
        surprise = o.surprise.update(kl[search_rows].detach())
        if surprise > 0:
            scaled = o.surprise.weight(kl, o.surprise_weight, surprise)
            weight = torch.where(search_rows, weight * scaled, weight * 1.0)
    policy_loss = (ce * weight).sum() / policy_rows

    root = b["root"]
    target_value = b["outcome"]
    if "td" in b:
        # The lambda-return over the seat's later searched root values (targets.td), instead of the mix.
        target_value = b["td"]
    elif o.value_mix_full is not None or o.value_mix_fast is not None:
        # By the search's budget: a full search's root (a policy target's) and a fast one's (playout-cap randomisation's
        # value-only searches) each at its own share; an unset one at `value_mix`.
        full = value_mix if o.value_mix_full is None else o.value_mix_full
        quick = value_mix if o.value_mix_fast is None else o.value_mix_fast
        mix = torch.where(b["fast"] > 0, quick, full)
        safe_root = torch.nan_to_num(root, nan=0.0, posinf=0.0, neginf=0.0)
        target_value = torch.where(torch.isfinite(root), (1 - mix) * target_value + mix * safe_root, target_value)
    elif value_mix > 0:
        safe_root = torch.nan_to_num(root, nan=0.0, posinf=0.0, neginf=0.0)
        target_value = torch.where(torch.isfinite(root), (1 - value_mix) * target_value + value_mix * safe_root, target_value)
    if "td" not in b and distill_mix > 0 and "distill" in b:
        # The perfect-information value's mean over the seat's worlds, mixed where one was distilled into the target the
        # root's mix made (the outcome, with the root value at --value-mix's or --value-mix-full/fast's share): the
        # distilled share of the whole, the rest the mix's.
        d = b["distill"]
        target_value = torch.where(torch.isfinite(d), (1 - distill_mix) * target_value + distill_mix * torch.nan_to_num(d), target_value)
    if teacher_value is not None:
        target_value = (1 - o.distill_share) * target_value + o.distill_share * teacher_value
    logit = out["logit"]
    if parts is not None and detach_heuristic:
        # The outcomes train the residual only: the heuristic head is the tactical model's, distilled, and the value
        # loss sees it as given (the same value at play; only where the outcome's gradient goes differs).
        value = torch.tanh(0.5 * (parts[0].detach() + parts[1]))
        logit = parts[0].detach() + parts[1]
    # The rows the value and the auxiliary targets read: the samples themselves, a long game's at its cap's share.
    rows = fresh
    if "value_weight" in b:
        value_rows = b["value_weight"] if rows is None else rows * b["value_weight"]
    else:
        value_rows = rows
    if o.value_loss == "bce":
        # Against the outcome's win probability (a draw is a half), on the win logit: calibrated as a probability.
        per = F.binary_cross_entropy_with_logits(logit.float(), (target_value + 1) / 2, reduction="none")
        value_loss = per.mean() if value_rows is None else _rows_mean(per, value_rows)
    elif value_rows is None:
        value_loss = F.mse_loss(value, target_value)
    else:
        value_loss = _rows_mean((value.float() - target_value) ** 2, value_rows)
    if rows is None:
        swing_loss = F.smooth_l1_loss(swing, b["swing"] / model.SWING_SCALE)
        dead_loss = F.binary_cross_entropy_with_logits(dead, b["dead"])
    else:
        swing_loss = _rows_mean(F.smooth_l1_loss(swing.float(), b["swing"] / model.SWING_SCALE, reduction="none"), rows)
        dead_loss = _rows_mean(F.binary_cross_entropy_with_logits(dead.float(), b["dead"], reduction="none"), rows)
    total = policy_weight * policy_loss + value_loss + AUX_WEIGHT * (swing_loss + dead_loss)
    heuristic_loss = torch.zeros(())
    residual_loss = torch.zeros(())
    if parts is not None:
        h, r = parts
        noted = b["heuristic"]
        known = torch.isfinite(noted)
        if rows is not None:
            known = known & (rows > 0)
        if heuristic_weight > 0 and bool(known.any()):
            # The noted log-odds, in units of 4 so a typical swing is about one; clipped to the range they fall in.
            want = torch.nan_to_num(noted, nan=0.0).clamp(-12.0, 12.0)
            heuristic_loss = (((h - want) / 4.0) ** 2 * known).sum() / known.sum()
            total = total + heuristic_weight * heuristic_loss
        if residual_l2 > 0:
            residual_loss = (r ** 2).mean() if rows is None else _rows_mean(r.float() ** 2, rows)
            total = total + residual_l2 * residual_loss
    future_loss = torch.zeros(())
    if future is not None and "future" in b and future_weight > 0:
        if rows is None:
            future_loss = F.smooth_l1_loss(future, b["future"])
        else:
            future_loss = _rows_mean(F.smooth_l1_loss(future.float(), b["future"], reduction="none").mean(-1), rows)
        total = total + future_weight * future_loss
    belief_loss = torch.zeros(())
    if belief:
        pool = b["pool"].float()
        if rows is not None:
            pool = pool * rows.unsqueeze(-1)
        bce = F.binary_cross_entropy_with_logits(belief_logits, b["in_hand"], reduction="none")
        belief_loss = (bce * pool).sum() / pool.sum().clamp(min=1)
        total = total + BELIEF_WEIGHT * belief_loss
    entropy = torch.zeros(())
    if o.entropy_weight > 0:
        p = logp.exp()
        each = -torch.where(legal, p * logp, torch.zeros_like(logp)).sum(-1)
        entropy = (each * choice).sum() / choice.sum().clamp(min=1)
        total = total - o.entropy_weight * entropy
    anchor_loss = torch.zeros(())
    if o.anchor_kl > 0 and anchor_policy is not None and bool(anchor_rows.any()):
        la = F.log_softmax(anchor_policy[anchor_rows], -1)
        kl = torch.where(legal[anchor_rows], la.exp() * (la - logp[anchor_rows].float()), torch.zeros_like(la)).sum(-1)
        anchor_loss = kl.mean()
        total = total + o.anchor_kl * anchor_loss
    q_loss = torch.zeros(())
    if o.q_weight > 0 and out["q"] is not None and "q" in b:
        q = b["q"]
        known = torch.isfinite(q) & legal
        if rows is not None:
            known = known & (rows.unsqueeze(-1) > 0)
        if bool(known.any()):
            q_loss = ((out["q"].float() - torch.nan_to_num(q)) ** 2 * known).sum() / known.sum()
            total = total + o.q_weight * q_loss
    fate_loss = torch.zeros(())
    if o.fate_weight > 0 and out["fates"] is not None and "fates" in b:
        mask = b["fate_mask"]
        if rows is not None:
            mask = mask * rows[:, None, None]
        each = F.binary_cross_entropy_with_logits(out["fates"].float(), b["fates"], reduction="none")
        fate_loss = (each * mask).sum() / mask.sum().clamp(min=1)
        total = total + o.fate_weight * fate_loss
    top1 = ((policy.argmax(-1) == b["target"]) & choice).sum()
    top1_visits = ((policy.argmax(-1) == visits.argmax(-1)) & choice).sum()
    # The search targets alone, against the search's visits whatever the policy was trained towards.
    recorded_ce = ce if target is recorded else -torch.where(recorded > 0, recorded * logp, torch.zeros_like(logp)).sum(-1)
    search_top1 = ((policy.argmax(-1) == recorded.argmax(-1)) & search_rows).sum()
    return total, dict(policy=policy_loss.item(), value=value_loss.item(), swing=swing_loss.item(), dead=dead_loss.item(),
                       belief=belief_loss.item(), future=future_loss.item(), heuristic=heuristic_loss.item(),
                       residual=residual_loss.item(), top1=top1.item(), top1_visits=top1_visits.item(), choices=choice.sum().item(),
                       n=n, search_ce=float((recorded_ce.detach() * search_rows).sum()), search_top1=search_top1.item(),
                       search_choices=search_rows.sum().item(), anchor=anchor_loss.item(), entropy=entropy.item(),
                       q=q_loss.item(), fates=fate_loss.item(), surprise=surprise)


class Frozen:
    """A checkpoint's network, frozen, reading the training batches (`--anchor`, `--distill-from`): its card table made
    for the training index as a fine-tuned network's is (its own names' rows kept, each name new to it added), and a
    batch's card rows mapped onto it. Its `state` is what it plays with (an averaged one's average)."""

    def __init__(self, path: str, index: list, known: dict, header: dict, device: torch.device, log=print):
        ck = torch.load(path, map_location="cpu", weights_only=False)
        theirs = {k: ck.get("header", {}).get(k) for k in ("schema", "encoder_hash")}
        if theirs != {k: header.get(k) for k in ("schema", "encoder_hash")}:
            raise SystemExit(f"{path}: encoding {theirs}, the training data's {header}")
        names = model.checkpoint_cards(ck, Path(path))
        seen = set(names)
        own = list(names) + [n for n in index if n not in seen]
        table, _, features = card_inputs(ck, path, names, own, "text", known, log=lambda *_: None)
        self.config = model.Config(**ck["config"])
        net = model.build(self.config, table, 0, features.buffers() if features is not None else None)
        state = model.grow_state(net, dict(ck["state"], table=torch.from_numpy(np.ascontiguousarray(table, np.float32))))
        net.load_state_dict(state)
        self.net = net.eval().requires_grad_(False).to(device)
        row = {n: i + 1 for i, n in enumerate(own)}
        self.lut = torch.tensor([0] + [row[n] for n in index], dtype=torch.long, device=device)
        log(f"frozen {path}: {model.config_dict(self.config)['width']}x{self.config.layers}, step {ck.get('step')}, "
            f"{len(own) - len(names)} card names new to it")

    @torch.no_grad()
    def __call__(self, b: dict, rows: torch.Tensor | None = None) -> tuple[torch.Tensor, torch.Tensor]:
        """Its value [B] and policy logits [B, T] (masked) for the batch's rows `rows` (a mask; None: all), in fp32."""
        x = [b[k] if rows is None else b[k][rows] for k in model.INPUTS]
        x[1] = self.lut[x[1]]
        value, policy, *_ = self.net.heads(*x)
        return value.float(), policy.float()


class EMA:
    """An exponential moving average of the network's parameters (`--ema DECAY`), the model a run writes as its
    `state`: export, the gate and the next generation's init read the average, as KataGo exports one (it removes some
    of the step-to-step noise a candidate carries). Buffers are the network's own. It starts at the network's weights
    (a fine-tuned run's average starts at its champion), or at a given state's.

    Its decay warms up as the standard average's does, min(decay, (1 + t) / (10 + t)) at its t-th update, so a fresh
    average follows the weights while it has few of them to average. `updates` is where t starts: an average carried
    from a checkpoint (a resumed run's, a continued one's) continues its count, as the checkpoint wrote it
    (`ema_updates`)."""

    def __init__(self, net: torch.nn.Module, decay: float, state: dict | None = None, updates: int = 0):
        self.decay = decay
        self.updates = updates
        self.shadow = {k: p.detach().clone() for k, p in net.named_parameters()}
        if state is not None:
            for k, v in self.shadow.items():
                if k in state and state[k].shape == v.shape:
                    v.copy_(state[k])

    def decay_at(self, t: int) -> float:
        return min(self.decay, (1.0 + t) / (10.0 + t))

    @torch.no_grad()
    def update(self, net: torch.nn.Module):
        d = self.decay_at(self.updates)
        self.updates += 1
        for k, p in net.named_parameters():
            self.shadow[k].lerp_(p.detach(), 1.0 - d)

    def state_dict(self, net: torch.nn.Module) -> dict:
        """The network's state with the averaged parameters, on the CPU."""
        state = {k: v.detach().cpu() for k, v in net.state_dict().items()}
        state.update({k: v.detach().cpu().clone() for k, v in self.shadow.items()})
        return state

    @contextlib.contextmanager
    def applied(self, net: torch.nn.Module):
        """The network with the averaged parameters in place while inside (evaluation), its own after."""
        kept = {k: p.detach().clone() for k, p in net.named_parameters()}
        with torch.no_grad():
            for k, p in net.named_parameters():
                p.copy_(self.shadow[k])
        try:
            yield net
        finally:
            with torch.no_grad():
                for k, p in net.named_parameters():
                    p.copy_(kept[k])


def decays(name: str, p: torch.nn.Parameter) -> bool:
    """Whether AdamW decays a parameter: not an embedding, nor a 1-d one."""
    return p.dim() >= 2 and "kind" not in name and "cats" not in name and not name.startswith("templates.")


def param_groups(net: torch.nn.Module) -> list[list[str]]:
    """The parameters' names in the optimizer's two groups (decayed, then not), in its order."""
    named = list(net.named_parameters())
    return [[k for k, p in named if decays(k, p)], [k for k, p in named if not decays(k, p)]]


def carry_optimizer(optimizer, net: torch.nn.Module, source: dict, log=print, why: str = "--init-optimizer") -> int:
    """The `--init` checkpoint's AdamW state (moments and step counts) moved onto this run's parameters by name
    (`--init-optimizer`, continuous training; and `--resume` of a run whose parameters changed since its checkpoint,
    a head added): a parameter that grew (new card template keys, the numbers' projection widened by `--log-nums`)
    keeps its old part's moments and starts the rest at zero; a new one (a new head) starts fresh. A checkpoint from
    before checkpoints named their groups' parameters is named from its own configuration (the groups are made the
    same way). Returns how many parameters took a state."""
    saved = source.get("optimizer")
    if not saved:
        log(f"{why}: the checkpoint has no optimizer state; starting fresh")
        return 0
    names = source.get("param_names")
    if names is None:
        names = param_groups(model.from_state(model.Config(**source["config"]), source["state"]))
    old = {k: i for i, k in enumerate(k for group in names for k in group)}
    states = saved["state"]
    carried = 0
    for k, p in net.named_parameters():
        i = old.get(k)
        st = states.get(i) if i is not None else None
        if st is None:
            continue
        moved = {}
        for key, v in st.items():
            if torch.is_tensor(v) and v.dim() > 0:
                if v.dim() != p.dim() or any(a > b for a, b in zip(v.shape, p.shape)):
                    moved = None
                    break
                grown = torch.zeros_like(p, memory_format=torch.preserve_format)
                grown[tuple(slice(0, s) for s in v.shape)] = v.to(p.device)
                moved[key] = grown
            else:
                moved[key] = v.clone() if torch.is_tensor(v) else v
        if moved:
            optimizer.state[p] = moved
            carried += 1
    log(f"{why}: carried the optimizer state of {carried} of {len(list(net.parameters()))} parameters")
    return carried


@torch.no_grad()
def evaluate(net, data, batches, workers, belief: bool, device: torch.device) -> dict:
    net.eval()
    sums = dict(policy=0.0, value=0.0, swing=0.0, dead=0.0, belief=0.0, top1=0, top1_visits=0, choices=0, n=0, sign=0, first=0,
                heuristic=0.0, residual=0.0, head_mse=0.0, noted_mse=0.0, noted=0, search_ce=0.0, search_top1=0,
                search_choices=0)
    # In this process, whatever `workers` says: an evaluation's workers would attach the shared training data again beside
    # the training loader's own, and on Windows a window near the machine's limits then fails to attach (WinError 1450,
    # gen2 of the templates line at 8.6M samples). The batches are the same either way.
    del workers
    for b in loader(data, batches, 0, device.type == "cuda"):
        b = to_device(b, device)
        # The value held out is against the outcome, whatever target it was trained on.
        b.pop("td", None)
        b.pop("value_weight", None)
        _, m = losses(net, b, belief, heuristic_weight=1.0, residual_l2=1.0)
        value = net.heads(b["kind"], b["card"], b["cats"], b["ptrs"], b["nums"], b["legal"])[0]
        if net.heuristic is not None:
            # The residual design's parts (model.Config.residual): the heuristic head's fit to the noted heuristic, the
            # residual's size, and as values against the outcome the heuristic head alone and the noted heuristic itself.
            h, r = net.value_parts(net.body(b["kind"], b["card"], b["cats"], b["ptrs"], b["nums"])[1])[1]
            known = torch.isfinite(b["heuristic"])
            k = int(known.sum())
            sums["noted"] += k
            sums["heuristic"] += m["heuristic"] * k
            sums["residual"] += float((r ** 2).sum())
            sums["head_mse"] += float(((torch.tanh(0.5 * h) - b["outcome"]) ** 2).sum())
            if k:
                noted = torch.tanh(0.5 * b["heuristic"][known].clamp(-12.0, 12.0))
                sums["noted_mse"] += float(((noted - b["outcome"][known]) ** 2).sum())
        sums["sign"] += ((value > 0) == (b["outcome"] > 0)).sum().item()
        # The baseline: always the first legal option (passing, usually).
        sums["first"] += ((b["legal"].int().argmax(-1) == b["target"]) & b["choice"]).sum().item()
        n, c = m["n"], m["choices"]
        for k in ("value", "swing", "dead", "belief"):
            sums[k] += m[k] * n
        sums["policy"] += m["policy"] * c
        sums["top1"] += m["top1"]
        sums["top1_visits"] += m["top1_visits"]
        sums["choices"] += c
        sums["n"] += n
        for k in ("search_ce", "search_top1", "search_choices"):
            sums[k] += m[k]
    net.train()
    n, c = max(sums["n"], 1), max(sums["choices"], 1)
    return dict(samples=sums["n"], choices=sums["choices"], top1=sums["top1"] / c, top1_visits=sums["top1_visits"] / c, first_legal=sums["first"] / c, policy_ce=sums["policy"] / c,
                value_mse=sums["value"] / n, value_sign=sums["sign"] / n, swing=sums["swing"] / n, dead=sums["dead"] / n,
                belief=sums["belief"] / n if belief else None,
                # The search targets alone (`dataset.search_targets`): how the policy reads the searches it is fitted to.
                search_choices=sums["search_choices"], search_policy_ce=sums["search_ce"] / max(sums["search_choices"], 1),
                search_top1=sums["search_top1"] / max(sums["search_choices"], 1),
                **(dict(heuristic=sums["heuristic"] / max(sums["noted"], 1), residual_ms=sums["residual"] / n,
                        heuristic_head_mse=sums["head_mse"] / n, noted_mse=sums["noted_mse"] / max(sums["noted"], 1),
                        noted=sums["noted"]) if net.heuristic is not None else {}))


def deck_names(spec: str | None) -> set[str] | None:
    """`--decks`: a directory's decklists by name (the file stem), or names separated by commas."""
    if not spec:
        return None
    if Path(spec).is_dir():
        return {p.stem for p in Path(spec).glob("*.txt")}
    return {n.strip() for n in spec.split(",") if n.strip()}


def card_inputs(source: dict | None, source_path, base: list, names: list, mode: str, known: dict, log=print,
                table_source: dict | None = None):
    """The card table (and its basis) and, for a templates network, its features, for the index `names`: a
    checkpoint's rows kept as they were and the new names' rows added, or all made fresh. `mode` is a fresh network's
    `Config.card_features`; a checkpoint's own config decides for it. `table_source` (`fingerprints.state`) makes a
    fresh network's table the cards' fingerprints, alone or beside the text's rows; a checkpoint's own decides for it."""
    recorded = source.get("card_table") if source is not None else table_source
    if recorded is not None and recorded.get("source") == "fingerprints":
        from . import fingerprints  # noqa: PLC0415
        fp = fingerprints.rows(names, recorded["path"], recorded["dim"])
        if fingerprints.table_of(str(recorded["path"]), recorded["dim"])[2] != recorded["hash"]:
            raise SystemExit(f"{recorded['path']}: the fingerprints are not the ones {source_path or 'asked for'} was made with")
        basis = None
        if recorded["text"]:
            basis = embed.Basis.from_state(source["embed"]) if source is not None and source.get("embed") else None
            text, basis = embed.rows(names, basis, known)
            fp = np.concatenate([text, fp], axis=1)
        if source is not None:
            # The checkpoint's rows kept as they were and the new names' added. Its fingerprints must be the file's
            # (another file is another table); a card whose text changed since (an Oracle update) keeps the row the
            # network was trained with, as a text network's does (`embed.extend`).
            kept = source["state"]["table"].numpy()
            prints = fp.shape[1] - (text.shape[1] if recorded["text"] else 0)
            if not np.allclose(fp[: kept.shape[0], -prints:], kept[:, -prints:], atol=1e-5):
                raise SystemExit(f"{source_path}: its table is not the fingerprints' it records")
            if recorded["text"]:
                changed = np.flatnonzero(np.abs(fp[: kept.shape[0], :-prints] - kept[:, :-prints]).max(1) > 1e-5)
                if len(changed):
                    shown = ", ".join(base[i - 1] for i in changed[:5] if i > 0)
                    log(f"card table: {len(changed)} card(s) whose text changed keep their trained rows ({shown}"
                        f"{', ...' if len(changed) > 5 else ''})")
            fp[: kept.shape[0]] = kept
        log(f"card table: fingerprints {recorded['path']} ({recorded['dim']} wide){' beside the text' if recorded['text'] else ''}")
        return fp, basis, None
    if source is not None:
        kept = source["state"]["table"].numpy()
        if kept.shape[0] != len(base) + 1:
            raise SystemExit(f"{source_path}: its table has {kept.shape[0]} rows for {len(base)} names")
        basis = (embed.Basis.from_state(source["embed"]) if source.get("embed")
                 else embed.legacy_basis(base, kept, log=log, known=known))
        table = embed.extend(kept, names, basis, known)
        mode = source["config"].get("card_features", "text")
    else:
        table, basis = embed.rows(names, None, known)
    if mode != "templates":
        return table, basis, None
    from . import card_features  # noqa: PLC0415 (only a templates network reads the ability index)

    items = card_features.card_items()
    old = card_features.from_checkpoint(source) if source is not None else None
    if source is not None and old is None:
        raise SystemExit(f"{source_path}: a templates network without its card features")
    features = card_features.extend(old, names, len(base), items) if old is not None else card_features.build(names, items)
    added = len(features.vocab) - (len(old.vocab) if old is not None else 0)
    log(f"card templates: {len(features.ids)} items over {len(features.vocab)} keys ({added} new), "
        f"{features.slots} value slots")
    return table, basis, features


def eval_set(specs: list, games: int | None, samples: int, seed: int, base: list, decks: set[str] | None = None,
             drop_damage_before: float | None = None, workers: int | None = None) -> tuple[dataset.Data, np.ndarray]:
    """`--eval-data`: the directories' positions (at most `games` games from each; every position, none thinned), their
    card rows onto `base` (the training index; each name new to it follows), and the `samples` of them evaluated,
    chosen by `seed` alone: the same positions whatever the training data, its chunks or the index."""
    data = dataset.load(specs, games, log=lambda *_: None, seed=seed, base=base, decks=decks,
                        drop_damage_before=drop_damage_before, workers=workers)
    rows = np.sort(np.random.default_rng([seed, 0xE7A1]).permutation(len(data))[:samples])
    return data, rows


def grown_table(table, basis, features, mode: str, table_source: dict | None, names: list, index: list, known: dict,
                log=print):
    """A fresh network's card inputs, made for the training names `names`, with rows for the names of `index` after
    them (`--eval-data`'s own), added as a fine-tuned network's new cards are: through `card_inputs` from a checkpoint
    of the fresh table."""
    state = dict(table=torch.from_numpy(np.ascontiguousarray(table, np.float32)))
    if features is not None:
        state.update(feat_ids=torch.as_tensor(features.ids), feat_rows=torch.as_tensor(features.rows),
                     feat_values=torch.as_tensor(features.values))
    fresh = dict(state=state, config=dict(card_features=mode), card_table=table_source,
                 embed=basis.state() if basis is not None else None,
                 card_features=features.state() if features is not None else None)
    return card_inputs(fresh, "the training names' table", names, index, mode, known, log)


def posix_time(text: str) -> float:
    """An ISO 8601 date and time with its offset (2026-11-11T11:48:13-08:00), as POSIX seconds."""
    try:
        when = datetime.datetime.fromisoformat(text)
    except ValueError as e:
        raise argparse.ArgumentTypeError(str(e)) from e
    if when.tzinfo is None:
        raise argparse.ArgumentTypeError(f"{text} names no offset (say -08:00 or Z)")
    return when.timestamp()


def main(argv=None):
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("--data", nargs="+", required=True, help="shard directories, DIR or DIR:WEIGHT")
    a.add_argument("--init", help="a checkpoint.pt to fine-tune (its config and weights)")
    a.add_argument("--resume", action="store_true", help="continue from --out/checkpoint.pt, if it is there")
    a.add_argument("--value-mix", type=float, default=0.0, help="the share of the root value in the value's target")
    a.add_argument("--td-lambda", type=float, help="the value's target: the lambda-return over the seat's later root values (targets.td)")
    a.add_argument("--future-weight", type=float, default=0.0, help="train the training-only future head (targets.future) at this weight")
    a.add_argument("--residual", action="store_true", help="a fresh network's value as a residual on a heuristic head (model.Config.residual)")
    a.add_argument("--card-features", choices=["text", "templates"], default="text",
                   help="a fresh network's card input (model.Config.card_features); a checkpoint keeps its own")
    a.add_argument("--fingerprints", help="a fresh network's card table from this cantrip-fingerprint file (fingerprints.py)")
    a.add_argument("--fingerprint-dim", type=int, default=96, help="the fingerprint table's width")
    a.add_argument("--fingerprints-with-text", action="store_true", help="the fingerprint table beside the text's rows")
    a.add_argument("--template-init", choices=["normal", "zero"], default="normal",
                   help="a fresh templates network's key embeddings: N(0,1), or zeros (a key no training game shows stays nothing)")
    a.add_argument("--heuristic-weight", type=float, default=1.0, help="the heuristic head's loss weight, towards the noted heuristic")
    a.add_argument("--residual-l2", type=float, default=0.1, help="the residual's pull towards zero")
    a.add_argument("--distill-mix", type=float, default=0.0,
                   help="the share of the value target that is the distilled perfect-information value (distill_values), "
                        "mixed into the target --value-mix (or --value-mix-full/fast) made of the outcome and the root")
    a.add_argument("--policy-weight", type=float, default=1.0,
                   help="the policy loss's weight: 0 trains a value alone (a perfect-information value on Full positions)")
    a.add_argument("--fast-policy", action="store_true",
                   help="a fast search's visits are a policy target too (dataset.load's fast_policy): for self-play run "
                        "with Gumbel, whose target is softmax(logit + sigma(completed q)) and improves on the prior at "
                        "any budget; visit counts of a 64-simulation PUCT search mostly echo the prior")
    a.add_argument("--searched-policy", action="store_true",
                   help="in self-played games, only the decisions a search answered are policy targets "
                        "(dataset.load's searched_policy); imitation data is unchanged")
    a.add_argument("--detach-heuristic", action="store_true",
                   help="the outcomes train the residual only, never the heuristic head (which the noted heuristic trains)")
    # The review of 2026-11-13 (docs/ai-training.md, "Training options"): each off by default.
    a.add_argument("--search-target-repeat", type=float, default=1.0,
                   help="each search target dealt this many times an epoch, the copies beyond the first policy-only")
    a.add_argument("--target-uses", type=float,
                   help="the run's length: each search target used about this many times (epochs = uses / repeat)")
    a.add_argument("--imitation-policy-weight", type=float, default=1.0,
                   help="the policy loss's weight on games with no search (imitation): 0 takes them out of it")
    a.add_argument("--anchor", help="a frozen checkpoint (the champion) whose policy --anchor-kl and --surprise-weight read")
    a.add_argument("--anchor-kl", type=float, default=0.0,
                   help="KL(anchor || network) at this weight on self-played positions with a choice that are no search target")
    a.add_argument("--surprise-weight", type=float, default=0.0,
                   help="KataGo's policy surprise weighting on the search targets: (1 - S) + S * KL(target || prior) / its mean")
    a.add_argument("--value-mix-full", type=float, help="the root value's share of a full search's value target (else --value-mix)")
    a.add_argument("--value-mix-fast", type=float, help="the root value's share of a fast search's value target (else --value-mix)")
    a.add_argument("--value-games-cap", type=float, default=0.0,
                   help="a game's positions count in the value loss as at most this many do an epoch (0: off)")
    a.add_argument("--value-loss", choices=["mse", "bce"], default="mse",
                   help="the value's loss: squared error on tanh, or a binary cross-entropy on the win logit")
    a.add_argument("--fate-weight", type=float, default=0.0, help="train per-token fates (targets.fates) at this weight")
    a.add_argument("--q-weight", type=float, default=0.0,
                   help="a per-option Q head (model.Config.q_head) trained at this weight where the data has root Q values")
    a.add_argument("--policy-smoothing", type=float, default=0.0,
                   help="this share of each policy target spread evenly over the legal options")
    a.add_argument("--entropy-weight", type=float, default=0.0, help="reward the policy's entropy on its targets at this weight")
    a.add_argument("--distill-from", help="a frozen checkpoint whose policy and value are the targets (a new size, grown)")
    a.add_argument("--distill-share", type=float, default=1.0, help="with --distill-from, the teacher's share of each target")
    a.add_argument("--ema", type=float, default=0.0,
                   help="keep an exponential moving average of the weights at this decay (0.999) and write it as the model")
    a.add_argument("--schedule", choices=["cosine", "constant"], default="cosine",
                   help="after the warm-up, a cosine to zero over the run, or the rate held (continuous training)")
    a.add_argument("--init-optimizer", action="store_true",
                   help="carry --init's optimizer state (and its unaveraged weights, under --ema) into this run")
    a.add_argument("--log-nums", action="store_true",
                   help="the network reads each number clipped and as sign(x) log(1 + |x|) (model.Config.nums_log); "
                        "a checkpoint without it is widened onto an unclipped linear copy, to compute the same for every input")
    a.add_argument("--nums-clip", type=float,
                   help=f"--log-nums' clip of the linear copy (0: none); a fresh network's default {model.Config.nums_clip}, "
                        "a widened checkpoint's 0 (any other clip changes what it computes past the clip)")
    a.add_argument("--out", required=True)
    a.add_argument("--epochs", type=float, default=1.0)
    a.add_argument("--steps", type=int, help="stop after this many steps (the cosine spans them)")
    a.add_argument("--hours", type=float, help="a wall-clock limit")
    a.add_argument("--max-games", type=int, help="at most this many games from each directory (smoke tests)")
    a.add_argument("--decks", help="only the games played among these decks: a decks directory (its .txt names) or "
                                   "names separated by commas")
    a.add_argument("--drop-damage-before", type=posix_time, metavar="ISO_TIME",
                   help="leave out the combat damage assignments of every shard file modified before this time "
                        "(2026-11-11T11:48:13-08:00: until commit c50710e01 their Damage options meant one point, not "
                        "lethal damage to a blocker); kept in the checkpoint's args as POSIX seconds")
    a.add_argument("--chunks", type=int, default=1,
                   help="train on a set larger than memory: its shards in this many chunks, one loaded at a time, an epoch "
                        "each chunk once in a shuffled order; single-option positions are thinned at load (--trivial)")
    a.add_argument("--batch", type=int, default=256)
    a.add_argument("--lr", type=float, default=5e-4)
    a.add_argument("--weight-decay", type=float, default=0.01)
    a.add_argument("--warmup", type=int, default=500)
    a.add_argument("--held-out", type=float, default=0.02, help="the fraction of games held out")
    a.add_argument("--eval-samples", type=int, default=20000)
    a.add_argument("--eval-data", nargs="+", metavar="DIR",
                   help="a fixed evaluation set (shard directories, DIR or DIR:WEIGHT) loaded once and scored beside the "
                        "held-out set at every --eval-every and at the end, as fixed_eval")
    a.add_argument("--eval-data-games", type=int, default=600, help="at most this many games from each --eval-data directory")
    a.add_argument("--eval-data-samples", type=int, default=20000, help="the --eval-data positions scored, chosen by --seed")
    a.add_argument("--references", type=int, default=512, help="held-out positions written for export's references")
    a.add_argument("--eval-every", type=int, default=2000)
    a.add_argument("--checkpoint-every", type=int, default=200, help="write the resume checkpoint this often, without a held-out evaluation")
    a.add_argument("--trivial", type=float, default=0.25, help="the rate single-option positions are kept at")
    a.add_argument("--width", type=int, default=model.Config.width)
    a.add_argument("--layers", type=int, default=model.Config.layers)
    a.add_argument("--heads", type=int, default=model.Config.heads)
    a.add_argument("--ffn", type=int, default=model.Config.ffn)
    a.add_argument("--threads", type=int, default=max(1, (os.cpu_count() or 4) - 2),
                   help="CPU threads for torch (default: all but two)")
    a.add_argument("--workers", type=int, default=2)
    a.add_argument("--load-workers", type=int, default=dataset.default_workers(),
                   help="processes reading the shards as they load (0 or 1: this one alone); the data is the same either way")
    a.add_argument("--precision", choices=["bf16", "fp32"], default="bf16",
                   help="bf16: the forward and backward passes under bfloat16 autocast on CUDA (weights, optimizer and "
                        "losses stay fp32; held-out evaluation runs fp32); fp32 throughout. A CPU device trains fp32")
    a.add_argument("--device", default="auto", help="auto, cpu, cuda (HIP on a ROCm build), or another torch device")
    a.add_argument("--progress", choices=["auto", "on", "off"], default="auto",
                   help="the step bar on stderr: auto is on only when stderr is a terminal")
    a.add_argument("--seed", type=int, default=0)
    args = a.parse_args(argv)

    device = choose_device(args.device)
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    for stale in ("checkpoint.pt.tmp", "metrics.json.tmp"):
        try:
            (out / stale).unlink()
        except FileNotFoundError:
            pass
    log_file = (out / "train.log").open("a")
    bar = None

    def log(*parts):
        if bar is not None:
            bar.clear()
        line = " ".join(str(p) for p in parts)
        print(line, flush=True)
        log_file.write(line + "\n")
        log_file.flush()

    bar = Progress(args.progress == "on" or (args.progress == "auto" and sys.stderr.isatty()))
    torch.set_num_threads(args.threads)
    log(f"device {device}" + (f" ({torch.cuda.get_device_name(device)})" if device.type == "cuda" else ""))
    # bf16 autocast on CUDA: matrix products in bfloat16 on the tensor cores, reductions (softmax, LayerNorm, the
    # losses) in fp32 as autocast keeps them; the fp32 matrix products left (evaluation) may use TF32.
    half = args.precision == "bf16" and device.type == "cuda"
    if device.type == "cuda":
        torch.backends.cuda.matmul.allow_tf32 = True
        torch.backends.cudnn.allow_tf32 = True
        torch.cuda.reset_peak_memory_stats(device)
    log(f"precision {'bf16 autocast' if half else 'fp32'}")
    resumed = None
    if args.resume:
        checkpoint_path = out / "checkpoint.pt"
        if checkpoint_path.exists():
            resumed = torch.load(checkpoint_path, map_location="cpu", weights_only=False)
            log(f"resuming {checkpoint_path} (step {resumed.get('step')})")
        else:
            log(f"no {checkpoint_path}: starting a fresh run")
    start = time.time() - float((resumed or {}).get("seconds", (resumed or {}).get("metrics", {}).get("seconds", 0.0)))
    source = resumed
    source_path = out / "checkpoint.pt"
    if source is None and args.init:
        source = torch.load(args.init, map_location="cpu", weights_only=False)
        source_path = Path(args.init)
    base = model.checkpoint_cards(source, source_path) if source is not None else []
    # The training index: the checkpoint's, then each name the training data adds, then each the fixed set adds.
    index = list(base)
    chunks = max(1, args.chunks)
    # Chunked, the single-option positions are thinned as they load, and every one loaded is trained on.
    trivial = args.trivial if chunks == 1 else 1.0

    spawned = torch.multiprocessing.get_start_method() != "fork"

    def load_chunk(part: int):
        log(f"loading {args.data}" + (f" chunk {part + 1} of {chunks}" if chunks > 1 else ""))
        data = dataset.load(args.data, args.max_games, log=lambda *_: None, seed=args.seed, base=index,
                            chunk=(part, chunks) if chunks > 1 else None, trivial=args.trivial if chunks > 1 else 1.0,
                            decks=deck_names(args.decks), drop_damage_before=args.drop_damage_before,
                            workers=args.load_workers, blob_file=bool(args.workers) and spawned,
                            searched_policy=args.searched_policy, fast_policy=args.fast_policy)
        log(dataset.describe(data), f"({time.time() - start:.0f} s)")
        if args.td_lambda is not None:
            from . import targets
            data.td = targets.td(data, args.td_lambda)
            log(f"value target: the lambda-return, lambda {args.td_lambda} ({time.time() - start:.0f} s)")
        if args.future_weight > 0:
            from . import targets
            data.future = targets.future(data, log=log)
            log(f"future head: weight {args.future_weight} ({time.time() - start:.0f} s)")
        if args.fate_weight > 0:
            from . import targets
            targets.fates(data, log=log)
            log(f"fates: weight {args.fate_weight} ({time.time() - start:.0f} s)")
        if args.value_games_cap > 0:
            from . import targets
            data.value_weight = targets.game_cap(data, args.value_games_cap, trivial)
            log(f"value: at most {args.value_games_cap:g} positions a game an epoch, a sample's mean weight "
                f"{float(data.value_weight.mean()):.3f}")
        if args.search_target_repeat > 1 or args.surprise_weight > 0 or args.target_uses is not None:
            flagged = sum(d.get("per_sample_searched", 0) for d in data.directories)
            log(f"search targets: {int(dataset.search_targets(data).sum())} samples "
                f"({'flagged per sample' if flagged else 'by decision: the shards flag no sample'})")
        if args.workers and spawned:
            # Spawned workers would each be handed a copy of the data; shared, they read the one.
            data = dataset.share(data)
            log(f"the data in shared memory for {args.workers} spawned workers ({time.time() - start:.0f} s)")
        for d in data.directories:
            dropped = f", {d['nonfinite_roots']} non-finite root values ignored" if d["nonfinite_roots"] else ""
            if d.get("dropped_damage"):
                dropped += f", {d['dropped_damage']} combat damage assignments of older shards left out"
            log(f"  {d['directory']} (weight {d['weight']}): {d['games']} games, {d['samples']} samples{dropped}")
        train, held = dataset.split(data, args.held_out, args.seed)
        rng = np.random.default_rng(args.seed)
        held_eval = held[np.sort(rng.permutation(len(held))[: args.eval_samples])] if len(held) else held
        eval_batches = dataset.batches(data, held_eval, 512, None)
        log(f"training on {len(train)} samples, {len(held)} held out ({len(held_eval)} evaluated)")
        return data, train, held, held_eval, eval_batches

    def unload(data):
        blocks = list(data.blocks.values())
        data.blocks.clear()
        for block in blocks:
            try:
                block.close()
            except BufferError:
                pass
        dataset._release(blocks)

    def order(epoch: int) -> list[int]:
        """The chunks in the order epoch `epoch` trains them."""
        return [int(k) for k in np.random.default_rng(args.seed + 7919 * (epoch + 1)).permutation(chunks)] if chunks > 1 else [0]

    # Where the run is: [the place in its epoch's order of the chunk being trained, the step that chunk began at].
    if resumed is not None and "part_step" in resumed:
        position = [int(resumed.get("part", 0)), int(resumed["part_step"])]
        loaded = order(int(resumed.get("epoch", 0)))[position[0]]
    else:
        position = None
        loaded = order(0)[0]
    data, train, held, held_eval, eval_batches = load_chunk(loaded)
    known = embed.texts()
    table_source = None
    if args.fingerprints and source is None:
        from . import fingerprints  # noqa: PLC0415
        table_source = fingerprints.state(args.fingerprints, args.fingerprint_dim, args.fingerprints_with_text)
    eval_data, fixed, fixed_batches = None, np.zeros(0, np.int64), []
    index = list(data.names)
    if args.eval_data:
        log(f"loading the fixed evaluation set {args.eval_data}")
        eval_data, fixed = eval_set(args.eval_data, args.eval_data_games, args.eval_data_samples, args.seed, index,
                                    deck_names(args.decks), args.drop_damage_before, args.load_workers)
        theirs, ours = ({k: d.header.get(k) for k in dataset.SHARED} for d in (eval_data, data))
        if theirs != ours:
            raise SystemExit(f"--eval-data's shards are {theirs}, the training data's {ours}")
        if not len(fixed):
            raise SystemExit(f"no positions in --eval-data {args.eval_data}")
        index = list(eval_data.names)
        log(dataset.describe(eval_data), f"({time.time() - start:.0f} s)")
        for d in eval_data.directories:
            dropped = f", {d['dropped_damage']} combat damage assignments of older shards left out" if d.get("dropped_damage") else ""
            log(f"  {d['directory']}: {d['games']} games, {d['samples']} samples{dropped}")
        log(f"fixed evaluation set: {len(fixed)} of its {len(eval_data)} positions, {len(index) - len(data.names)} card "
            f"names the training data lacks")
        recorded = (resumed or {}).get("eval_data")
        if recorded and (recorded.get("samples"), recorded.get("evaluated")) != (len(eval_data), len(fixed)):
            log(f"warning: the fixed evaluation set is {len(eval_data)} positions ({len(fixed)} scored), the checkpoint's "
                f"was {recorded.get('samples')} ({recorded.get('evaluated')}): its curve does not continue on the same set")
        if args.workers and torch.multiprocessing.get_start_method() != "fork":
            eval_data = dataset.share(eval_data)
        fixed_batches = dataset.batches(eval_data, fixed, 512, None)
    if source is None and len(index) > len(data.names):
        # Made for the training names as without the fixed set, its own names then added as new cards.
        table, basis, features = card_inputs(None, None, [], data.names, args.card_features, known, log, table_source)
        table, basis, features = grown_table(table, basis, features, args.card_features, table_source, data.names, index,
                                             known, log)
    else:
        table, basis, features = card_inputs(source, source_path, base, index, args.card_features, known, log, table_source)
    if table_source is None and source is not None:
        table_source = source.get("card_table")
    new = len(index) - len(base)
    log(f"card index: {len(index)} names, {sum(n in known for n in index)} with text, {new} new to "
        f"{'the checkpoint' if source is not None else 'this network'}")
    belief = data.header["mode"] != "full"
    log(f"mode {data.header['mode']}: the belief loss is {'on' if belief else 'off (the hands are in the input)'}")

    from . import targets

    # The heads and the numbers' reading the options ask for, added to a checkpoint that has none.
    wanted = {}
    if args.future_weight > 0:
        wanted["future"] = targets.FUTURE
    if args.fate_weight > 0:
        wanted["fates"] = targets.FATES
    if args.q_weight > 0:
        wanted["q_head"] = True
    if args.log_nums:
        wanted.update(nums_log=True, nums_clip=model.Config.nums_clip if args.nums_clip is None else args.nums_clip)
    ema_start = None
    if source is not None:
        config = model.Config(**source["config"])
        grown = {k: v for k, v in wanted.items() if not getattr(config, k) and k != "nums_clip"}
        if grown:
            # A checkpoint widened to --log-nums reads its linear copy unclipped unless told otherwise, so it computes
            # what it did for every number (`model.widen_nums`).
            clip = {"nums_clip": 0.0 if args.nums_clip is None else args.nums_clip} if "nums_log" in grown else {}
            if clip.get("nums_clip"):
                log(f"--nums-clip {args.nums_clip}: the widened checkpoint computes what it did only within the clip")
            config = model.Config(**dict(model.config_dict(config), **grown, **clip))
        net = model.build(config, table, args.seed, features.buffers() if features is not None else None)
        # The weights the run trains on. An averaged checkpoint (--ema) keeps its unaveraged ones as `train_state`; its
        # `state`, the average, is the model the gate passed. A resumption continues the unaveraged weights, its average
        # where it was. Continuous training (--init-optimizer) continues them only when it keeps an average too
        # (--ema), which starts at the gated one; without --ema the run's weights are the model it writes, so it starts
        # from the gated model (the average) as a fine-tuned run does, the optimizer's moments carried onto it.
        weights = source["state"]
        if "train_state" in source and (resumed is not None or (args.init_optimizer and args.ema > 0)):
            weights = source["train_state"]
            ema_start = source["state"]
        # The checkpoint's rows are kept as they were; the table only gains the new cards' rows. A head the checkpoint
        # has none of starts fresh; a projection of the numbers widened by --log-nums computes what it did.
        table_tensor = torch.from_numpy(np.ascontiguousarray(table, np.float32))
        state = model.widen_nums(net, model.grow_state(net, dict(weights, table=table_tensor)))
        missing, unexpected = net.load_state_dict(state, strict=False)
        if unexpected or any(not k.startswith(("future.", "q.", "fate.")) for k in missing):
            raise SystemExit(f"{source_path}: state does not fit the network: missing {missing}, unexpected {unexpected}")
        if ema_start is not None:
            ema_start = model.widen_nums(net, model.grow_state(net, dict(ema_start, table=table_tensor)))
        log(f"{'resuming' if resumed is not None else 'fine-tuning'} step {source.get('step')}")
    else:
        config = model.Config(width=args.width, layers=args.layers, heads=args.heads, ffn=args.ffn, residual=args.residual,
                              card_features=args.card_features, **wanted)
        net = model.build(config, table, args.seed, features.buffers() if features is not None else None)
        if args.template_init == "zero" and features is not None:
            with torch.no_grad():
                net.templates.weight.zero_()
    bad = [k for k, v in net.state_dict().items() if v.is_floating_point() and not bool(torch.isfinite(v).all())]
    if bad:
        more = "..." if len(bad) > 5 else ""
        raise SystemExit(f"non-finite values in {', '.join(bad[:5])}{more}: refusing to train from this checkpoint")
    net.to(device)
    log(f"model {model.config_dict(config)}: {model.parameters(net)} parameters")
    groups = param_groups(net)
    named = dict(net.named_parameters())
    optimizer = torch.optim.AdamW([dict(params=[named[k] for k in groups[0]], weight_decay=args.weight_decay),
                                   dict(params=[named[k] for k in groups[1]], weight_decay=0.0)],
                                  lr=args.lr, betas=(0.9, 0.98))
    carried = 0
    if resumed is None and args.init_optimizer:
        if source is None:
            raise SystemExit("--init-optimizer needs --init")
        carried = carry_optimizer(optimizer, net, source, log)
    ema = None
    if args.ema > 0:
        # An average carried from the checkpoint continues its count (its warm-up done); a new one warms up.
        updates = int(source.get("ema_updates", source.get("step") or 0)) if ema_start is not None else 0
        ema = EMA(net, args.ema, ema_start, updates)
    if ema is not None:
        log(f"weights averaged at decay {args.ema}" + (", the average carried from the checkpoint" if ema_start is not None else ""))
    opts = LossOptions(value_mix_full=args.value_mix_full, value_mix_fast=args.value_mix_fast, value_loss=args.value_loss,
                       imitation_policy_weight=args.imitation_policy_weight, surprise_weight=args.surprise_weight,
                       anchor_kl=args.anchor_kl, policy_smoothing=args.policy_smoothing,
                       entropy_weight=args.entropy_weight, fate_weight=args.fate_weight, q_weight=args.q_weight,
                       distill_share=args.distill_share)
    if (args.anchor_kl > 0 or args.surprise_weight > 0) and args.anchor:
        opts.anchor = Frozen(args.anchor, index, known, data.header, device, log)
    elif args.anchor_kl > 0:
        raise SystemExit("--anchor-kl needs --anchor")
    if args.distill_from:
        opts.teacher = (opts.anchor if opts.anchor is not None and Path(args.distill_from) == Path(args.anchor)
                        else Frozen(args.distill_from, index, known, data.header, device, log))
    if resumed is not None and resumed.get("optimizer"):
        if resumed.get("param_names") in (None, groups):
            optimizer.load_state_dict(resumed["optimizer"])
            log("resumed the optimizer")
        else:
            # The parameters changed since the checkpoint (a head an option of this run adds): carried by name.
            carry_optimizer(optimizer, net, resumed, log, "--resume")
        bad = [k for state in optimizer.state.values() for k, v in state.items()
               if torch.is_tensor(v) and v.is_floating_point() and not bool(torch.isfinite(v).all())]
        if bad:
            raise SystemExit("non-finite values in the resumed optimizer state; refusing to train")

    per_part = max(1, len(dataset.batches(data, train, args.batch, np.random.default_rng(args.seed), trivial,
                                          args.search_target_repeat)))
    # Chunked, an epoch is estimated from the chunk loaded first: the chunks are alike, a share of the same games each.
    # A resumed run keeps the total its checkpoint was making (it loads a different chunk first, so its estimate differs,
    # and the schedule would jump); a checkpoint from before the total was recorded needs --steps.
    per_epoch = per_part * chunks
    # --target-uses: an epoch deals each search target --search-target-repeat times, so that many uses are this many epochs.
    epochs = args.target_uses / max(args.search_target_repeat, 1.0) if args.target_uses is not None else args.epochs
    total = args.steps if args.steps is not None else int((resumed or {}).get("total") or 0) or max(1, int(per_epoch * epochs))
    log(f"{per_epoch} batches an epoch{f' (about {per_part} a chunk)' if chunks > 1 else ''}, {total} steps"
        + (f" ({epochs:.4g} epochs: each search target used about {args.target_uses:g} times)" if args.target_uses is not None else ""))

    # Continuous training whose optimizer state was carried does not warm up again: its moments are the last run's,
    # already at its rate.
    # A resumed run warms up as the run it continues did.
    warmup = int(resumed.get("warmup", args.warmup)) if resumed is not None else (0 if carried else args.warmup)
    if carried:
        log("no warm-up: the optimizer state is carried")

    def lr_at(step):
        if step < warmup:
            return args.lr * (step + 1) / warmup
        if args.schedule == "constant":
            return args.lr
        return args.lr * 0.5 * (1 + math.cos(math.pi * min(1.0, (step - warmup) / max(1, total - warmup))))

    history = []
    if resumed is not None and (out / "metrics.json").exists():
        try:
            history = json.loads((out / "metrics.json").read_text()).get("history", [])
        except (OSError, json.JSONDecodeError):
            history = []

    def checkpoint(step, epoch, metrics, final=False):
        record = dict(step=step, seconds=time.time() - start, **metrics)
        if history and history[-1].get("step") == step:
            history.pop()
        history.append(record)
        weights = {k: v.detach().cpu() for k, v in net.state_dict().items()}
        state = dict(config=model.config_dict(config), state=ema.state_dict(net) if ema is not None else weights,
                     param_names=groups, header=data.header, step=step, total=total, warmup=warmup,
                     epoch=epoch, part=position[0], part_step=position[1], seconds=time.time() - start, metrics=record, optimizer=optimizer.state_dict(),
                     args=vars(args), data=dict(games=data.games, samples=len(data), train=len(train),
                                                                 held_out=len(held), evaluated=len(held_eval),
                                                                 directories=data.directories),
                     eval_data=dict(directories=eval_data.directories, games=eval_data.games, samples=len(eval_data),
                                    evaluated=len(fixed)) if eval_data is not None else None,
                     init=args.init, cards=index, embed=basis.state() if basis is not None else None,
                     card_features=features.state() if features is not None else None, card_table=table_source)
        if ema is not None:
            state["train_state"] = weights
            state["ema_updates"] = ema.updates
        temporary = out / "checkpoint.pt.tmp"
        torch.save(state, temporary)
        os.replace(temporary, out / "checkpoint.pt")
        metrics_tmp = out / "metrics.json.tmp"
        metrics_tmp.write_text(json.dumps(dict(final=final, history=history), indent=1))
        os.replace(metrics_tmp, out / "metrics.json")

    def scored(on, batches):
        """`evaluate`, of the model the run writes (the average, under --ema)."""
        with ema.applied(net) if ema is not None else contextlib.nullcontext():
            return evaluate(net, on, batches, args.workers, belief, device)

    step = int(resumed.get("step", 0)) if resumed is not None else 0
    seen = int(resumed.get("metrics", {}).get("seen", 0)) if resumed is not None else 0
    epoch = int(resumed.get("epoch", step // per_epoch)) if resumed is not None else 0
    if position is not None:
        skip = max(0, step - position[1])
    else:
        skip = max(0, step - epoch * per_epoch)
        while skip >= per_epoch:
            skip -= per_epoch
            epoch += 1
        position = [0, step - skip]
    window = dict(policy=0.0, value=0.0, top1=0, choices=0, n=0)
    tick = time.time()
    skipped = 0
    nonfinite_limit = max(10, total // 100)
    done = step >= total
    while not done:
        parts = order(epoch)
        while position[0] < len(parts) and not done:
            if parts[position[0]] != loaded:
                unload(data)
                del data, train, held, held_eval, eval_batches
                loaded = parts[position[0]]
                data, train, held, held_eval, eval_batches = load_chunk(loaded)
            key = args.seed + 1 + epoch if chunks == 1 else args.seed + 1 + epoch * chunks + position[0]
            for b in loader(data, dataset.batches(data, train, args.batch, np.random.default_rng(key), trivial,
                                                  args.search_target_repeat),
                            args.workers, device.type == "cuda"):
                if skip:
                    skip -= 1
                    continue
                b = to_device(b, device)
                for group in optimizer.param_groups:
                    group["lr"] = lr_at(step)
                with torch.autocast("cuda", dtype=torch.bfloat16, enabled=half):
                    loss, m = losses(net, b, belief, args.value_mix, args.future_weight, args.heuristic_weight, args.residual_l2,
                                     args.detach_heuristic, args.policy_weight, args.distill_mix, opts)
                loss = loss.float()
                why = None
                if torch.isfinite(loss):
                    optimizer.zero_grad(set_to_none=True)
                    loss.backward()
                    norm = torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0)
                    if not torch.isfinite(norm):
                        why = "gradient"
                else:
                    why = "loss"
                if why is not None:
                    skipped += 1
                    optimizer.zero_grad(set_to_none=True)
                    if skipped <= 5 or skipped % 100 == 0:
                        log(f"step {step + 1}/{total}: skipped a non-finite {why} ({skipped})")
                    if skipped > nonfinite_limit:
                        raise SystemExit(f"{skipped} non-finite batches; the data or the model is corrupt")
                else:
                    optimizer.step()
                    if ema is not None:
                        ema.update(net)
                    seen += m["n"]
                    window["policy"] += m["policy"] * m["choices"]
                    window["value"] += m["value"] * m["n"]
                    for k in ("top1", "choices", "n"):
                        window[k] += m[k]
                step += 1
                if why is None and (step % 10 == 0 or step == total):
                    bar.update(step, total, seen, m["policy"], m["top1"] / max(m["choices"], 1), start)
                if step % 200 == 0:
                    now = time.time()
                    log(f"step {step}/{total} epoch {epoch} lr {lr_at(step):.2e} train ce {window['policy'] / max(window['choices'], 1):.4f} "
                        f"top1 {window['top1'] / max(window['choices'], 1):.4f} value {window['value'] / max(window['n'], 1):.4f} "
                        f"{window['n'] / (now - tick):.0f} samples/s, {skipped} skipped"
                        + (f", peak {torch.cuda.max_memory_allocated(device) / 2**30:.1f} GiB" if device.type == "cuda" else ""))
                    window = dict(policy=0.0, value=0.0, top1=0, choices=0, n=0)
                    tick = now
                if step % args.eval_every == 0 and (len(held_eval) or len(fixed)):
                    record = dict(seen=seen)
                    if len(held_eval):
                        record["held_out"] = scored(data, eval_batches)
                        log(f"held out at step {step}: {json.dumps(record['held_out'])}")
                    if len(fixed):
                        record["fixed_eval"] = scored(eval_data, fixed_batches)
                        log(f"fixed eval at step {step}: {json.dumps(record['fixed_eval'])}")
                    checkpoint(step, epoch, record)
                elif args.checkpoint_every > 0 and step % args.checkpoint_every == 0:
                    checkpoint(step, epoch, dict(seen=seen))
                if step >= total or (args.hours and time.time() - start > args.hours * 3600):
                    done = True
                    break
            if not done:
                position = [position[0] + 1, step]
        if not done:
            epoch += 1
            position = [0, step]
    bar.close()
    metrics = scored(data, eval_batches) if len(held_eval) else {}
    log(f"final held out after {step} steps, {seen} samples, {(time.time() - start) / 3600:.2f} h, {skipped} skipped: {json.dumps(metrics)}")
    record = dict(seen=seen, held_out=metrics)
    if len(fixed):
        record["fixed_eval"] = scored(eval_data, fixed_batches)
        log(f"final fixed eval at step {step}: {json.dumps(record['fixed_eval'])}")
    checkpoint(step, epoch, record, final=True)
    # The held-out positions export.py takes its references from: positions with a number too large for f16 (an old
    # shard's overflow) are left out, since export's Rust decoder reads them as the infinity Python sanitises away.
    picks = held_eval[np.sort(np.random.default_rng(args.seed).permutation(len(held_eval))[: args.references])]
    if len(picks):
        raw = dataset.decode(data.blob, data.offset, picks, sanitize=False)
        keep = np.isfinite(raw["nums"]).all(axis=(1, 2))
        if not keep.all():
            log(f"references: skipped {int((~keep).sum())} positions with non-finite numbers")
            picks = picks[keep]
    # In the training index's rows, which are the model's.
    with (out / "reference.bin").open("wb") as f:
        f.write(len(picks).to_bytes(4, "little"))
        for r in picks:
            p = dataset.with_rows(data.blob[data.offset[r] : data.offset[r + 1]].tobytes(), data.luts[data.index_of[r]])
            if p[0] == 1 and dataset.LAYOUT["schema"] == 4:
                # A position of a schema-1 shard read as schema 4 (`dataset.UPGRADABLE`): the same bytes, relabelled,
                # as the Rust reader takes a position's schema byte as it is.
                p = bytes([4]) + p[1:]
            f.write(len(p).to_bytes(4, "little") + p)
    log_file.close()


if __name__ == "__main__":
    main()
