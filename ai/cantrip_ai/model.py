"""The network (docs/ai-player-plan.md, "Network"): a pre-norm transformer encoder over a position as
`cantrip_obs::encode` makes it, a value, two auxiliary targets, a pointer policy over the step's options and a
per-token belief.

A token's input is the sum of
- its kind's embedding;
- its card's row of the frozen card-text table (a buffer, never trained; `embed`), through a learned projection, and
  with `Config.card_features = "templates"` the card's template items as well (`card_features`, `Net.card_table`);
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

Training-only outputs, which the graphs leave out (`train_heads`): the future head (`Config.future`), a per-option Q
head (`Config.q_head`) and per-token fates (`Config.fates`).
"""

from __future__ import annotations

import contextlib
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
    # What a card's vector is made from: "text", the card-text table's row through the card projection; "templates",
    # that plus the card's template items (`card_features`), each an embedding of its key and its values through a
    # small map, summed per card. Export folds either into one row per card.
    card_features: str = "text"
    # The numbers as the network reads them (review 2026-11-13, item 12): False, as they come (scaled linearly by the
    # encoding, so a count is unbounded); True, each number clipped to +/- `nums_clip` beside sign(x) log(1 + |x|),
    # both through the numbers' projection (twice as wide). The observation still carries the exact number: only the
    # network's reading of it is compressed. Inside the graph's input stage, which every server keeps in fp32.
    # `nums_clip` 0 leaves the linear copy unclipped: what a checkpoint widened to `nums_log` reads (`widen_nums`), so
    # it computes what it did for every number, however large.
    nums_log: bool = False
    nums_clip: float = 8.0
    # Training-only per-option value head (`train.py --q-weight`): for each token, the search's root value of that
    # option, tanh, trained where the data records the root's per-option values. Export leaves it out of the graphs.
    q_head: bool = False
    # Training-only per-token fates (`targets.fates`, `train.py --fate-weight`): this many logits a token. 0, none.
    fates: int = 0
    # A network upgraded from encoding schema 1 (`upgrade_schema1`): the inputs schema 4 put in places schema 1 left
    # empty (SCHEMA1_NEW_NUMS, SCHEMA1_NEW_PTRS) are read by projections of their own, which start at zero, and kept
    # out of the old ones, whose shared columns carry other kinds' features. It computes what it did at schema 1 until
    # training moves the new projections.
    schema1_inputs: bool = False


# Encoding schema 1 (ai/encoding.json before main's schema 4, 2026-11), what the fp line's networks gen0-gen7 and their
# shards were made in: the categorical vocabularies by slot, and the encoder hash its shards and models carry. Schema 4
# only added to it: a category at the end of `ask`, `event` and `tok` (ChooseCostPayment, Untapped, CostDigit), numbers
# in columns that a token kind left empty (snow, cost payments, block capacity), the option's pointers 2 and 3 (cost
# objects), and keywords past the first 48 bits (the encoding reads the first 48, whose meanings are unchanged). So a
# schema-1 position reads as a schema-4 one with nothing new in it (`dataset.UPGRADABLE`).
SCHEMA1_HASH = "6fed5023a1c55ffc"
SCHEMA1_VOCABS = [10, 9, 9, 13, 27, 66, 10, 29, 13]
# The rows schema 4 appends to each categorical slot, each started as the slot's row it was read as before: a new ask
# kind and a new token as none (0), the new event kind as the unknown event (65).
SCHEMA1_NEW_CATS = {4: [0], 5: [65], 7: [0]}
# The numbers schema 4 puts where a kind had none (by kind name: columns), and the option's new pointers.
SCHEMA1_NEW_NUMS = {"player": range(25, 31), "game": range(31, 33), "option": range(29, 81)}
SCHEMA1_NEW_PTRS = {"option": (2, 3)}


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


def _tensor(x, dtype):
    return x.to(dtype) if torch.is_tensor(x) else torch.from_numpy(np.ascontiguousarray(x)).to(dtype)


class Net(nn.Module):
    def __init__(self, config: Config, table: np.ndarray, features: dict | None = None):
        """`features`, for `card_features = "templates"`: `card_features.Features.buffers()` (ids, rows, values [items,
        slots, FEATS], and the vocabulary's size)."""
        super().__init__()
        c = self.config = config
        w = c.width
        self.register_buffer("table", torch.from_numpy(np.ascontiguousarray(table, np.float32)))
        self.card = nn.Linear(table.shape[1], w)
        if c.card_features == "templates":
            if features is None:
                raise ValueError("a templates network needs its card features")
            self.register_buffer("feat_ids", _tensor(features["ids"], torch.long))
            self.register_buffer("feat_rows", _tensor(features["rows"], torch.long))
            self.register_buffer("feat_values", _tensor(features["values"], torch.float32))
            slots, feats = self.feat_values.shape[1], self.feat_values.shape[2]
            self.templates = nn.Embedding(int(features["vocab"]), w)
            self.template_item = nn.Sequential(nn.Linear(w + slots * feats, w), nn.GELU(), nn.Linear(w, w))
        elif c.card_features != "text":
            raise ValueError(f"unknown card_features {c.card_features!r}")
        self.kind = nn.Embedding(KINDS, w)
        self.cats = nn.Embedding(sum(VOCABS), w)
        self.register_buffer("cat_offsets", torch.tensor(np.cumsum([0] + VOCABS[:-1]), dtype=torch.long), persistent=False)
        # Kind 0 is padding: everything else is a token.
        self.register_buffer("present_of", torch.tensor([0.0] + [1.0] * (KINDS - 1)), persistent=False)
        self.nums = nn.Linear(2 * NUMS if c.nums_log else NUMS, w)
        self.pointers = nn.Linear(PTRS * w, w)
        if c.schema1_inputs:
            kinds = LAYOUT["kinds"]
            new_nums = torch.zeros(KINDS, NUMS)
            for name, cols in SCHEMA1_NEW_NUMS.items():
                new_nums[kinds.index(name), list(cols)] = 1.0
            new_ptrs = torch.zeros(KINDS, PTRS, dtype=torch.long)
            for name, slots in SCHEMA1_NEW_PTRS.items():
                new_ptrs[kinds.index(name), list(slots)] = 1
            self.register_buffer("new_nums", new_nums, persistent=False)
            self.register_buffer("new_ptrs", new_ptrs, persistent=False)
            self.nums_new = nn.Linear(2 * NUMS if c.nums_log else NUMS, w, bias=False)
            self.pointers_new = nn.Linear(PTRS * w, w, bias=False)
            nn.init.zeros_(self.nums_new.weight)
            nn.init.zeros_(self.pointers_new.weight)
        self.blocks = nn.ModuleList(Block(c) for _ in range(c.layers))
        self.norm = nn.LayerNorm(w)
        self.value = nn.Sequential(nn.Linear(2 * w, w), nn.GELU(), nn.Linear(w, 1))
        self.aux = nn.Sequential(nn.Linear(2 * w, w), nn.GELU(), nn.Linear(w, 2))
        self.policy = nn.Sequential(nn.Linear(w, w), nn.GELU(), nn.Linear(w, 1))
        self.belief = nn.Linear(w, 1)
        self.future = nn.Sequential(nn.Linear(2 * w, w), nn.GELU(), nn.Linear(w, c.future)) if c.future else None
        self.heuristic = nn.Sequential(nn.Linear(2 * w, w), nn.GELU(), nn.Linear(w, 1)) if c.residual else None
        self.q = nn.Sequential(nn.Linear(w, w), nn.GELU(), nn.Linear(w, 1)) if c.q_head else None
        self.fate = nn.Linear(w, c.fates) if c.fates else None
        self.register_buffer("masked", torch.tensor(MASKED), persistent=False)
        # Set for export: a constant token count keeps Shape ops out of the graph; the batch stays free.
        self.tokens = 0
        # Set for export by `fold`: the card-text table already through the card projection.
        self.folded = None
        # Set by the inference server: under autocast, the input stage (everything before the first block: the card
        # table, the embeddings, the numbers' and the pointers' projections, which read the raw features) runs in fp32.
        # The numbers are unbounded (a count is scaled linearly), and in fp16 their projections can overflow (gen2's
        # did, on 131,700 counters). Training leaves it off: it runs bf16, whose range is fp32's, so nothing it learns
        # changes.
        self.inputs_in_fp32 = False

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

    def value_logit(self, summary, parts=None):
        """The value's win logit [B]: value = tanh(logit / 2) = 2 P(win) - 1, so a binary cross-entropy can be taken on
        it (`train.py --value-loss bce`). With `Config.residual`, the heuristic head's logit plus the residual's."""
        if self.heuristic is None:
            return 2.0 * self.value(summary).squeeze(-1)
        h, r = parts if parts is not None else self.value_parts(summary)[1]
        return h + r

    def train_heads(self, kind, card, cats, ptrs, nums, legal) -> dict:
        """Every output training reads: `heads`' (value, policy, swing, dead, belief), the residual's `parts`, the value's
        win `logit`, and the training-only heads' (`future` [B, future], `q` [B, T], `fates` [B, T, fates]; None
        without the head)."""
        x, summary = self.body(kind, card, cats, ptrs, nums)
        value, parts = self.value_parts(summary)
        swing, dead = self.aux(summary).unbind(-1)
        logits = self.policy(x).squeeze(-1)
        return dict(value=value, policy=torch.where(legal, logits, self.masked), swing=swing, dead=dead,
                    belief=self.belief(x).squeeze(-1), parts=parts, logit=self.value_logit(summary, parts),
                    future=self.future(summary) if self.future is not None else None,
                    q=torch.tanh(self.q(x).squeeze(-1)) if self.q is not None else None,
                    fates=self.fate(x) if self.fate is not None else None)

    def heads_and_future(self, kind, card, cats, ptrs, nums, legal):
        """`heads`, and the training-only future head's outputs [B, future] (None without one)."""
        o = self.train_heads(kind, card, cats, ptrs, nums, legal)
        return o["value"], o["policy"], o["swing"], o["dead"], o["belief"], o["future"], o["parts"]

    def read_nums(self, nums):
        """The numbers as the numbers' projection reads them (`Config.nums_log`). The logarithm is taken of each sign's
        part apart, log(1 + max(x, 0)) - log(1 - min(x, 0)), which is sign(x) log(1 + |x|) in the graph's own
        operators (Clip, Add, Sub, Log: no Abs, Sign or Neg)."""
        if not self.config.nums_log:
            return nums
        c = float(self.config.nums_clip)
        log = torch.log(1.0 + torch.clamp(nums, min=0.0)) - torch.log(1.0 - torch.clamp(nums, max=0.0))
        return torch.cat([torch.clamp(nums, -c, c) if c > 0 else nums, log], dim=-1)

    def body(self, kind, card, cats, ptrs, nums):
        """The encoder: each token's output [B,T,W], and the summary the value and auxiliary heads read [B,2W] (the game
        token's output beside the mean of the tokens')."""
        tokens = self.tokens or kind.shape[1]
        w = self.config.width
        fp32 = torch.autocast(kind.device.type, enabled=False) if self.inputs_in_fp32 else contextlib.nullcontext()
        with fp32:
            if self.folded is not None:
                card_x = self.folded[card]
            elif self.config.card_features == "text":
                card_x = self.card(self.table[card])
            else:
                card_x = self.card_table()[card]
            if self.config.schema1_inputs:
                # Schema 4's new inputs apart (`Config.schema1_inputs`): out of the old projection, into their own.
                new = self.new_nums[kind]
                numbers = self.nums(self.read_nums(nums * (1.0 - new))) + self.nums_new(self.read_nums(nums * new))
            else:
                numbers = self.nums(self.read_nums(nums))
            x = self.kind(kind) + card_x + self.cats(cats + self.cat_offsets).sum(2) + numbers
            # A pointer as a one-hot row over the tokens: rows past the last token (255 among them) are zero.
            eye = torch.eye(NO_POINTER + 1, tokens, dtype=x.dtype, device=x.device)
            if self.config.schema1_inputs:
                new = self.new_ptrs[kind]
                old_ptrs = ptrs * (1 - new) + NO_POINTER * new
                new_ptrs = ptrs * new + NO_POINTER * (1 - new)
                pointed = torch.matmul(eye[old_ptrs].reshape(-1, tokens * PTRS, tokens), x).reshape(-1, tokens, PTRS * w)
                pointed_new = torch.matmul(eye[new_ptrs].reshape(-1, tokens * PTRS, tokens), x).reshape(-1, tokens, PTRS * w)
                x = x + self.pointers(pointed) + self.pointers_new(pointed_new)
            else:
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


    def card_table(self):
        """Every card row's vector [rows, W]: the text row through the card projection, plus (templates) the sum of
        the row's items, each its key's embedding beside its values through `template_item`."""
        x = self.card(self.table)
        if self.config.card_features == "templates" and self.feat_ids.numel():
            item = torch.cat([self.templates(self.feat_ids), self.feat_values.flatten(1)], dim=-1)
            x = x.index_add(0, self.feat_rows, self.template_item(item))
        return x


@torch.no_grad()
def fold(net: Net):
    """The card vectors computed for the whole table once (the same function): one row per card in the graph, where the
    network is narrower than the text table, and always for templates (whose items the graph then never sees)."""
    net.folded = net.card_table().detach()


def must_fold(net: Net) -> bool:
    """Whether export folds the card table into the graph."""
    return net.config.card_features != "text" or net.config.width < net.table.shape[1]


def state_features(state: dict) -> dict | None:
    """The template features a state dict's buffers hold (None for a text network)."""
    if "feat_ids" not in state:
        return None
    return dict(ids=state["feat_ids"], rows=state["feat_rows"], values=state["feat_values"],
                vocab=state["templates.weight"].shape[0])


def from_state(config: Config, state: dict, table=None) -> Net:
    """A network shaped for `state` (its table, and a templates network's features), weights not yet loaded."""
    table = state["table"] if table is None else table
    return Net(config, table.numpy() if torch.is_tensor(table) else table, state_features(state))


def grow_state(net: Net, state: dict) -> dict:
    """A checkpoint's `state` fitted to `net`, a templates network made for an index that grew from it
    (`card_features.extend`): `net`'s own item buffers; the old keys' embeddings kept and each new key's at zero; and
    if the items gained value slots, the old slots' weights kept and the new ones' at zero. A text network's state is
    returned as it is."""
    if net.config.card_features != "templates":
        return state
    state = dict(state, feat_ids=net.feat_ids, feat_rows=net.feat_rows, feat_values=net.feat_values)
    old = state["templates.weight"]
    grown = torch.zeros_like(net.templates.weight)
    grown[: old.shape[0]] = old
    state["templates.weight"] = grown
    first = state["template_item.0.weight"]
    if first.shape != net.template_item[0].weight.shape:
        wide = torch.zeros_like(net.template_item[0].weight)
        wide[:, : first.shape[1]] = first
        state["template_item.0.weight"] = wide
    return state


def widen_nums(net: Net, state: dict) -> dict:
    """A state from a network that read the numbers as they come, fitted to `net`, which reads them through
    `Config.nums_log`: the old projection's weights on the linear copy and zeros on the logarithms, so it computes what
    the old one did on every number within the clip, and on every number at all with the linear copy unclipped
    (`Config.nums_clip` 0, what `train.py --log-nums` widens a checkpoint to)."""
    old = state["nums.weight"]
    if not net.config.nums_log or old.shape == net.nums.weight.shape:
        return state
    wide = torch.zeros_like(net.nums.weight)
    wide[:, : old.shape[1]] = old
    return dict(state, **{"nums.weight": wide})


def upgrade_schema1(config: dict, state: dict) -> tuple[dict, dict]:
    """A schema-1 network's config and state fitted to this encoding, computing what it did (`Config.schema1_inputs`):
    each categorical slot's rows kept and the rows schema 4 appends started as the rows they were read as before
    (SCHEMA1_NEW_CATS); the new inputs' projections at zero. A state whose category table is not schema 1's is
    refused."""
    cats = state["cats.weight"]
    if cats.shape[0] != sum(SCHEMA1_VOCABS):
        raise ValueError(f"the category table has {cats.shape[0]} rows, schema 1's {sum(SCHEMA1_VOCABS)}")
    if config.get("schema1_inputs"):
        raise ValueError("already upgraded")
    rows, at = [], 0
    for slot, (old, new) in enumerate(zip(SCHEMA1_VOCABS, VOCABS)):
        seg = cats[at : at + old]
        added = SCHEMA1_NEW_CATS.get(slot, [])
        if old + len(added) != new:
            raise ValueError(f"slot {slot}: {old} rows and {len(added)} new are not this encoding's {new}")
        rows.append(torch.cat([seg] + [seg[i : i + 1] for i in added]))
        at += old
    w = cats.shape[1]
    nums_in = state["nums.weight"].shape[1]
    state = dict(state, **{"cats.weight": torch.cat(rows),
                           "nums_new.weight": torch.zeros(w, nums_in, dtype=cats.dtype),
                           "pointers_new.weight": torch.zeros(w, PTRS * w, dtype=cats.dtype)})
    return dict(config, schema1_inputs=True), state


def build(config: Config, table: np.ndarray, seed: int = 0, features: dict | None = None) -> Net:
    torch.manual_seed(seed)
    return Net(config, table, features)


def parameters(net: nn.Module) -> int:
    return sum(p.numel() for p in net.parameters())


def config_dict(config: Config) -> dict:
    return asdict(config)
