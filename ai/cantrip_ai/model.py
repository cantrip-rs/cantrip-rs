"""The network (docs/ai-player-plan.md, "Network"): a pre-norm transformer encoder over a position as
`cantrip_obs::encode` makes it, a value, two auxiliary targets, a pointer policy over the step's options and a
per-token belief.

A token's input is the sum of
- its kind's embedding;
- its card's row of the frozen card-text table (a buffer, never trained; `embed`), through a learned projection;
- a learned embedding for each of its 9 categoricals (one table, each slot's ids offset into it);
- its 96 numbers through a linear layer;
- what its 4 pointers point at: each pointed-to token's own input (the sum above), through a learned map per slot.

A pointer is looked up as a one-hot row of a constant identity (`Gather`) and a matrix product, so the graph needs no
`GatherElements`, `Equal` or `Shape`. Attention is written out, as in the spike, so every backend sees plain MatMul and
Softmax, and padding (kind 0) is masked out of it. There are no positions: the encoder's token order carries no
meaning the numbers and pointers do not.

Outputs, for a batch of B positions of T tokens:
- `value` [B]: the expected outcome for the seat, tanh, from the game token and the mean of the tokens;
- `policy` [B, T]: option logits, `MASKED` wherever the token is not a legal option;
- `aux` [B, 2]: the life swing over the turn (in life points) and the probability the seat is dead by the next turn;
- `belief` [B, T]: per token, the logit that the card is in an opponent's hand. Trained only on data whose mode hides
  the hands (in Full mode the hands are in the input), so a Full model's belief is untrained.
"""

from __future__ import annotations

import json
from dataclasses import asdict, dataclass
from pathlib import Path

import numpy as np
import torch
from torch import nn

AI = Path(__file__).resolve().parent.parent
LAYOUT = json.loads((AI / "encoding.json").read_text())
KINDS = len(LAYOUT["kinds"])
CATS = LAYOUT["cats"]
PTRS = LAYOUT["ptrs"]
NUMS = LAYOUT["nums"]
NO_POINTER = LAYOUT["no_pointer"]
VOCABS = [c["vocab"] for c in sorted(LAYOUT["categorical"], key=lambda c: c["slot"])]
MASKED = -1e9
INPUTS = ("kind", "card", "cats", "ptrs", "nums", "legal")
OUTPUTS = ("value", "policy", "aux", "belief")
SWING_SCALE = 10.0


@dataclass
class Config:
    width: int = 144
    layers: int = 4
    heads: int = 4
    ffn: int = 576
    # Training-only auxiliary outputs (targets.future): a head the search never reads and export leaves out. 0, none.
    future: int = 0
    # The value as a residual on a heuristic: win logit = heuristic head + residual head, value = tanh(logit / 2)
    # = 2 P(win) - 1. The heuristic head is trained towards a noted heuristic's log-odds (the tactical model's), the
    # sum towards the outcomes, the residual held towards zero (`train.py --residual`). False: the value head alone.
    residual: bool = False


def checkpoint_cards(checkpoint: dict, path: str | Path) -> list:
    """The card index a checkpoint's table rows are of: its `cards` (`names[i]` is row `i + 1`), or, for a checkpoint
    from before checkpoints kept it, the nearest `cards.json` above it (its model directory's) whose names are its
    header's `text_hash`."""
    if checkpoint.get("cards"):
        return list(checkpoint["cards"])
    from . import dataset  # noqa: PLC0415 (dataset reads the layout this module does)

    found = dataset.nearest_cards(Path(path))
    if found is not None:
        names = json.loads(found.read_text())["names"]
        if dataset.index_hash(names) == checkpoint.get("header", {}).get("text_hash"):
            return names
    raise SystemExit(f"{path} keeps no card index: put the cards.json it was trained with in a directory above it")


class Attention(nn.Module):
    def __init__(self, c: Config):
        super().__init__()
        self.c = c
        self.qkv = nn.Linear(c.width, 3 * c.width)
        self.out = nn.Linear(c.width, c.width)

    def forward(self, x, bias, tokens: int):
        w, h = self.c.width, self.c.heads
        head = w // h
        q, k, v = self.qkv(x).reshape(-1, tokens, 3, h, head).permute(2, 0, 3, 1, 4).unbind(0)
        scores = torch.matmul(q, k.transpose(-1, -2)) * (head**-0.5) + bias
        y = torch.matmul(torch.softmax(scores, dim=-1), v)
        return self.out(y.transpose(1, 2).reshape(-1, tokens, w))


class Block(nn.Module):
    def __init__(self, c: Config):
        super().__init__()
        self.norm1 = nn.LayerNorm(c.width)
        self.attention = Attention(c)
        self.norm2 = nn.LayerNorm(c.width)
        self.ffn = nn.Sequential(nn.Linear(c.width, c.ffn), nn.GELU(), nn.Linear(c.ffn, c.width))

    def forward(self, x, bias, tokens: int):
        x = x + self.attention(self.norm1(x), bias, tokens)
        return x + self.ffn(self.norm2(x))


class Net(nn.Module):
    def __init__(self, config: Config, table: np.ndarray):
        super().__init__()
        c = self.config = config
        w = c.width
        self.register_buffer("table", torch.from_numpy(np.ascontiguousarray(table, np.float32)))
        self.card = nn.Linear(table.shape[1], w)
        self.kind = nn.Embedding(KINDS, w)
        self.cats = nn.Embedding(sum(VOCABS), w)
        self.register_buffer("cat_offsets", torch.tensor(np.cumsum([0] + VOCABS[:-1]), dtype=torch.long), persistent=False)
        # Kind 0 is padding: everything else is a token.
        self.register_buffer("present_of", torch.tensor([0.0] + [1.0] * (KINDS - 1)), persistent=False)
        self.nums = nn.Linear(NUMS, w)
        self.pointers = nn.Linear(PTRS * w, w)
        self.blocks = nn.ModuleList(Block(c) for _ in range(c.layers))
        self.norm = nn.LayerNorm(w)
        self.value = nn.Sequential(nn.Linear(2 * w, w), nn.GELU(), nn.Linear(w, 1))
        self.aux = nn.Sequential(nn.Linear(2 * w, w), nn.GELU(), nn.Linear(w, 2))
        self.policy = nn.Sequential(nn.Linear(w, w), nn.GELU(), nn.Linear(w, 1))
        self.belief = nn.Linear(w, 1)
        self.future = nn.Sequential(nn.Linear(2 * w, w), nn.GELU(), nn.Linear(w, c.future)) if c.future else None
        self.heuristic = nn.Sequential(nn.Linear(2 * w, w), nn.GELU(), nn.Linear(w, 1)) if c.residual else None
        self.register_buffer("masked", torch.tensor(MASKED), persistent=False)
        # Set for export: a constant token count keeps Shape ops out of the graph; the batch stays free.
        self.tokens = 0
        # Set for export by `fold`: the card-text table already through the card projection.
        self.folded = None

    def forward(self, kind, card, cats, ptrs, nums, legal):
        value, policy, swing, dead, belief = self.heads(kind, card, cats, ptrs, nums, legal)
        # sigmoid(d) as 0.5 * (1 + tanh(d / 2)): Sigmoid is not on the operator allow-list.
        aux = torch.stack([swing * SWING_SCALE, 0.5 * (1.0 + torch.tanh(0.5 * dead))], dim=-1)
        return value, policy, aux, belief

    def heads(self, kind, card, cats, ptrs, nums, legal):
        """kind [B,T] i64, card [B,T] i64 (card index rows), cats [B,T,9] i64, ptrs [B,T,4] i64 (255 = none), nums
        [B,T,96] f32, legal [B,T] bool. Returns value [B], policy [B,T], the life swing / SWING_SCALE [B], the dead-next-turn logit [B], belief [B,T]."""
        x, summary = self.body(kind, card, cats, ptrs, nums)
        value, _ = self.value_parts(summary)
        swing, dead = self.aux(summary).unbind(-1)
        logits = self.policy(x).squeeze(-1)
        belief = self.belief(x).squeeze(-1)
        return value, torch.where(legal, logits, self.masked), swing, dead, belief

    def value_parts(self, summary):
        """The value [B], and with `Config.residual` the heuristic head's and the residual head's logits [B] each."""
        if self.heuristic is None:
            return torch.tanh(self.value(summary)).squeeze(-1), None
        h = self.heuristic(summary).squeeze(-1)
        r = self.value(summary).squeeze(-1)
        return torch.tanh(0.5 * (h + r)), (h, r)

    def heads_and_future(self, kind, card, cats, ptrs, nums, legal):
        """`heads`, and the training-only future head's outputs [B, future] (None without one)."""
        x, summary = self.body(kind, card, cats, ptrs, nums)
        value, parts = self.value_parts(summary)
        swing, dead = self.aux(summary).unbind(-1)
        logits = self.policy(x).squeeze(-1)
        belief = self.belief(x).squeeze(-1)
        future = self.future(summary) if self.future is not None else None
        return value, torch.where(legal, logits, self.masked), swing, dead, belief, future, parts

    def body(self, kind, card, cats, ptrs, nums):
        """The encoder: each token's output [B,T,W], and the summary the value and auxiliary heads read [B,2W] (the game
        token's output beside the mean of the tokens')."""
        tokens = self.tokens or kind.shape[1]
        w = self.config.width
        card_x = self.folded[card] if self.folded is not None else self.card(self.table[card])
        x = (self.kind(kind) + card_x + self.cats(cats + self.cat_offsets).sum(2)
             + self.nums(nums))
        # A pointer as a one-hot row over the tokens: rows past the last token (255 among them) are zero.
        eye = torch.eye(NO_POINTER + 1, tokens, dtype=x.dtype, device=x.device)
        onehot = eye[ptrs].reshape(-1, tokens * PTRS, tokens)
        pointed = torch.matmul(onehot, x).reshape(-1, tokens, PTRS * w)
        x = x + self.pointers(pointed)

        keep = self.present_of[kind]
        bias = ((1.0 - keep) * MASKED).reshape(-1, 1, 1, tokens)
        for block in self.blocks:
            x = block(x, bias, tokens)
        x = self.norm(x)
        pooled = (x * keep.unsqueeze(-1)).sum(1) / keep.sum(1, keepdim=True).clamp(min=1.0)
        return x, torch.cat([x[:, 0], pooled], dim=-1)


@torch.no_grad()
def fold(net: Net):
    """The card projection applied to the whole table once (the same function): a narrower table in the graph,
    where the network is narrower than the table."""
    net.folded = net.card(net.table).detach()


def build(config: Config, table: np.ndarray, seed: int = 0) -> Net:
    torch.manual_seed(seed)
    return Net(config, table)


def parameters(net: nn.Module) -> int:
    return sum(p.numel() for p in net.parameters())


def config_dict(config: Config) -> dict:
    return asdict(config)
