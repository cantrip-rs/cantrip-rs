"""The inference server (docs/ai-player-plan.md, "Decisions": GPU inference for self-play and server seats): one process
holding the networks on the GPU, which every self-play lane and every AI host sends its leaves to
(`cantrip_ai::remote::RemoteEvaluator`), and which runs the rows of all of them in large batches.

    python -m cantrip_ai.serve --model DIR [--model DIR ...] [--host 127.0.0.1] [--port 7710] [--device auto]
        [--batch N | --batch T:N,...] [--flush-ms 2] [--pipelines 2] [--precision fp32|tf32|fp16|bf16] [--cuda-graphs on|off]
        [--ready-file PATH]

A model directory is export's (`manifest.json` and `model.pt`, the weights the server runs; `export --weights-only`
writes `model.pt` into a directory exported before export wrote it). At start each model's parity references
(`reference/`) are run through the serving path and checked against PyTorch's: at fp32 within export's tolerances
(value, aux and legal logits within 1e-3), below it within `REDUCED` (what the search uses: value, option probabilities,
dead-next-turn within 1e-2; the life swing within half a point, or 1% above 50 points), and the same best option on 99.5%; a model that fails
is refused (`--no-check` skips it).

The wire protocol is `crates/cantrip-ai/src/remote.rs`'s: TCP, length-prefixed frames, a hello that names the model by
its manifest's FNV-1a 64 and carries the client's encoding schema and encoder hash, which must be the model's and
`ai/encoding.json`'s. A client sends a batch's real tokens (narrow types, the numbers sparse as f16), and gets back the
value, aux and the logits at the legal tokens.

**Batching.** Rows from every connection are queued per model and token bucket; a row goes to the smallest bucket that
holds its real tokens. A GPU call takes up to `--batch` rows of one queue and is always padded to exactly that many,
so every call of a bucket has one shape. A queue is run when it holds a full batch, when every connected client is
waiting on the server (no more rows can come), or when its oldest row has waited `--flush-ms`. `--pipelines` workers
assemble and run batches at once, each on its own CUDA stream with its own buffers, so one batch is copied in while
another computes.

**Reproducibility.** A row's outputs never depend on what it was batched with: one shape per bucket, deterministic
algorithms (`torch.use_deterministic_algorithms`, `CUBLAS_WORKSPACE_CONFIG=:4096:8`, no cuDNN autotuning), and no
reduction across rows in the network. So self-play through the server plays the same games at the same seed.
`--precision fp32` (the default) keeps TF32 off; `tf32` allows it in matrix products; `fp16` and `bf16` run the network
under autocast (measured in docs/ai-training.md). Reproducibility holds at each precision, not across them.
CUDA graphs (`--cuda-graphs`, on by default on a GPU) replay each bucket's captured forward, the same kernels.

ROCm builds of PyTorch present the GPU as `cuda`, so `--device auto` serves on an AMD GPU too; the CPU serves as well
(slowly; a smaller `--batch`).
"""

from __future__ import annotations

import argparse
import contextlib
import json
import os
import selectors
import socket
import struct
import sys
import threading
import time
from collections import deque
from dataclasses import dataclass, field
from pathlib import Path

# Before CUDA starts: cuBLAS is deterministic only with a fixed workspace.
os.environ.setdefault("CUBLAS_WORKSPACE_CONFIG", ":4096:8")

import numpy as np  # noqa: E402
import torch  # noqa: E402

from . import dataset, model  # noqa: E402

MAGIC = b"CNTRPAI\0"
PROTOCOL = 1
EVALUATE = 1
NUMS_SPARSE = 1
NUMS_DENSE = 2
MAX_FRAME = 1 << 28
BUCKETS = (32, 64, 128)
MAX_TOKENS = BUCKETS[-1]
CATS, PTRS, NUMS = model.CATS, model.PTRS, model.NUMS
NO_POINTER = model.NO_POINTER
MASKED = np.float32(model.MASKED)
TOLERANCE = 1e-3
SAME_ARGMAX = 0.995
# What a precision below fp32 is held to: the outputs the search uses, within what rounding moves them and well inside
# what the network itself gets wrong. The value (-1..1) and the dead-next-turn probability within 1e-2, each legal
# option's probability (the search's prior) within 1e-2, the same best option on 99.5%; the life swing (in life points,
# which no search reads) within half a point, or 1% of the swing above 50 points (`SWING_RELATIVE`: fp16's own spacing is
# a quarter point at 256, and a network that predicts a swing of hundreds holds it only to its relative precision; A'
# predicts 329 on one reference and moves it by 0.7-1.2 under autocast, with no activation near fp16's range). Measured on gen0 (docs/ai-training.md): fp16 and tf32 hold to it, bf16
# does not (its 8-bit mantissa moves logits by up to 1).
REDUCED = dict(value=1e-2, dead=1e-2, probability=1e-2, swing=0.5)
SWING_RELATIVE = 50.0  # above this many points, the swing's drift is measured against 1% of it
DEFAULT_PORT = 7710


def fnv1a64(data: bytes) -> int:
    h = 0xCBF29CE484222325
    for b in data:
        h = ((h ^ b) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return h


HALF = {"fp16": torch.float16, "bf16": torch.bfloat16}


def deterministic(precision: str = "fp32"):
    """The kernels chosen the same way every call: what makes a row's outputs independent of its companions."""
    torch.use_deterministic_algorithms(True)
    torch.backends.cudnn.benchmark = False
    torch.backends.cudnn.deterministic = True
    tf32 = precision == "tf32"
    torch.backends.cuda.matmul.allow_tf32 = tf32
    torch.backends.cudnn.allow_tf32 = tf32
    torch.set_float32_matmul_precision("high" if tf32 else "highest")


def choose_device(name: str) -> torch.device:
    """`auto` is CUDA (HIP on a ROCm build of PyTorch) when there is a GPU, else the CPU."""
    if name == "auto":
        return torch.device("cuda" if torch.cuda.is_available() else "cpu")
    return torch.device(name)


# --- The wire -----------------------------------------------------------------------------------------------------


class ProtocolError(Exception):
    pass


def hello(identity: int, name: str, schema: int | None = None, encoder: int | None = None) -> bytes:
    """A client's hello (the tests' client; the Rust one is `remote::hello`)."""
    schema = model.LAYOUT["schema"] if schema is None else schema
    encoder = int(dataset.encoder_hash(), 16) if encoder is None else encoder
    n = name.encode()
    return MAGIC + struct.pack("<IIQQH", PROTOCOL, schema, encoder, identity, len(n)) + n


def read_hello(p: bytes) -> tuple[int, int, int, int, str]:
    if len(p) < 34 or p[:8] != MAGIC:
        raise ProtocolError("not a Cantrip inference client")
    protocol, schema, encoder, identity, n = struct.unpack_from("<IIQQH", p, 8)
    if len(p) != 34 + n:
        raise ProtocolError("a hello of the wrong length")
    return protocol, schema, encoder, identity, p[34:].decode(errors="replace")


def status(ok: bool, text: str) -> bytes:
    t = text.encode()[:65535]
    return struct.pack("<BH", 0 if ok else 1, len(t)) + t


def frame(payload: bytes) -> bytes:
    return struct.pack("<I", len(payload)) + payload


@dataclass
class Chunk:
    """The rows of one request bound for one bucket, their real tokens end to end, ready to be put into a batch."""

    request: "Request"
    tokens: int  # the bucket
    rows: np.ndarray  # i64 [R]: the request's rows
    tok_cum: np.ndarray  # i64 [R + 1]: row i's tokens are tok_cum[i]..tok_cum[i + 1]
    num_cum: np.ndarray  # i64 [R + 1]: and its non-zero numbers num_cum[i]..num_cum[i + 1]
    kind: np.ndarray  # u8 [n]
    card: np.ndarray  # i32 [n]
    cats: np.ndarray  # u8 [n, CATS]
    ptrs: np.ndarray  # u8 [n, PTRS]
    legal: np.ndarray  # bool [n]
    pos: np.ndarray  # i64 [n]: the token's place in its row
    row: np.ndarray  # i64 [n]: its row, by its index in `rows`
    ntok: np.ndarray  # i64 [k]: each non-zero number's token
    nfeat: np.ndarray  # i64 [k]: and feature
    nval: np.ndarray  # f32 [k]


@dataclass
class Request:
    """One evaluate frame: its rows split by bucket, and what the network makes of them as it comes back."""

    conn: "Conn"
    rows: int
    chunks: list
    legal_row: np.ndarray  # the legal tokens' rows and places, in the order the reply gives their logits
    legal_pos: np.ndarray
    value: np.ndarray = None
    aux: np.ndarray = None
    policy: np.ndarray = None  # f32 [rows, MAX_TOKENS]
    remaining: int = 0
    arrived: float = 0.0


def decode(p: bytes) -> dict:
    """An evaluate frame's arrays (the inverse of `remote::request`): each row's real tokens end to end, and the
    non-zero numbers as (token, feature, value)."""
    if len(p) < 7 or p[0] != EVALUATE:
        raise ProtocolError("not an evaluate frame")
    rows, width, form = struct.unpack_from("<IBB", p, 1)
    if width not in (2, 4):
        raise ProtocolError(f"a card width of {width}")
    at = 7
    data = np.frombuffer(p, np.uint8)

    def take(n: int) -> np.ndarray:
        nonlocal at
        if at + n > len(p):
            raise ProtocolError("a frame cut short")
        s = data[at : at + n]
        at += n
        return s

    lengths = take(rows).astype(np.int64)
    if (lengths > MAX_TOKENS).any():
        raise ProtocolError("a row longer than the largest bucket")
    n = int(lengths.sum())
    kind = take(n)
    card = take(n * width).view("<u2" if width == 2 else "<u4")
    cats = take(n * CATS).reshape(n, CATS)
    ptrs = take(n * PTRS).reshape(n, PTRS)
    legal = np.unpackbits(take((n + 7) // 8), bitorder="little")[:n].astype(bool)
    if form == NUMS_SPARSE:
        counts = take(n)
        ntok = np.repeat(np.arange(n), counts)
        nfeat = take(len(ntok)).astype(np.int64)
        nval = take(2 * len(ntok)).view("<f2").astype(np.float32)
        if len(nfeat) and nfeat.max() >= NUMS:
            raise ProtocolError("a number's feature past the last")
    elif form == NUMS_DENSE:
        nums = take(4 * n * NUMS).view("<f4").reshape(n, NUMS)
        ntok, nfeat = np.nonzero(nums.view(np.uint32) != 0)
        nval = nums[ntok, nfeat].astype(np.float32)
    else:
        raise ProtocolError(f"a numbers form {form}")
    if at != len(p):
        raise ProtocolError("a frame with bytes left over")
    if width == 4 and (card > np.iinfo(np.int32).max).any():
        raise ProtocolError("a card row past the largest table")
    return dict(rows=rows, lengths=lengths, kind=kind, card=card.astype(np.int32), cats=cats, ptrs=ptrs, legal=legal,
                ntok=ntok, nfeat=nfeat, nval=nval)


def request(conn, d: dict) -> Request:
    """A decoded frame as a request: its rows split by the smallest bucket that holds each."""
    rows, lengths = d["rows"], d["lengths"]
    starts = np.zeros(rows, np.int64)
    np.cumsum(lengths[:-1], out=starts[1:])
    row_of = np.repeat(np.arange(rows), lengths)
    pos = np.arange(len(row_of)) - starts[row_of]
    bucket = np.searchsorted(np.array(BUCKETS), np.maximum(lengths, 1))
    chunks = []
    if rows and (bucket[1:] >= bucket[:-1]).all():
        # Rows by bucket, as the Rust client sends them: each bucket's rows and tokens are one run, taken as views.
        ends = np.searchsorted(bucket, np.arange(len(BUCKETS)), side="right")
        tok_ends = np.concatenate([[0], np.cumsum(lengths)])
        num_ends = np.searchsorted(d["ntok"], tok_ends)
        r0 = 0
        for b, t in enumerate(BUCKETS):
            r1 = int(ends[b])
            if r1 == r0:
                continue
            t0, t1, k0, k1 = tok_ends[r0], tok_ends[r1], num_ends[r0], num_ends[r1]
            chunks.append(Chunk(None, t, np.arange(r0, r1), tok_ends[r0 : r1 + 1] - t0, num_ends[r0 : r1 + 1] - k0,
                                d["kind"][t0:t1], d["card"][t0:t1], d["cats"][t0:t1], d["ptrs"][t0:t1],
                                d["legal"][t0:t1], pos[t0:t1], row_of[t0:t1] - r0, d["ntok"][k0:k1] - t0,
                                d["nfeat"][k0:k1], d["nval"][k0:k1]))
            r0 = r1
    for b, t in enumerate(BUCKETS if not chunks else ()):
        mine = np.flatnonzero(bucket == b)
        if not len(mine):
            continue
        if len(mine) == rows:
            tok = slice(None)
            ntok, nfeat, nval = d["ntok"], d["nfeat"], d["nval"]
            rank = row_of
        else:
            keep = bucket[row_of] == b
            tok = np.flatnonzero(keep)
            local = np.cumsum(keep) - 1
            sel = keep[d["ntok"]]
            ntok, nfeat, nval = local[d["ntok"][sel]], d["nfeat"][sel], d["nval"][sel]
            index = np.empty(rows, np.int64)
            index[mine] = np.arange(len(mine))
            rank = index[row_of[tok]]
        r = len(mine)
        tok_cum = np.zeros(r + 1, np.int64)
        np.cumsum(lengths[mine], out=tok_cum[1:])
        num_cum = np.zeros(r + 1, np.int64)
        np.cumsum(np.bincount(rank[ntok], minlength=r), out=num_cum[1:])
        chunks.append(Chunk(None, t, mine, tok_cum, num_cum, d["kind"][tok], d["card"][tok], d["cats"][tok],
                            d["ptrs"][tok], d["legal"][tok], pos[tok], rank, ntok, nfeat, nval))
    legal = d["legal"]
    r = Request(conn=conn, rows=rows, chunks=chunks, legal_row=row_of[legal], legal_pos=pos[legal])
    for c in chunks:
        c.request = r
    r.value = np.zeros(rows, np.float32)
    r.aux = np.zeros((rows, 2), np.float32)
    r.policy = np.full((rows, MAX_TOKENS), MASKED, np.float32)
    r.remaining = rows
    return r


def encode(kind, card, cats, ptrs, nums, legal) -> bytes:
    """A client's evaluate frame for padded arrays ([rows, T], ...), as `remote::request` writes it (the tests' client).
    Sparse f16 numbers when they are exact in f16, else dense f32."""
    kind, card, cats, ptrs = (np.asarray(x, np.int64) for x in (kind, card, cats, ptrs))
    nums, legal = np.asarray(nums, np.float32), np.asarray(legal, bool)
    rows, t = kind.shape
    pad = ((kind == 0) & (card == 0) & ~legal & (cats == 0).all(-1) & (ptrs == NO_POINTER).all(-1)
           & (nums.view(np.uint32) == 0).all(-1))
    lengths = np.array([t - int(np.argmin(r[::-1])) if not r.all() else 0 for r in pad], np.int64)
    real = np.arange(t)[None, :] < lengths[:, None]
    wide = bool((card[real] > 65535).any())
    exact = bool((nums[real].astype(np.float16).astype(np.float32).view(np.uint32) == nums[real].view(np.uint32)).all())
    out = [struct.pack("<BIBB", EVALUATE, rows, 4 if wide else 2, NUMS_SPARSE if exact else NUMS_DENSE),
           lengths.astype(np.uint8).tobytes(), kind[real].astype(np.uint8).tobytes(),
           card[real].astype("<u4" if wide else "<u2").tobytes(), cats[real].astype(np.uint8).tobytes(),
           ptrs[real].astype(np.uint8).tobytes(), np.packbits(legal[real], bitorder="little").tobytes()]
    if exact:
        n = nums[real]
        mask = n.view(np.uint32) != 0
        tok, feat = np.nonzero(mask)
        out += [mask.sum(1).astype(np.uint8).tobytes(), feat.astype(np.uint8).tobytes(), n[tok, feat].astype("<f2").tobytes()]
    else:
        out.append(nums[real].astype("<f4").tobytes())
    return b"".join(out)


def read_reply(p: bytes, legal: np.ndarray) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """A reply as value [rows], aux [rows, 2] and the policy [rows, T] (`MASKED` where not legal)."""
    if p[0] != 0:
        (n,) = struct.unpack_from("<H", p, 1)
        raise ProtocolError(p[3 : 3 + n].decode(errors="replace"))
    rows = legal.shape[0]
    value = np.frombuffer(p, "<f4", rows, 1)
    aux = np.frombuffer(p, "<f4", 2 * rows, 1 + 4 * rows).reshape(rows, 2)
    policy = np.full(legal.shape, MASKED, np.float32)
    policy[legal] = np.frombuffer(p, "<f4", int(legal.sum()), 1 + 12 * rows)
    return value, aux, policy


# --- The models ---------------------------------------------------------------------------------------------------


def bucket_of(length: int) -> int:
    for b in BUCKETS:
        if b >= length:
            return b
    raise ProtocolError(f"{length} tokens")


class Runner:
    """One model's forward at one bucket and batch size, on one stream. A batch's real tokens are written into padded
    inputs in pinned host memory (one vectorised scatter a field, and the last batch's tokens put back to padding),
    copied to the device, and run through the network: the captured CUDA graph when graphs are on. The inputs always
    have the same shape, so every call takes the same kernels."""

    def __init__(self, net, device: torch.device, tokens: int, batch: int, graphs: bool, half: torch.dtype | None = None):
        self.net, self.device, self.t, self.b, self.half = net, device, tokens, batch, half
        cuda = device.type == "cuda"
        b, t = batch, tokens
        cap, kcap = b * t, b * t * NUMS
        # The batch's tokens end to end, before they are scattered.
        self.stage = dict(kind=np.zeros(cap, np.uint8), card=np.zeros(cap, np.int32), cats=np.zeros((cap, CATS), np.uint8),
                          ptrs=np.zeros((cap, PTRS), np.uint8), legal=np.zeros(cap, bool), slot=np.zeros(cap, np.int64),
                          pos=np.zeros(cap, np.int64), ntok=np.zeros(kcap, np.int64), nfeat=np.zeros(kcap, np.int64),
                          nval=np.zeros(kcap, np.float32))
        self.host = (torch.zeros((b, t), dtype=torch.uint8, pin_memory=cuda),
                     torch.zeros((b, t), dtype=torch.int32, pin_memory=cuda),
                     torch.zeros((b, t, CATS), dtype=torch.uint8, pin_memory=cuda),
                     torch.full((b, t, PTRS), NO_POINTER, dtype=torch.uint8, pin_memory=cuda),
                     torch.zeros((b, t, NUMS), dtype=torch.float32, pin_memory=cuda),
                     torch.zeros((b, t), dtype=torch.bool, pin_memory=cuda))
        self.kind, self.card, self.cats, self.ptrs, self.nums, self.legal = (x.numpy() for x in self.host)
        self.written = None  # the last batch's token places and number places, to put back to padding
        self.stream = torch.cuda.Stream(device) if cuda else None
        with self.on_stream():
            self.inputs = tuple(x.to(device) for x in self.host) if cuda else self.host
        self.out_host = (torch.zeros((b,), pin_memory=cuda), torch.zeros((b, 2), pin_memory=cuda),
                         torch.zeros((b, t), pin_memory=cuda))
        self.value, self.aux, self.policy = (x.numpy() for x in self.out_host)
        self.graph = None
        if cuda:
            self.stream.synchronize()
            if graphs:
                # Warm up on the side stream (as torch.cuda.graphs asks), then capture.
                with self.on_stream():
                    for _ in range(2):
                        self._forward()
                self.stream.synchronize()
                self.graph = torch.cuda.CUDAGraph()
                with torch.cuda.graph(self.graph, stream=self.stream):
                    self.outputs = self._forward()
                self.stream.synchronize()

    def on_stream(self):
        return torch.cuda.stream(self.stream) if self.stream is not None else contextlib.nullcontext()

    def _forward(self):
        kind, card, cats, ptrs, nums, legal = self.inputs
        with torch.inference_mode(), torch.autocast(self.device.type, dtype=self.half or torch.float16,
                                                    enabled=self.half is not None):
            value, policy, aux, _ = self.net(kind.long(), card.long(), cats.long(), ptrs.long(), nums, legal)
        return value.float(), aux.float(), policy.float()

    def run(self, parts) -> int:
        """`parts`, a list of (chunk, first row, end row), at slots 0.. in order, through the network: the outputs in
        `value`, `aux` and `policy` when it returns. Returns the rows used."""
        h = self.stage
        at = kat = slot = 0
        for c, r0, r1 in parts:
            t0, t1 = c.tok_cum[r0], c.tok_cum[r1]
            k0, k1 = c.num_cum[r0], c.num_cum[r1]
            n, k = t1 - t0, k1 - k0
            h["kind"][at : at + n] = c.kind[t0:t1]
            h["card"][at : at + n] = c.card[t0:t1]
            h["cats"][at : at + n] = c.cats[t0:t1]
            h["ptrs"][at : at + n] = c.ptrs[t0:t1]
            h["legal"][at : at + n] = c.legal[t0:t1]
            h["pos"][at : at + n] = c.pos[t0:t1]
            np.add(c.row[t0:t1], slot - r0, out=h["slot"][at : at + n])
            np.add(c.ntok[k0:k1], at - t0, out=h["ntok"][kat : kat + k])
            h["nfeat"][kat : kat + k] = c.nfeat[k0:k1]
            h["nval"][kat : kat + k] = c.nval[k0:k1]
            at, kat, slot = at + n, kat + k, slot + (r1 - r0)
        if slot > self.b:
            raise ValueError(f"{slot} rows in a batch of {self.b}")
        if self.written is not None:
            s, p, ns, np_, nf = self.written
            self.kind[s, p] = 0
            self.card[s, p] = 0
            self.cats[s, p] = 0
            self.ptrs[s, p] = NO_POINTER
            self.legal[s, p] = False
            self.nums[ns, np_, nf] = 0.0
        s, p = h["slot"][:at].copy(), h["pos"][:at].copy()
        nt = h["ntok"][:kat]
        ns, np_, nf = s[nt], p[nt], h["nfeat"][:kat].copy()
        self.kind[s, p] = h["kind"][:at]
        self.card[s, p] = h["card"][:at]
        self.cats[s, p] = h["cats"][:at]
        self.ptrs[s, p] = h["ptrs"][:at]
        self.legal[s, p] = h["legal"][:at]
        self.nums[ns, np_, nf] = h["nval"][:kat]
        self.written = (s, p, ns, np_, nf)
        if self.stream is None:
            with torch.inference_mode():
                outputs = self._forward()
            for host, out in zip(self.out_host, outputs):
                host.copy_(out)
            return slot
        with self.on_stream():
            for d, x in zip(self.inputs, self.host):
                d.copy_(x, non_blocking=True)
            if self.graph is not None:
                self.graph.replay()
                outputs = self.outputs
            else:
                outputs = self._forward()
            for host, out in zip(self.out_host, outputs):
                host.copy_(out, non_blocking=True)
        self.stream.synchronize()
        return slot


class Served:
    """A model directory loaded for serving."""

    def __init__(self, directory: Path, device: torch.device):
        self.dir = Path(directory)
        self.name = self.dir.name
        manifest_bytes = (self.dir / "manifest.json").read_bytes()
        self.identity = fnv1a64(manifest_bytes)
        self.manifest = json.loads(manifest_bytes)
        if self.manifest.get("schema") != model.LAYOUT["schema"] or self.manifest.get("encoder_hash") != dataset.encoder_hash():
            raise SystemExit(f"{self.dir}: encoding ({self.manifest.get('schema')}, {self.manifest.get('encoder_hash')}) "
                             f"is not ai/encoding.json's ({model.LAYOUT['schema']}, {dataset.encoder_hash()})")
        path = self.dir / "model.pt"
        if not path.exists():
            raise SystemExit(f"{path} is missing: export the model again, or write it with `python -m cantrip_ai.export "
                             f"--checkpoint CHECKPOINT --out {self.dir} --weights-only`")
        w = torch.load(path, map_location="cpu", weights_only=True)
        net = model.Net(model.Config(**w["config"]), w["state"]["table"].numpy())
        net.load_state_dict(w["state"])
        net.eval()
        if w["folded"]:
            net.folded = w["table"]
        net.to(device)
        if net.folded is not None:
            net.folded = net.folded.to(device)
        self.net = net
        self.parameters = model.parameters(net)


# --- The server ---------------------------------------------------------------------------------------------------


@dataclass
class Group:
    """Rows `first..end` of a chunk, bound for its queue."""

    chunk: Chunk
    first: int
    end: int

    @property
    def rows(self) -> int:
        return self.end - self.first


@dataclass
class Queue:
    groups: deque = field(default_factory=deque)
    rows: int = 0
    oldest: float = 0.0


class Conn:
    def __init__(self, sock: socket.socket, address):
        self.sock, self.address = sock, address
        self.buffer = bytearray()
        self.model: Served | None = None
        self.outstanding = False
        self.closed = False  # by the reading thread, which alone unregisters and closes the socket
        self.send_lock = threading.Lock()

    def send(self, payload: bytes):
        with self.send_lock:
            try:
                self.sock.sendall(frame(payload))
            except OSError:
                pass

    def hang_up(self):
        """End the connection from a worker: the reading thread sees it end and closes it."""
        try:
            self.sock.shutdown(socket.SHUT_RDWR)
        except OSError:
            pass


class Server:
    def __init__(self, models: list[Served], device: torch.device, batch: dict[int, int], flush: float,
                 pipelines: int, graphs: bool, log=None, half: torch.dtype | None = None):
        self.models = {m.identity: m for m in models}
        self.device, self.batch, self.flush = device, batch, flush
        self.log = log or (lambda *parts: print(*parts, flush=True))
        self.log_every = 60.0
        self.lock = threading.Condition()
        self.queues: dict[tuple[int, int], Queue] = {(m.identity, t): Queue() for m in models for t in BUCKETS}
        self.connections = 0  # welcomed and open
        self.waiting = 0  # of those, with a request not yet answered
        self.running = 0  # batches on the GPU
        self.stopping = False
        self.stats = dict(requests=0, rows=0, batches=0, padded=0, gpu_seconds=0.0, started=time.time())
        self.runners = [{(m.identity, t): Runner(m.net, device, t, batch[t], graphs, half) for m in models for t in BUCKETS}
                        for _ in range(pipelines)]
        self.workers = [threading.Thread(target=self.work, args=(k,), daemon=True, name=f"serve-{k}")
                        for k in range(pipelines)]

    # The workers.

    def ready(self, now: float):
        """The queue to run now, and how long to wait if none."""
        best, wait = None, None
        for key, q in self.queues.items():
            if not q.rows:
                continue
            if q.rows >= self.batch[key[1]]:
                return key, None
            if best is None or q.oldest < self.queues[best].oldest:
                best = key
        if best is None:
            return None, None
        # Every client waiting and nothing running: no row can come until this runs. (While a batch runs, its answers
        # bring more rows, and a partial batch costs the GPU as much as a full one.)
        if (self.waiting >= self.connections and not self.running) or now - self.queues[best].oldest >= self.flush:
            return best, None
        return None, self.flush - (now - self.queues[best].oldest)

    def take(self, key) -> list[Group]:
        q, room, out = self.queues[key], self.batch[key[1]], []
        while q.groups and room:
            g = q.groups[0]
            if g.rows <= room:
                q.groups.popleft()
                out.append(g)
                room -= g.rows
            else:
                out.append(Group(g.chunk, g.first, g.first + room))
                g.first += room
                room = 0
        q.rows -= sum(g.rows for g in out)
        q.oldest = q.groups[0].chunk.request.arrived if q.groups else 0.0
        return out

    def work(self, k: int):
        while True:
            with self.lock:
                while True:
                    if self.stopping:
                        return
                    key, wait = self.ready(time.perf_counter())
                    if key is not None:
                        break
                    self.lock.wait(wait)
                groups = self.take(key)
                self.running += 1
            runner = self.runners[k][key]
            t0 = time.perf_counter()
            try:
                runner.run([(g.chunk, g.first, g.end) for g in groups])
            except Exception as e:  # noqa: BLE001: a failed batch answers its requests with the error
                self.log(f"serve: a batch failed: {e!r}")
                with self.lock:
                    self.running -= 1
                    self.lock.notify_all()
                for g in groups:
                    self.fail(g.chunk.request, f"the server's batch failed: {e!r}")
                continue
            done = []
            slot = 0
            for g in groups:
                n, c = g.rows, g.chunk
                rows = c.rows[g.first : g.end]
                r = c.request
                r.value[rows] = runner.value[slot : slot + n]
                r.aux[rows] = runner.aux[slot : slot + n]
                r.policy[rows, : runner.t] = runner.policy[slot : slot + n]
                slot += n
            t1 = time.perf_counter()
            with self.lock:
                self.running -= 1
                self.lock.notify_all()
                self.stats["batches"] += 1
                self.stats["padded"] += runner.b
                self.stats["gpu_seconds"] += t1 - t0
                self.stats[f"batches{runner.t}"] = self.stats.get(f"batches{runner.t}", 0) + 1
                self.stats[f"rows{runner.t}"] = self.stats.get(f"rows{runner.t}", 0) + slot
                for g in groups:
                    r = g.chunk.request
                    r.remaining -= g.rows
                    if r.remaining == 0 and r.conn.outstanding:
                        done.append(r)
                        self.waiting -= 1
                        r.conn.outstanding = False
            for r in done:
                r.conn.send(self.reply(r))
            t2 = time.perf_counter()
            self.add("reply_s", t2 - t1)
            self.add("latency_s", sum(t2 - r.arrived for r in done))
            self.add("queued_s", sum(t0 - g.chunk.request.arrived for g in groups))
            self.add("groups", len(groups))

    @staticmethod
    def reply(r: Request) -> bytes:
        logits = r.policy[r.legal_row, r.legal_pos]
        return b"\0" + r.value.tobytes() + r.aux.tobytes() + logits.tobytes()

    def fail(self, r: Request, why: str):
        with self.lock:
            if not r.conn.outstanding:
                return  # answered already, by another of its batches failing
            r.conn.outstanding = False
            self.waiting -= 1
        r.conn.send(status(False, why))
        r.conn.hang_up()

    # The connections.

    def add(self, key: str, seconds: float):
        self.stats[key] = self.stats.get(key, 0.0) + seconds

    def submit(self, conn: Conn, payload: bytes):
        t0 = time.perf_counter()
        r = request(conn, decode(payload))
        r.arrived = time.perf_counter()
        self.add("decode_s", r.arrived - t0)
        if r.rows == 0:
            conn.send(self.reply(r))
            return
        with self.lock:
            self.stats["requests"] += 1
            self.stats["rows"] += r.rows
            conn.outstanding = True
            self.waiting += 1
            for c in r.chunks:
                q = self.queues[(conn.model.identity, c.tokens)]
                if not q.rows:
                    q.oldest = r.arrived
                q.groups.append(Group(c, 0, len(c.rows)))
                q.rows += len(c.rows)
            self.lock.notify_all()

    def welcome(self, conn: Conn, payload: bytes) -> bool:
        protocol, schema, encoder, identity, name = read_hello(payload)
        ours = int(dataset.encoder_hash(), 16)
        why = None
        if protocol != PROTOCOL:
            why = f"protocol {protocol}; this server speaks {PROTOCOL}"
        elif schema != model.LAYOUT["schema"] or encoder != ours:
            why = f"encoding ({schema}, {encoder:016x}); this server's is ({model.LAYOUT['schema']}, {ours:016x})"
        elif identity not in self.models:
            why = (f"no model {name} with that manifest here; this server has "
                   + ", ".join(f"{m.name} ({m.dir})" for m in self.models.values()))
        if why:
            conn.send(status(False, why))
            return False
        conn.model = self.models[identity]
        gpu = torch.cuda.get_device_name(self.device) if self.device.type == "cuda" else "cpu"
        conn.send(status(True, f"{conn.model.name} on {self.device} ({gpu})"))
        with self.lock:
            self.connections += 1
        return True

    def close(self, conn: Conn):
        """Called by the reading thread only."""
        with self.lock:
            if conn.closed:
                return
            conn.closed = True
            if conn.model is not None:
                self.connections -= 1
            if conn.outstanding:
                # Its rows still run; the answer goes nowhere.
                conn.outstanding = False
                self.waiting -= 1
            self.lock.notify_all()
        try:
            self.selector.unregister(conn.sock)
        except (KeyError, ValueError):
            pass
        conn.sock.close()

    def readable(self, conn: Conn):
        try:
            data = conn.sock.recv(1 << 20)
        except OSError:
            data = b""
        if not data:
            self.close(conn)
            return
        conn.buffer += data
        while len(conn.buffer) >= 4:
            (n,) = struct.unpack_from("<I", conn.buffer, 0)
            if n > MAX_FRAME:
                conn.send(status(False, f"a frame of {n} bytes"))
                self.close(conn)
                return
            if len(conn.buffer) < 4 + n:
                break
            payload = bytes(conn.buffer[4 : 4 + n])
            del conn.buffer[: 4 + n]
            try:
                if conn.model is None:
                    if not self.welcome(conn, payload):
                        self.close(conn)
                        return
                elif conn.outstanding:
                    raise ProtocolError("a request before the last was answered")
                else:
                    self.submit(conn, payload)
            except ProtocolError as e:
                conn.send(status(False, str(e)))
                self.close(conn)
                return

    def serve(self, listener: socket.socket, ready=None):
        self.selector = selectors.DefaultSelector()
        listener.setblocking(False)
        self.selector.register(listener, selectors.EVENT_READ, None)
        for w in self.workers:
            w.start()
        if ready:
            ready()
        last = time.time()
        try:
            while not self.stopping:
                for key, _ in self.selector.select(timeout=1.0):
                    if key.data is None:
                        try:
                            sock, address = listener.accept()
                        except OSError:
                            continue
                        sock.setblocking(True)
                        sock.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
                        conn = Conn(sock, address)
                        self.selector.register(sock, selectors.EVENT_READ, conn)
                    else:
                        self.readable(key.data)
                if time.time() - last >= self.log_every:
                    last = time.time()
                    self.log(self.summary())
        finally:
            self.stop()

    def stop(self):
        with self.lock:
            self.stopping = True
            self.lock.notify_all()

    def summary(self) -> str:
        s = self.stats
        seconds = max(time.time() - s["started"], 1e-9)
        with self.lock:
            fill = s["rows"] / max(s["padded"], 1)
            buckets = ", ".join(f"T{t}: {s.get(f'rows{t}', 0)} rows in {s.get(f'batches{t}', 0)} batches" for t in BUCKETS)
            return (f"serve: {s['requests']} requests, {s['rows']} rows ({s['rows'] / seconds:.0f} a second) in "
                    f"{s['batches']} batches ({buckets}), {fill:.0%} of the padded rows real; a batch running "
                    f"{s['gpu_seconds'] / seconds:.0%} of the time; seconds a second decoding "
                    f"{s.get('decode_s', 0) / seconds:.2f}, assembling {s.get('assemble_s', 0) / seconds:.2f}, replying "
                    f"{s.get('reply_s', 0) / seconds:.2f}; a request {1e3 * s.get('latency_s', 0) / max(s['requests'], 1):.1f} ms "
                    f"in the server, a row group queued {1e3 * s.get('queued_s', 0) / max(s.get('groups', 0), 1):.1f} ms; "
                    f"{self.connections} connections")


# --- Checking the references --------------------------------------------------------------------------------------


def check(served: Served, runners: dict, log=print, precision: str = "fp32") -> dict:
    """The model's references through the serving path (the wire form, each at its own bucket, padded to the batch),
    against PyTorch's: at fp32 within export's tolerances, at a reduced precision within `REDUCED`. Returns the drifts;
    exits if they are outside."""
    from . import export  # noqa: PLC0415 (onnx is needed only here)

    ref = served.dir / "reference"
    positions = export.read_positions(ref / "positions.bin")
    want_value = np.fromfile(ref / "value.f32", np.float32)
    want_aux = np.fromfile(ref / "auxiliary.f32", np.float32).reshape(-1, 2)
    want_policy = np.fromfile(ref / "policy.f32", np.float32)
    offsets = np.concatenate([[0], np.cumsum([p[4] for p in positions])])
    drift = dict(value=0.0, aux=0.0, policy=0.0)
    reduced = dict(swing=0.0, dead=0.0, probability=0.0)
    same = 0
    for t in BUCKETS:
        rows = [i for i, p in enumerate(positions) if p[1] == t]
        if not rows:
            continue
        runner = runners[(served.identity, t)]
        x = export.inputs_of([positions[i] for i in rows], t)
        r = request(None, decode(encode(*(x[k] for k in model.INPUTS))))
        if [c.tokens for c in r.chunks] != [t]:
            raise SystemExit(f"{served.dir}: a reference position of the {t}-token bucket does not fill it")
        c = r.chunks[0]
        for at in range(0, len(rows), runner.b):
            end = min(at + runner.b, len(rows))
            runner.run([(c, at, end)])
            for j, i in enumerate(rows[at:end]):
                first, options = positions[i][3], positions[i][4]
                legal = x["legal"][at + j, first : first + options]
                got = runner.policy[j, first : first + options]
                want = want_policy[offsets[i] : offsets[i] + options]
                drift["value"] = max(drift["value"], float(abs(runner.value[j] - want_value[i])))
                # The swing in life points, held as below fp32 to its relative size above SWING_RELATIVE points.
                swing_scale = max(1.0, abs(float(want_aux[i, 0])) / SWING_RELATIVE)
                drift["aux"] = max(drift["aux"], float(abs(runner.aux[j, 0] - want_aux[i, 0])) / swing_scale,
                                   float(abs(runner.aux[j, 1] - want_aux[i, 1])))
                reduced["swing"] = max(reduced["swing"], float(abs(runner.aux[j, 0] - want_aux[i, 0]))
                                       / max(1.0, abs(float(want_aux[i, 0])) / SWING_RELATIVE))
                reduced["dead"] = max(reduced["dead"], float(abs(runner.aux[j, 1] - want_aux[i, 1])))
                if legal.any():
                    drift["policy"] = max(drift["policy"], float(np.abs(got[legal] - want[legal]).max()))
                    reduced["probability"] = max(reduced["probability"],
                                                 float(np.abs(softmax(got[legal]) - softmax(want[legal])).max()))
                    choices = np.flatnonzero(legal)
                    same += choices[np.argmax(got[legal])] == choices[np.argmax(want[legal])]
                else:
                    same += 1
    out = dict(positions=len(positions), **drift, same_argmax=float(same) / max(len(positions), 1))
    if precision == "fp32":
        within = max(drift.values()) < TOLERANCE
        bar = f"tolerance {TOLERANCE}"
    else:
        out.update(reduced)
        within = all(out[k] < limit for k, limit in REDUCED.items())
        bar = f"{precision}: " + ", ".join(f"{k} {v}" for k, v in REDUCED.items())
    ok = within and out["same_argmax"] >= SAME_ARGMAX
    log(f"serve: {served.name} references {'pass' if ok else 'FAIL'} ({bar}): {out}")
    if not ok:
        raise SystemExit(f"{served.dir}: the served model is not its references ({bar}, argmax {SAME_ARGMAX}): {out}")
    return out


def softmax(z: np.ndarray) -> np.ndarray:
    e = np.exp(z.astype(np.float64) - z.max())
    return e / e.sum()


def raise_priority(log=print):
    """Above the lanes that feed it: self-play runs more lanes than cores, and a server starved of the CPU starves
    every lane. Where the OS does not allow it (an unprivileged nice on Linux), the server runs at normal priority."""
    try:
        if os.name == "nt":
            import ctypes  # noqa: PLC0415

            kernel32 = ctypes.windll.kernel32
            kernel32.GetCurrentProcess.restype = ctypes.c_void_p
            kernel32.SetPriorityClass.argtypes = [ctypes.c_void_p, ctypes.c_uint32]
            if not kernel32.SetPriorityClass(kernel32.GetCurrentProcess(), 0x80):  # HIGH_PRIORITY_CLASS
                raise OSError("SetPriorityClass failed")
        else:
            os.nice(-5)
        return True
    except (OSError, AttributeError) as e:
        log(f"serve: running at normal priority ({e})")
        return False


def fine_timer():
    """Windows rounds a timed wait up to its timer's 15.6 ms unless a process asks for 1 ms; the flush timeout is a
    few milliseconds."""
    if os.name == "nt":
        try:
            import ctypes  # noqa: PLC0415

            ctypes.windll.winmm.timeBeginPeriod(1)
        except (OSError, AttributeError):
            pass


def parse_batch(text: str, device: torch.device) -> dict[int, int]:
    if not text:
        # Measured on an RTX 4080 (docs/ai-training.md): rows a second level off at these sizes, so a larger batch only
        # pads more when the queue is short.
        return {32: 256, 64: 128, 128: 64} if device.type == "cuda" else {t: 32 for t in BUCKETS}
    if ":" not in text:
        return {t: int(text) for t in BUCKETS}
    out = {}
    for part in text.split(","):
        t, n = part.split(":")
        out[int(t)] = int(n)
    missing = [t for t in BUCKETS if t not in out]
    if missing:
        raise SystemExit(f"--batch names no size for buckets {missing}")
    return out


def main(argv=None):
    a = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    a.add_argument("--model", action="append", required=True, help="a model directory (repeatable)")
    a.add_argument("--host", default="127.0.0.1")
    a.add_argument("--port", type=int, default=DEFAULT_PORT, help="0 picks a free port (see --ready-file)")
    a.add_argument("--device", default="auto", help="auto, cpu, cuda (HIP on ROCm), cuda:1, ...")
    a.add_argument("--batch", default="", help="rows a GPU call is padded to: N, or 32:N,64:N,128:N (default 32:256,64:128,128:64; 32 on the CPU)")
    a.add_argument("--flush-ms", type=float, default=2.0, help="how long a partial batch waits for more rows")
    a.add_argument("--pipelines", type=int, default=2, help="batches assembled and run at once")
    a.add_argument("--precision", choices=["fp32", "tf32", "fp16", "bf16"], default="fp32",
                   help="fp16 and bf16 run under autocast; below fp32 the references are held to REDUCED, and a "
                        "precision that fails them is refused")
    a.add_argument("--cuda-graphs", choices=["on", "off"], default="on")
    a.add_argument("--threads", type=int, default=2, help="torch's CPU threads")
    a.add_argument("--no-check", action="store_true", help="skip running the references at start")
    a.add_argument("--ready-file", help="write HOST:PORT here once listening")
    a.add_argument("--priority", choices=["high", "normal"], default="high",
                   help="the server's CPU priority: high, above the lanes it serves, where the OS allows it")
    a.add_argument("--switch-ms", type=float, default=0.2, help="Python's thread switch interval")
    a.add_argument("--log-every", type=float, default=60.0, help="seconds between the throughput lines")
    args = a.parse_args(argv)

    torch.set_num_threads(args.threads)
    deterministic(args.precision)
    if args.priority == "high":
        raise_priority()
    fine_timer()
    # The reading thread and the workers share the GIL; the default 5 ms switch interval would hold a request that
    # arrived while a worker assembles a batch for whole milliseconds.
    sys.setswitchinterval(args.switch_ms / 1000.0)
    device = choose_device(args.device)
    batch = parse_batch(args.batch, device)
    started = time.time()
    models = [Served(Path(d), device) for d in args.model]
    if len({m.identity for m in models}) != len(models):
        raise SystemExit("the same model twice")
    graphs = args.cuda_graphs == "on" and device.type == "cuda"
    gpu = f" ({torch.cuda.get_device_name(device)})" if device.type == "cuda" else ""
    server = Server(models, device, batch, args.flush_ms / 1000.0, max(1, args.pipelines), graphs,
                    half=HALF.get(args.precision))
    server.log_every = args.log_every
    for m in models:
        print(f"serve: {m.name} ({m.dir}), {m.parameters} parameters", flush=True)
        if not args.no_check:
            check(m, server.runners[0], precision=args.precision)
    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    if os.name != "nt":
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind((args.host, args.port))
    listener.listen(1024)
    host, port = listener.getsockname()[:2]
    print(f"serve: listening on {host}:{port}, {device}{gpu}, batch {batch}, flush {args.flush_ms} ms, "
          f"{args.pipelines} pipelines, {args.precision}, CUDA graphs {'on' if graphs else 'off'}; ready in "
          f"{time.time() - started:.1f} s", flush=True)

    def ready():
        if args.ready_file:
            tmp = Path(args.ready_file + ".tmp")
            tmp.write_text(f"{host}:{port}")
            os.replace(tmp, args.ready_file)

    try:
        server.serve(listener, ready)
    except KeyboardInterrupt:
        pass
    finally:
        print(server.summary(), flush=True)


if __name__ == "__main__":
    main()
