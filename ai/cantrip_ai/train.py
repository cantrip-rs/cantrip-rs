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

The card-text table (`embed`) is made here, never checked in. A fresh network's is fitted to the names its shards
name; a fine-tuned one keeps its checkpoint's rows and basis, and each card new to it is projected into that basis.
The checkpoint keeps the names (`cards`) and the basis (`embed`), and export writes the names as the model's
`cards.json`.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import sys
import time
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
        inputs = dataset.decode(data.blob, data.offset, rows)
        inputs["card"] = dataset.cards(data, rows, inputs["card"])
        target = dataset.labels(data, rows, inputs)
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


def losses(net, b, belief: bool, value_mix: float = 0.0, future_weight: float = 0.0, heuristic_weight: float = 0.0,
           residual_l2: float = 0.0, detach_heuristic: bool = False, policy_weight: float = 1.0, distill_mix: float = 0.0):
    value, policy, swing, dead, belief_logits, future, parts = net.heads_and_future(b["kind"], b["card"], b["cats"], b["ptrs"], b["nums"], b["legal"])
    choice = b["choice"]
    visits = b["visits"]
    logp = F.log_softmax(policy, -1)
    ce = -torch.where(visits > 0, visits * logp, torch.zeros_like(logp)).sum(-1)
    policy_loss = (ce * choice).sum() / choice.sum().clamp(min=1)
    root = b["root"]
    target_value = b["outcome"]
    if "td" in b:
        # The lambda-return over the seat's later searched root values (targets.td), instead of the mix.
        target_value = b["td"]
    elif distill_mix > 0 and "distill" in b:
        # The perfect-information value's mean over the seat's worlds, mixed with the outcome where one was distilled.
        d = b["distill"]
        target_value = torch.where(torch.isfinite(d), (1 - distill_mix) * target_value + distill_mix * torch.nan_to_num(d), target_value)
    elif value_mix > 0:
        safe_root = torch.nan_to_num(root, nan=0.0, posinf=0.0, neginf=0.0)
        target_value = torch.where(torch.isfinite(root), (1 - value_mix) * target_value + value_mix * safe_root, target_value)
    if parts is not None and detach_heuristic:
        # The outcomes train the residual only: the heuristic head is the tactical model's, distilled, and the value
        # loss sees it as given (the same value at play; only where the outcome's gradient goes differs).
        value = torch.tanh(0.5 * (parts[0].detach() + parts[1]))
    value_loss = F.mse_loss(value, target_value)
    swing_loss = F.smooth_l1_loss(swing, b["swing"] / model.SWING_SCALE)
    dead_loss = F.binary_cross_entropy_with_logits(dead, b["dead"])
    total = policy_weight * policy_loss + value_loss + AUX_WEIGHT * (swing_loss + dead_loss)
    heuristic_loss = torch.zeros(())
    residual_loss = torch.zeros(())
    if parts is not None:
        h, r = parts
        noted = b["heuristic"]
        known = torch.isfinite(noted)
        if heuristic_weight > 0 and bool(known.any()):
            # The noted log-odds, in units of 4 so a typical swing is about one; clipped to the range they fall in.
            want = torch.nan_to_num(noted, nan=0.0).clamp(-12.0, 12.0)
            heuristic_loss = (((h - want) / 4.0) ** 2 * known).sum() / known.sum()
            total = total + heuristic_weight * heuristic_loss
        if residual_l2 > 0:
            residual_loss = (r ** 2).mean()
            total = total + residual_l2 * residual_loss
    future_loss = torch.zeros(())
    if future is not None and "future" in b and future_weight > 0:
        future_loss = F.smooth_l1_loss(future, b["future"])
        total = total + future_weight * future_loss
    belief_loss = torch.zeros(())
    if belief:
        pool = b["pool"].float()
        bce = F.binary_cross_entropy_with_logits(belief_logits, b["in_hand"], reduction="none")
        belief_loss = (bce * pool).sum() / pool.sum().clamp(min=1)
        total = total + BELIEF_WEIGHT * belief_loss
    top1 = ((policy.argmax(-1) == b["target"]) & choice).sum()
    top1_visits = ((policy.argmax(-1) == visits.argmax(-1)) & choice).sum()
    return total, dict(policy=policy_loss.item(), value=value_loss.item(), swing=swing_loss.item(), dead=dead_loss.item(),
                       belief=belief_loss.item(), future=future_loss.item(), heuristic=heuristic_loss.item(),
                       residual=residual_loss.item(), top1=top1.item(), top1_visits=top1_visits.item(), choices=choice.sum().item(),
                       n=len(choice))


@torch.no_grad()
def evaluate(net, data, batches, workers, belief: bool, device: torch.device) -> dict:
    net.eval()
    sums = dict(policy=0.0, value=0.0, swing=0.0, dead=0.0, belief=0.0, top1=0, top1_visits=0, choices=0, n=0, sign=0, first=0,
                heuristic=0.0, residual=0.0, head_mse=0.0, noted_mse=0.0, noted=0)
    for b in loader(data, batches, workers, device.type == "cuda"):
        b = to_device(b, device)
        # The value held out is against the outcome, whatever target it was trained on.
        b.pop("td", None)
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
    net.train()
    n, c = max(sums["n"], 1), max(sums["choices"], 1)
    return dict(samples=sums["n"], choices=sums["choices"], top1=sums["top1"] / c, top1_visits=sums["top1_visits"] / c, first_legal=sums["first"] / c, policy_ce=sums["policy"] / c,
                value_mse=sums["value"] / n, value_sign=sums["sign"] / n, swing=sums["swing"] / n, dead=sums["dead"] / n,
                belief=sums["belief"] / n if belief else None,
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


def main(argv=None):
    a = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    a.add_argument("--data", nargs="+", required=True, help="shard directories, DIR or DIR:WEIGHT")
    a.add_argument("--init", help="a checkpoint.pt to fine-tune (its config and weights)")
    a.add_argument("--resume", action="store_true", help="continue from --out/checkpoint.pt, if it is there")
    a.add_argument("--value-mix", type=float, default=0.0, help="the share of the root value in the value's target")
    a.add_argument("--td-lambda", type=float, help="the value's target: the lambda-return over the seat's later root values (targets.td)")
    a.add_argument("--future-weight", type=float, default=0.0, help="train the training-only future head (targets.future) at this weight")
    a.add_argument("--residual", action="store_true", help="a fresh network's value as a residual on a heuristic head (model.Config.residual)")
    a.add_argument("--heuristic-weight", type=float, default=1.0, help="the heuristic head's loss weight, towards the noted heuristic")
    a.add_argument("--residual-l2", type=float, default=0.1, help="the residual's pull towards zero")
    a.add_argument("--distill-mix", type=float, default=0.0,
                   help="the share of the value target that is the distilled perfect-information value (distill_values)")
    a.add_argument("--policy-weight", type=float, default=1.0,
                   help="the policy loss's weight: 0 trains a value alone (a perfect-information value on Full positions)")
    a.add_argument("--detach-heuristic", action="store_true",
                   help="the outcomes train the residual only, never the heuristic head (which the noted heuristic trains)")
    a.add_argument("--out", required=True)
    a.add_argument("--epochs", type=float, default=1.0)
    a.add_argument("--steps", type=int, help="stop after this many steps (the cosine spans them)")
    a.add_argument("--hours", type=float, help="a wall-clock limit")
    a.add_argument("--max-games", type=int, help="at most this many games from each directory (smoke tests)")
    a.add_argument("--decks", help="only the games played among these decks: a decks directory (its .txt names) or "
                                   "names separated by commas")
    a.add_argument("--chunks", type=int, default=1,
                   help="train on a set larger than memory: its shards in this many chunks, one loaded at a time, an epoch "
                        "each chunk once in a shuffled order; single-option positions are thinned at load (--trivial)")
    a.add_argument("--batch", type=int, default=256)
    a.add_argument("--lr", type=float, default=5e-4)
    a.add_argument("--weight-decay", type=float, default=0.01)
    a.add_argument("--warmup", type=int, default=500)
    a.add_argument("--held-out", type=float, default=0.02, help="the fraction of games held out")
    a.add_argument("--eval-samples", type=int, default=20000)
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
    chunks = max(1, args.chunks)
    # Chunked, the single-option positions are thinned as they load, and every one loaded is trained on.
    trivial = args.trivial if chunks == 1 else 1.0

    def load_chunk(part: int):
        log(f"loading {args.data}" + (f" chunk {part + 1} of {chunks}" if chunks > 1 else ""))
        data = dataset.load(args.data, args.max_games, log=lambda *_: None, seed=args.seed, base=base,
                            chunk=(part, chunks) if chunks > 1 else None, trivial=args.trivial if chunks > 1 else 1.0,
                            decks=deck_names(args.decks))
        log(dataset.describe(data), f"({time.time() - start:.0f} s)")
        if args.td_lambda is not None:
            from . import targets
            data.td = targets.td(data, args.td_lambda)
            log(f"value target: the lambda-return, lambda {args.td_lambda} ({time.time() - start:.0f} s)")
        if args.future_weight > 0:
            from . import targets
            data.future = targets.future(data, log=log)
            log(f"future head: weight {args.future_weight} ({time.time() - start:.0f} s)")
        if args.workers and torch.multiprocessing.get_start_method() != "fork":
            # Spawned workers would each be handed a copy of the data; shared, they read the one.
            data = dataset.share(data)
            log(f"the data in shared memory for {args.workers} spawned workers ({time.time() - start:.0f} s)")
        for d in data.directories:
            dropped = f", {d['nonfinite_roots']} non-finite root values ignored" if d["nonfinite_roots"] else ""
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
    if source is not None:
        kept = source["state"]["table"].numpy()
        if kept.shape[0] != len(base) + 1:
            raise SystemExit(f"{source_path}: its table has {kept.shape[0]} rows for {len(base)} names")
        basis = (embed.Basis.from_state(source["embed"]) if source.get("embed")
                 else embed.legacy_basis(base, kept, log=log, known=known))
        table = embed.extend(kept, data.names, basis, known)
    else:
        table, basis = embed.rows(data.names, None, known)
    new = len(data.names) - len(base)
    log(f"card index: {len(data.names)} names, {sum(n in known for n in data.names)} with text, {new} new to "
        f"{'the checkpoint' if source is not None else 'this network'}")
    belief = data.header["mode"] != "full"
    log(f"mode {data.header['mode']}: the belief loss is {'on' if belief else 'off (the hands are in the input)'}")

    if source is not None:
        config = model.Config(**source["config"])
        if args.future_weight > 0 and not config.future:
            from . import targets
            config = model.Config(**dict(model.config_dict(config), future=targets.FUTURE))
        net = model.build(config, table, args.seed)
        # The checkpoint's rows are kept as they were; the table only gains the new cards' rows. A future head the
        # checkpoint has none of starts fresh.
        missing, unexpected = net.load_state_dict(
            dict(source["state"], table=torch.from_numpy(np.ascontiguousarray(table, np.float32))), strict=False)
        if unexpected or any(not k.startswith("future.") for k in missing):
            raise SystemExit(f"{source_path}: state does not fit the network: missing {missing}, unexpected {unexpected}")
        log(f"{'resuming' if resumed is not None else 'fine-tuning'} step {source.get('step')}")
    else:
        config = model.Config(width=args.width, layers=args.layers, heads=args.heads, ffn=args.ffn, residual=args.residual)
        net = model.build(config, table, args.seed)
    bad = [k for k, v in net.state_dict().items() if v.is_floating_point() and not bool(torch.isfinite(v).all())]
    if bad:
        more = "..." if len(bad) > 5 else ""
        raise SystemExit(f"non-finite values in {', '.join(bad[:5])}{more}: refusing to train from this checkpoint")
    net.to(device)
    log(f"model {model.config_dict(config)}: {model.parameters(net)} parameters")
    decay = [p for n, p in net.named_parameters() if p.dim() >= 2 and "kind" not in n and "cats" not in n]
    rest = [p for n, p in net.named_parameters() if not (p.dim() >= 2 and "kind" not in n and "cats" not in n)]
    optimizer = torch.optim.AdamW([dict(params=decay, weight_decay=args.weight_decay), dict(params=rest, weight_decay=0.0)],
                                  lr=args.lr, betas=(0.9, 0.98))
    if resumed is not None and resumed.get("optimizer"):
        optimizer.load_state_dict(resumed["optimizer"])
        bad = [k for state in optimizer.state.values() for k, v in state.items()
               if torch.is_tensor(v) and v.is_floating_point() and not bool(torch.isfinite(v).all())]
        if bad:
            raise SystemExit("non-finite values in the resumed optimizer state; refusing to train")
        log("resumed the optimizer")

    per_part = max(1, len(dataset.batches(data, train, args.batch, np.random.default_rng(args.seed), trivial)))
    # Chunked, an epoch is estimated from the chunk loaded first: the chunks are alike, a share of the same games each.
    per_epoch = per_part * chunks
    total = args.steps or max(1, int(per_epoch * args.epochs))
    log(f"{per_epoch} batches an epoch{f' (about {per_part} a chunk)' if chunks > 1 else ''}, {total} steps")

    def lr_at(step):
        if step < args.warmup:
            return args.lr * (step + 1) / args.warmup
        return args.lr * 0.5 * (1 + math.cos(math.pi * min(1.0, (step - args.warmup) / max(1, total - args.warmup))))

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
        state = dict(config=model.config_dict(config), state={k: v.detach().cpu() for k, v in net.state_dict().items()}, header=data.header, step=step,
                     epoch=epoch, part=position[0], part_step=position[1], seconds=time.time() - start, metrics=record, optimizer=optimizer.state_dict(),
                     args=vars(args), data=dict(games=data.games, samples=len(data), train=len(train),
                                                                 held_out=len(held), evaluated=len(held_eval),
                                                                 directories=data.directories),
                     init=args.init, cards=data.names, embed=basis.state() if basis is not None else None)
        temporary = out / "checkpoint.pt.tmp"
        torch.save(state, temporary)
        os.replace(temporary, out / "checkpoint.pt")
        metrics_tmp = out / "metrics.json.tmp"
        metrics_tmp.write_text(json.dumps(dict(final=final, history=history), indent=1))
        os.replace(metrics_tmp, out / "metrics.json")

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
            for b in loader(data, dataset.batches(data, train, args.batch, np.random.default_rng(key), trivial),
                            args.workers, device.type == "cuda"):
                if skip:
                    skip -= 1
                    continue
                b = to_device(b, device)
                for group in optimizer.param_groups:
                    group["lr"] = lr_at(step)
                loss, m = losses(net, b, belief, args.value_mix, args.future_weight, args.heuristic_weight, args.residual_l2,
                             args.detach_heuristic, args.policy_weight, args.distill_mix)
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
                        f"{window['n'] / (now - tick):.0f} samples/s, {skipped} skipped")
                    window = dict(policy=0.0, value=0.0, top1=0, choices=0, n=0)
                    tick = now
                if step % args.eval_every == 0 and len(held_eval):
                    metrics = evaluate(net, data, eval_batches, args.workers, belief, device)
                    log(f"held out at step {step}: {json.dumps(metrics)}")
                    checkpoint(step, epoch, dict(seen=seen, held_out=metrics))
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
    metrics = evaluate(net, data, eval_batches, args.workers, belief, device) if len(held_eval) else {}
    log(f"final held out after {step} steps, {seen} samples, {(time.time() - start) / 3600:.2f} h, {skipped} skipped: {json.dumps(metrics)}")
    checkpoint(step, epoch, dict(seen=seen, held_out=metrics), final=True)
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
            f.write(len(p).to_bytes(4, "little") + p)


if __name__ == "__main__":
    main()
