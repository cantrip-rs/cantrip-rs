"""The phase-0 spike model (docs/ai-player-plan.md, "Phase 0"): the planned size and shape, untrained, thrown away.

A pre-norm transformer encoder over at most 128 tokens: 4 layers, width 160, 4 heads, masked attention, no
positions. A value head (tanh) over the pooled tokens and a pointer policy over the option tokens, as logits masked
to the legal options. Attention is written out (no fused kernel), so every backend sees plain MatMul and Softmax.
"""

import torch
from torch import nn

FEATURES = 64
KINDS = 8
WIDTH = 160
HEADS = 4
LAYERS = 4
FFN = 4 * WIDTH
MASKED = -1e9


class Attention(nn.Module):
    def __init__(self):
        super().__init__()
        self.qkv = nn.Linear(WIDTH, 3 * WIDTH)
        self.out = nn.Linear(WIDTH, WIDTH)

    def forward(self, x, bias, tokens):
        head = WIDTH // HEADS
        q, k, v = self.qkv(x).reshape(-1, tokens, 3, HEADS, head).permute(2, 0, 3, 1, 4).unbind(0)
        scores = torch.matmul(q, k.transpose(-1, -2)) * (head**-0.5) + bias
        y = torch.matmul(torch.softmax(scores, dim=-1), v)
        return self.out(y.transpose(1, 2).reshape(-1, tokens, WIDTH))


class Block(nn.Module):
    def __init__(self):
        super().__init__()
        self.norm1 = nn.LayerNorm(WIDTH)
        self.attention = Attention()
        self.norm2 = nn.LayerNorm(WIDTH)
        self.ffn = nn.Sequential(nn.Linear(WIDTH, FFN), nn.GELU(), nn.Linear(FFN, WIDTH))

    def forward(self, x, bias, tokens):
        x = x + self.attention(self.norm1(x), bias, tokens)
        return x + self.ffn(self.norm2(x))


class Spike(nn.Module):
    def __init__(self):
        super().__init__()
        self.project = nn.Linear(FEATURES, WIDTH)
        self.kind = nn.Embedding(KINDS, WIDTH)
        self.blocks = nn.ModuleList(Block() for _ in range(LAYERS))
        self.norm = nn.LayerNorm(WIDTH)
        self.value = nn.Sequential(nn.Linear(WIDTH, WIDTH), nn.GELU(), nn.Linear(WIDTH, 1))
        self.policy = nn.Linear(WIDTH, 1)
        # Set for export: a constant token count keeps Shape ops out of the graph; the batch stays free.
        self.tokens = 0
        self.register_buffer("masked", torch.tensor(MASKED), persistent=False)

    def forward(self, features, kind, present, option):
        """features [B,T,F] f32, kind [B,T] i64, present [B,T] bool (false = padding), option [B,T] bool.
        Returns value [B] in (-1, 1) and policy logits [B,T], MASKED where not an option."""
        tokens = self.tokens or features.shape[1]
        x = self.project(features) + self.kind(kind)
        keep = present.to(x.dtype)
        bias = ((1.0 - keep) * MASKED).reshape(-1, 1, 1, tokens)
        for block in self.blocks:
            x = block(x, bias, tokens)
        x = self.norm(x)
        pooled = (x * keep.unsqueeze(-1)).sum(1) / keep.sum(1, keepdim=True).clamp(min=1.0)
        value = torch.tanh(self.value(pooled)).squeeze(-1)
        logits = self.policy(x).squeeze(-1)
        return value, torch.where(option, logits, self.masked)


def positions(n, tokens, seed):
    """`n` random encoded positions padded to `tokens`: a random length each, options among the present tokens."""
    g = torch.Generator().manual_seed(seed)
    lengths = torch.randint(tokens // 4, tokens + 1, (n,), generator=g)
    index = torch.arange(tokens).unsqueeze(0)
    present = index < lengths.unsqueeze(1)
    features = torch.randn(n, tokens, FEATURES, generator=g) * present.unsqueeze(-1)
    kind = torch.randint(0, KINDS, (n, tokens), generator=g) * present
    option = present & (torch.rand(n, tokens, generator=g) < 0.2)
    option[torch.arange(n), lengths - 1] = True
    return features, kind, present, option


def build(seed=0):
    torch.manual_seed(seed)
    return Spike().eval()


def parameters(model):
    return sum(p.numel() for p in model.parameters())
