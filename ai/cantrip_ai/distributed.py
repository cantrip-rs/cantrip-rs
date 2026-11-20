"""Distributed self-play (docs/ai-training.md, "Distributed self-play"): one generation's self-play stage played by this
machine and by worker machines over SSH, the driver (`generation.py --workers FILE`) the coordinator.

The work split. Game i of a generation is dealt from `seed + i`, and its pairing, seats and first player come from i
alone (`cantrip_ai_host::selfplay::schedule`), so a game is the same game wherever it is played; a different GPU can
round fp16 differently, so it replays exactly only on the hardware that played it, which `HOSTS.json` records. Every
self-play process plays only the indices its grant file grants (`cantrip-selfplay --grant`), as the file grows, until
it says `end`. The coordinator keeps the bookkeeping in `genN/dist/ledger.json` (`Ledger`), and three rules make every
game index 0..N-1 played into the generation's self-play exactly once:

1. An index is granted only while it is pending: not done (a game of it accepted), not a runaway, not dropped, and
   owned by no attempt (one self-play process's run). So at any moment at most one attempt owns an index.
2. A shard is accepted only if its header is the one this self-play's binary and model write (`--print-header`, asked
   before any game) and every game in it is of an index its attempt owns and no shard holds yet; it is moved into
   `genN/selfplay` first and then recorded, so an accepted shard is always there. Any other shard is rejected whole
   (moved aside, never trained on). So no index is accepted twice.
3. An attempt's indices go back to pending only when the attempt is over: it stopped (exit 3) or finished (exit 0) and
   its closed shards were pulled, or it crashed and its `.part`s were recovered (`cantrip-selfplay --recover`) and
   pulled, or it was given up (its host unreachable past `--worker-timeout`, its recovery failing that long, or its
   launch never started a process): anything it delivers later is rejected by rule 2. A grant never takes an index back
   from a live process. So every index stays owned until it is done or released, and the loop ends only when every
   index is done, a runaway (`runaways.jsonl`, as a single run counts one), or dropped: a game that failed on two
   attempts (a finished attempt's games missing from its shards go back to pending the first time).

Grants are handed out on demand: each live attempt is kept a backlog of about its lanes plus `--dist-lookahead` hours
of its measured rate, in chunks of `--dist-chunk`, lowest index first, so a fast host never waits on a slow one for
more than its backlog. The ledger is written before a grant file, every grant file is written whole, and each poll
compares a worker's grant file with the ledger's and writes it again when they differ, so a write that failed is
made good. A coordinator restart picks up where it was: it re-reads the ledger (refusing one made with other settings,
`identity`), checks that every accepted shard is still there, adopts shards it had not recorded, and re-attaches to
every attempt still running, local or remote. Each attempt holds a lease (`--lease`): a heartbeat file the coordinator
touches at each poll; once it is older than `--dist-lease`, the attempt's self-play stops gracefully, so a coordinator
that went away leaves no run playing on.

The hosts: `LocalHost` runs `cantrip-selfplay` here, in `genN/selfplay`, as the driver always has; `SshHost` pushes a
bundle to a worker and runs `cantrip-infer` and `cantrip-selfplay` there, detached from the SSH session
(`Win32_Process.Create`, so the session's job object does not take them down), in
`ROOT_REMOTE/gens/<root tag>/genN/<host>/<attempt>-<nonce>`. A process is taken for alive, or killed, only when its id,
start time and image all match what its attempt recorded. The coordinator talks to a host only through its methods
(`prepare`, `launch`, `grant`, `poll`, `fetch`, `recover`, `stop`, `kill`), so tests drive it with fakes.
"""

from __future__ import annotations

import base64
import contextlib
import gzip
import hashlib
import inspect
import io
import json
import math
import os
import secrets
import shutil
import signal
import socket
import subprocess
import sys
import tarfile
import time
from dataclasses import dataclass, field
from pathlib import Path

# The stop file a self-play run looks for in its directory (`cantrip_ai_host::selfplay::STOP_FILE`).
STOP_FILE = "selfplay.stop"
# Its status file (`cantrip_ai_host::selfplay::STATUS`).
STATUS = "selfplay-status.json"
RUNAWAYS = "runaways.jsonl"
# Which machine played which game: the shards' hosts, in the generation's self-play directory.
HOSTS = "HOSTS.json"
SELFPLAY_STOPPED = 3
# The CUDA libraries cantrip-infer's onnxruntime CUDA provider loads (the static imports of
# onnxruntime_providers_cuda.dll, and the cuDNN parts cudnn64_9 loads for this network: read from a running server's
# modules, and run on a worker), out of a PyTorch install's torch/lib (~1.2 GB of its ~4 GB).
CUDA_DLLS = ("cudart64_12.dll", "cublas64_12.dll", "cublasLt64_12.dll", "cufft64_11.dll", "cudnn64_9.dll",
             "cudnn_graph64_9.dll", "cudnn_ops64_9.dll")
ORT_DLLS = ("onnxruntime.dll", "onnxruntime_providers_shared.dll", "onnxruntime_providers_cuda.dll")
# A worker's cantrip-infer, unless the worker or `--worker-server-args` says otherwise: the native server's settings
# self-play runs at (its own defaults are fp32 on two pipelines).
WORKER_SERVER_ARGS = "--precision fp16 --pipelines 1"
# A launch whose process has not shown itself (no pids.json, no recorded process) after this long never started.
LAUNCH_GRACE = 180.0


class Unreachable(Exception):
    """A host could not be reached (or a command on it failed): the attempt is retried, and given up past the timeout."""


# --- Index ranges -------------------------------------------------------------------------------------------------


def to_ranges(indices) -> list[list[int]]:
    """Sorted, distinct indices as half-open ranges `[a, b]`."""
    out: list[list[int]] = []
    for i in sorted(set(indices)):
        if out and out[-1][1] == i:
            out[-1][1] = i + 1
        else:
            out.append([i, i + 1])
    return out


def from_ranges(ranges) -> list[int]:
    return [i for a, b in ranges for i in range(a, b)]


def grant_text(ranges, ended: bool) -> str:
    """A grant file (`cantrip_ai_host::selfplay::parse_grant`): a line a range, `end` when no more will come."""
    lines = [f"{a}..{b}" if b - a > 1 else f"{a}" for a, b in ranges]
    return "".join(line + "\n" for line in lines) + ("end\n" if ended else "")


def attempt_grant(a: dict) -> str:
    return grant_text(a["granted"], a["grant_ended"])


# --- The ledger ---------------------------------------------------------------------------------------------------

LIVE = ("launching", "running", "stopping")
OVER = ("finished", "stopped", "crashed", "lost", "failed")
# Over in a way that counts against its host (`Coordinator.failures`).
FAILED = ("crashed", "lost", "failed")
# How many attempts may fail a game (finish without it in their shards) before it is accounted dropped.
DROP_AFTER = 2


def identity_conflicts(old: dict, new: dict) -> list[str]:
    """What differs between two runs' settings (`Ledger.identity`): every key of `global`, and each host the two
    share (a host only one names may join or leave)."""
    out = [f"{k}: {old.get('global', {}).get(k)!r} then, {new.get('global', {}).get(k)!r} now"
           for k in sorted(set(old.get("global", {})) | set(new.get("global", {})))
           if old.get("global", {}).get(k) != new.get("global", {}).get(k)]
    for h in sorted(set(old.get("hosts", {})) & set(new.get("hosts", {}))):
        o, n = old["hosts"][h], new["hosts"][h]
        out += [f"{h}.{k}: {o.get(k)!r} then, {n.get(k)!r} now" for k in sorted(set(o) | set(n)) if o.get(k) != n.get(k)]
    return out


class Ledger:
    """The generation's distributed bookkeeping (`genN/dist/ledger.json`): the run's identity and the header its shards
    must carry, its attempts and what each was granted, the shards accepted and the indices they hold, runaways,
    failed and dropped games, and the shards rejected. `owner` (an index to the attempt that owns it) is derived on
    load: an attempt not released owns what it was granted that is not yet accounted for."""

    def __init__(self, path: Path | None, games: int, seed: int):
        self.path, self.games, self.seed = path, games, seed
        self.attempts: dict[str, dict] = {}
        self.shards: dict[str, dict] = {}
        self.done: dict[int, str] = {}
        self.runaways: dict[int, str] = {}
        self.dropped: dict[int, str] = {}
        self.fails: dict[int, int] = {}
        self.rejected: list[dict] = []
        self.identity: dict = {}
        self.header: dict | None = None
        self.owner: dict[int, str] = {}

    @classmethod
    def load(cls, path: Path, games: int, seed: int, identity: dict | None = None, header: dict | None = None) -> "Ledger":
        """The ledger at `path` (a new one if there is none), held to `identity` and `header`: a ledger made with other
        settings is refused, a new host's identity joins."""
        led = cls(path, games, seed)
        if path.exists():
            d = json.loads(path.read_text(encoding="utf-8"))
            if d["games"] != games or d["seed"] != seed:
                raise SystemExit(f"{path} is for {d['games']} games from seed {d['seed']}, not {games} from {seed}: "
                                 "another self-play; remove it (and the self-play directory) to start again")
            led.attempts = d["attempts"]
            led.shards = d["shards"]
            led.rejected = d.get("rejected", [])
            led.identity = d.get("identity") or {}
            led.header = d.get("header")
            led.runaways = {int(k): v for k, v in d.get("runaways", {}).items()}
            led.dropped = {int(k): v for k, v in d.get("dropped", {}).items()}
            led.fails = {int(k): v for k, v in d.get("fails", {}).items()}
            for name, s in led.shards.items():
                for i in from_ranges(s["indices"]):
                    led.done[i] = name
            led.derive_owners()
        if identity is not None:
            if led.identity and (conflicts := identity_conflicts(led.identity, identity)):
                raise SystemExit(f"{path} was made with other settings; this self-play cannot go on with these ("
                                 + "; ".join(conflicts) + "). Run it with the settings it began with, or remove its "
                                 "self-play directory and dist/ to start it again")
            merged = {"global": identity.get("global", {}), "hosts": {**led.identity.get("hosts", {}), **identity.get("hosts", {})}}
            led.identity = merged
        if header is not None:
            if led.header and led.header != header:
                raise SystemExit(f"{path}: this binary and model write another header ({header}) than the shards "
                                 f"accepted so far ({led.header}): rebuild as before, or start the self-play again")
            led.header = header
        return led

    def derive_owners(self):
        self.owner = {}
        for a in self.attempts.values():
            if not a.get("released"):
                for i in from_ranges(a["granted"]):
                    if not self.accounted(i):
                        self.owner[i] = a["id"]

    def save(self):
        if self.path is None:
            return
        self.path.parent.mkdir(parents=True, exist_ok=True)
        d = dict(games=self.games, seed=self.seed, identity=self.identity, header=self.header, attempts=self.attempts,
                 shards=self.shards, rejected=self.rejected,
                 runaways={str(k): v for k, v in sorted(self.runaways.items())},
                 dropped={str(k): v for k, v in sorted(self.dropped.items())},
                 fails={str(k): v for k, v in sorted(self.fails.items())})
        tmp = self.path.with_suffix(".tmp")
        tmp.write_text(json.dumps(d, indent=1), encoding="utf-8")
        os.replace(tmp, self.path)

    def verify(self, selfplay: Path, incoming: Path | None = None, elsewhere=(), log=print) -> list[str]:
        """Every accepted shard is where it was put (or in one of `elsewhere`, the held-out set once chosen): one still
        in `incoming` (moved by a coordinator that stopped before it moved it in) is moved in, and one that is nowhere is
        un-accepted, its games going back to their attempt if it is still live, else to pending. Returns the names
        un-accepted."""
        gone = []
        for name in list(self.shards):
            if (selfplay / name).exists() or any((Path(d) / name).exists() for d in elsewhere):
                continue
            pulled = incoming / f"{name}.pulling" if incoming else None
            if pulled is not None and pulled.exists():
                os.replace(pulled, selfplay / name)
                continue
            s = self.shards.pop(name)
            for i in from_ranges(s["indices"]):
                self.done.pop(i, None)
            gone.append(name)
            log(f"  {name} was accepted but is not in {selfplay}: un-accepted, its games played again")
        if gone:
            self.derive_owners()
        return gone

    def accounted(self, i: int) -> bool:
        return i in self.done or i in self.runaways or i in self.dropped

    def complete(self) -> bool:
        return len(self.done) + len(self.runaways) + len(self.dropped) == self.games

    def pending(self) -> list[int]:
        """The indices no one has: not accounted for and owned by no attempt, lowest first."""
        return [i for i in range(self.games) if i not in self.owner and not self.accounted(i)]

    def new_attempt(self, host: str, started: float | None = None) -> dict:
        k = 1 + sum(1 for a in self.attempts.values() if a["host"] == host)
        a = dict(id=f"{host}-a{k}", host=host, k=k, nonce=secrets.token_hex(4), state="launching", released=False,
                 granted=[], granted_count=0, grant_ended=False, pulled=[], rejected_shards=0,
                 started=time.time() if started is None else started)
        self.attempts[a["id"]] = a
        return a

    def grant(self, attempt: dict, indices: list[int]):
        """Gives `attempt` the indices, every one pending (rule 1)."""
        for i in indices:
            if i in self.owner or self.accounted(i):
                raise AssertionError(f"index {i} granted twice")
            self.owner[i] = attempt["id"]
        attempt["granted"] = to_ranges(from_ranges(attempt["granted"]) + list(indices))
        attempt["granted_count"] += len(indices)

    def owned(self, attempt_id: str) -> list[int]:
        return sorted(i for i, a in self.owner.items() if a == attempt_id)

    def check(self, header: dict) -> str | None:
        """Whether a shard's header is this self-play's (`header`, asked of the binary and the model before any game):
        why not, or None."""
        want_seeds = f"{self.seed}..{self.seed + self.games}"
        if header.get("seeds") != want_seeds:
            return f"seeds {header.get('seeds')}, not {want_seeds}"
        if self.header is None:
            return "no expected header to hold it to"
        for k, v in self.header.items():
            if header.get(k) != v:
                return f"{k} {header.get(k)!r}, not {v!r}"
        return None

    def acceptable(self, attempt_id: str | None, name: str, indices: list[int]) -> str | None:
        """Why a shard of `attempt_id`'s (or of none: a shard there before distribution, its indices no one's) cannot
        be accepted (rule 2), or None."""
        if name in self.shards:
            return f"a shard named {name} is accepted already"
        if len(set(indices)) != len(indices):
            return "an index twice in the shard"
        for i in indices:
            if not 0 <= i < self.games:
                return f"index {i} is not in 0..{self.games}"
            if self.accounted(i):
                return f"index {i} is accounted for already ({self.done.get(i) or 'a runaway or dropped'})"
            if self.owner.get(i) != attempt_id:
                return f"index {i} is {'owned by ' + self.owner[i] if i in self.owner else 'owned by no attempt'}, not {attempt_id}"
        return None

    def accept(self, attempt_id: str | None, name: str, indices: list[int], info: dict) -> str | None:
        """Accepts a shard (rule 2), or says why not."""
        if why := self.acceptable(attempt_id, name, indices):
            return why
        for i in indices:
            self.done[i] = name
            self.owner.pop(i, None)
        self.shards[name] = dict(info, attempt=attempt_id, indices=to_ranges(indices))
        return None

    def reject(self, attempt_id: str | None, host: str | None, name: str, why: str):
        self.rejected.append(dict(attempt=attempt_id, host=host, name=name, why=why, at=time.time()))
        if attempt_id in self.attempts:
            self.attempts[attempt_id]["rejected_shards"] += 1

    def runaway(self, attempt_id: str, i: int):
        """A game its attempt ended as a runaway: accounted for, never played again (as a single run's resume)."""
        if self.owner.get(i) == attempt_id:
            del self.owner[i]
            self.runaways[i] = attempt_id

    def release(self, attempt: dict, failed: bool = False) -> list[int]:
        """The attempt is over (rule 3): what it owns goes back to pending. With `failed` (it finished, so it took every
        game it owns and they are not in its shards: they failed), each counts a failure, and a game that has failed
        `DROP_AFTER` times is accounted dropped, as a single run drops it."""
        left = self.owned(attempt["id"])
        for i in left:
            del self.owner[i]
            if failed:
                self.fails[i] = self.fails.get(i, 0) + 1
                if self.fails[i] >= DROP_AFTER:
                    self.dropped[i] = attempt["id"]
        attempt["released"] = True
        attempt["ended"] = attempt.get("ended") or time.time()
        return left


class LedgerLock:
    """`genN/dist/ledger.lock`: one driver at a time on a generation's ledger (its pid and the process's start time; a
    lock whose process is gone is taken over)."""

    def __init__(self, path: Path):
        self.path = path

    def __enter__(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        me = dict(pid=os.getpid(), start=process_start(os.getpid()))
        for _ in range(3):
            try:
                with open(self.path, "x", encoding="utf-8") as f:
                    f.write(json.dumps(me))
                return self
            except FileExistsError:
                held = read_json(self.path)
                if held and process_matches(held.get("pid"), held.get("start")):
                    raise SystemExit(f"{self.path}: another driver (pid {held.get('pid')}) is running this generation's "
                                     "self-play; wait for it, or remove the lock if it is gone")
                self.path.unlink(missing_ok=True)
        raise SystemExit(f"{self.path}: the lock could not be taken")

    def __exit__(self, *_):
        self.path.unlink(missing_ok=True)


# --- Shards -------------------------------------------------------------------------------------------------------


def read_shard(path: Path, seed: int, pairings: int) -> tuple[dict, list[int], dict]:
    """A shard read whole (it must read to its end): its header, its games' indices (seed - `seed`), and what the
    stage's numbers count of it."""
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    import shards  # noqa: PLC0415 (ai/shards.py)

    header, games = shards.scan(path)
    indices, info = [], dict(games=0, samples=0, visit_samples=0, capped=0, decided=0, first_wins=0, turns=0, wins={})
    try:
        for g in games:
            m = g.meta
            i = int(m["seed"]) - seed
            indices.append(i)
            info["games"] += 1
            info["samples"] += g.count
            info["visit_samples"] += sum(1 for s in g.samples if len(s.visits) > 1)
            info["turns"] += int(m.get("turns") or 0)
            capped = m.get("ended") == "cap"
            info["capped"] += capped
            decks = m.get("decks") or m.get("names") or []
            w = m.get("winner")
            if w and not capped and 0 < int(w) <= len(decks):
                info["decided"] += 1
                info["wins"][decks[int(w) - 1]] = info["wins"].get(decks[int(w) - 1], 0) + 1
                first = (i // (2 * max(1, pairings))) % 2
                info["first_wins"] += int(w) - 1 == first
    finally:
        games.close()
    return header, indices, info


# --- The coordinator ----------------------------------------------------------------------------------------------


@dataclass
class Poll:
    """What a host says of an attempt: its status file, its exit code once it exited, whether it lives (its supervisor
    or its self-play, each matched by start time and image), its closed shards' names, its runaways' indices, its report
    once it exited, whether a launch got as far as starting it (None: not known), and its grant file as it is there."""
    status: dict | None = None
    exit: int | None = None
    alive: bool = True
    shards: list[str] = field(default_factory=list)
    runaways: list[int] = field(default_factory=list)
    report: dict | None = None
    started: bool | None = None
    grant: str | None = None


class Coordinator:
    """Runs a generation's self-play across `hosts` to the end (see the module): `run()` returns "done" or
    "stopped". Timeouts are on `monotonic`; how long a host has been unreachable is this run's alone."""

    def __init__(self, ledger: Ledger, hosts: list, selfplay_dir: Path, dist_dir: Path, read=None, log=print,
                 clock=time.time, sleep=time.sleep, monotonic=time.monotonic, chunk: int = 25, lookahead: float = 0.05,
                 initial: int = 16, poll_seconds: float = 30.0, timeout: float = 600.0, max_failures: int = 3,
                 report_every: float = 300.0, stop_file: Path | None = None, retry_seconds: float = 300.0,
                 heldout: Path | None = None, drop_limit: int = 3, drop_share: float = 0.2):
        self.ledger, self.hosts, self.dir, self.dist = ledger, {h.name: h for h in hosts}, selfplay_dir, dist_dir
        self.read = read
        self.log, self.clock, self.sleep, self.monotonic = log, clock, sleep, monotonic
        self.chunk, self.lookahead, self.initial = max(1, chunk), lookahead, max(1, initial)
        self.poll_seconds, self.timeout, self.max_failures = poll_seconds, timeout, max_failures
        self.report_every, self.retry_seconds = report_every, retry_seconds
        self.drop_limit, self.drop_share = drop_limit, drop_share
        self.stop_file = stop_file or selfplay_dir / STOP_FILE
        self.heldout = heldout
        self.stopping = False
        self.stopped_now = False
        self.interrupts = 0
        self.backoff: dict[str, float] = {}
        self.unreachable: dict[str, float] = {}
        self.recovering: dict[str, float] = {}
        self.prepared: set[str] = set()
        self.reported = -1e18
        self.started = monotonic()

    # The attempts.

    def live(self, host: str | None = None) -> list[dict]:
        return [a for a in self.ledger.attempts.values() if a["state"] in LIVE and (host is None or a["host"] == host)]

    def unconfirmed(self, host: str) -> list[dict]:
        """Attempts on `host` given up while their process may still run: no new one starts there until they are seen
        dead (two servers never share a GPU)."""
        return [a for a in self.ledger.attempts.values() if a["host"] == host and a["state"] == "lost" and not a.get("lost_over")]

    def failures(self, host: str) -> int:
        """What counts against a host this generation: its attempts that crashed, were lost or failed, those stopped for
        dropping games, and its shards rejected."""
        led = self.ledger
        return (sum(1 for a in led.attempts.values() if a["host"] == host and (a["state"] in FAILED or a.get("dropping")))
                + sum(1 for r in led.rejected if r.get("host") == host))

    # Shards.

    def take(self, attempt: dict | None, host, name: str) -> bool:
        """A shard of `attempt` (None: one in the self-play directory the ledger does not hold) by its name on its host:
        fetched, read, checked, moved into the self-play directory and then accepted, or rejected and moved aside
        (rule 2). False when it could not be fetched: tried again at the next poll."""
        local = host is None or not host.remote
        final = name if local else f"{attempt['host']}-{name}"
        if attempt is not None and name in attempt["pulled"]:
            return True
        if final in self.ledger.shards:
            if attempt is not None:
                attempt["pulled"].append(name)
            return True
        if local:
            path = self.dir / name
        else:
            incoming = self.dist / "incoming"
            incoming.mkdir(parents=True, exist_ok=True)
            path = incoming / f"{final}.pulling"
            try:
                host.fetch(attempt, name, path)
            except Unreachable as e:
                self.log(f"  [{attempt['host']}] {name} not fetched ({e}); tried again at the next poll")
                path.unlink(missing_ok=True)
                return False
        try:
            header, indices, info = self.read(path)
            why = self.ledger.check(header)
        except Exception as e:  # noqa: BLE001 (a shard that does not read is rejected, whatever the error)
            indices, info, why = [], {}, f"does not read: {e}"
        who = attempt["id"] if attempt is not None else self.owner_of(indices)
        owner = self.ledger.attempts.get(who or "")
        host_name = attempt["host"] if attempt is not None else (owner["host"] if owner else "local")
        if why is None:
            why = self.ledger.acceptable(who, final, indices)
        if why is None:
            # In first, recorded after: an accepted shard is always where training reads it (`Ledger.verify`).
            if not local:
                os.replace(path, self.dir / final)
            info["host"] = host_name
            if owner:
                info.update({k: owner.get(k) for k in ("gpu", "server_args", "precision") if owner.get(k)})
            self.ledger.accept(who, final, indices, info)
            self.log(f"  [{host_name}] accepted {final}: {len(indices)} games")
        else:
            self.ledger.reject(who if owner else None, host_name if owner or attempt else None, final, why)
            aside = self.dist / "rejected"
            aside.mkdir(parents=True, exist_ok=True)
            if path.exists():
                os.replace(path, aside / final)
            self.log(f"  [{host_name}] rejected {final} ({why}): moved to {aside}, never trained on")
        if attempt is not None:
            attempt["pulled"].append(name)
            if not local:
                attempt.setdefault("to_delete", []).append(name)
        self.ledger.save()
        return True

    def owner_of(self, indices: list[int]) -> str | None:
        """The one attempt that owns all of `indices`, or None (none owns any)."""
        owners = {self.ledger.owner.get(i) for i in indices}
        if not owners:
            return None
        return owners.pop() if len(owners) == 1 else "(several)"

    def adopt(self):
        """Shards in the self-play directory the ledger does not hold: the local attempts' (accepted by their owner),
        or a shard there before distribution (owned by no one), or one a coordinator moved in and stopped before
        recording."""
        for p in sorted(self.dir.glob("*.shard.zst")):
            if p.name not in self.ledger.shards:
                self.take(None, None, p.name)

    # One attempt.

    def give_up(self, a: dict, why: str):
        """Over, as lost: its games to the others, anything it delivers later rejected (rule 3)."""
        left = self.ledger.release(a)
        a["state"] = "lost"
        a["why"] = why
        self.log(f"  [{a['host']}] {a['id']} given up ({why}); its {len(left)} unplayed games go to the others, and "
                 "anything it sends later is rejected")

    def check(self, a: dict):
        host = self.hosts.get(a["host"])
        if host is None:
            return
        try:
            p = host.poll(a)
        except Unreachable as e:
            now = self.monotonic()
            since = self.unreachable.setdefault(a["id"], now)
            if now - since > self.timeout and a["state"] in LIVE:
                self.give_up(a, f"unreachable for {now - since:.0f} s: {e}")
            self.ledger.save()
            return
        self.unreachable.pop(a["id"], None)
        a.pop("to_delete", None)  # deleted by that poll
        if not host.remote:
            # After the poll: a local run that exited closed its shards before its exit code was read.
            self.adopt()
        if a["state"] == "lost":
            # Given up while it may have run on: stopped for good, and seen dead before its host gets another.
            if p.alive and p.exit is None:
                self.attempt(lambda: host.kill(a), a, "kill it")
            else:
                a["lost_over"] = True
            self.ledger.save()
            return
        if p.started is not True and not p.alive and p.exit is None:
            # A launch not seen to start: given its grace (its process shows itself within seconds), then, if nothing
            # was started (no job.json), failed and released; if something may have been, recovered as a crash.
            if self.clock() - a["started"] <= LAUNCH_GRACE:
                self.ledger.save()
                return
            if p.started is False:
                self.ledger.release(a)
                a["state"] = "failed"
                a["why"] = "its launch never started a process"
                self.log(f"  [{a['host']}] {a['id']} never started; its games go back to the rest")
                self.ledger.save()
                return
        if p.status:
            a["status"] = p.status
            if a["state"] == "launching":
                a["state"] = "running"
        # A grant file that is not the ledger's (a write that failed, or one interrupted): written again, whole.
        if p.grant is not None and p.exit is None and p.alive and p.grant != attempt_grant(a):
            self.write_grant(a)
        for name in p.shards:
            if not self.take(a, host, name):
                break
        for i in p.runaways:
            self.ledger.runaway(a["id"], i)
        if p.exit is None and p.alive:
            self.watch_drops(a, host, p)
            self.ledger.save()
            return
        self.over(a, host, p)
        self.ledger.save()

    def watch_drops(self, a: dict, host, p: Poll):
        """A run dropping games fast (its server gone, say) is stopped, counted against its host and fed no more."""
        s = p.status or {}
        dropped, played = int(s.get("Dropped") or 0), int(s.get("GamesThisRun") or 0)
        if not a.get("dropping") and dropped >= self.drop_limit and dropped > self.drop_share * (played + dropped):
            a["dropping"] = True
            self.log(f"  [{a['host']}] {a['id']} dropped {dropped} games against {played} played: stopping it")
            if self.attempt(lambda: host.stop(a, now=True), a, "stop it"):
                a["state"] = "stopping"

    def attempt(self, step, a: dict, what: str) -> bool:
        try:
            step()
            return True
        except Unreachable as e:
            self.log(f"  [{a['host']}] could not {what} for {a['id']} ({e}); trying again")
            return False

    def over(self, a: dict, host, p: Poll):
        a["exit"] = p.exit
        a["report"] = summary(p.report)
        if p.exit == 0:
            left = self.ledger.owned(a["id"])
            # Every granted game was handed out (the grant said end): those not in its shards failed. Each goes back to
            # the rest, and one that failed `DROP_AFTER` times is dropped, as a single run drops it.
            self.ledger.release(a, failed=not a["rejected_shards"])
            a["state"] = "finished"
            self.log(f"  [{a['host']}] {a['id']} finished" + (f"; {len(left)} games not in its shards go back" if left else ""))
        elif p.exit == SELFPLAY_STOPPED:
            left = self.ledger.release(a)
            a["state"] = "stopped"
            self.log(f"  [{a['host']}] {a['id']} stopped; {len(left)} unplayed games go back to the rest")
        else:
            # Crashed (or its supervisor did): its leftovers killed, its .parts recovered, their games taken.
            if a["id"] not in self.recovering:
                self.recovering[a["id"]] = self.monotonic()
                self.log(f"  [{a['host']}] {a['id']} crashed (exit {p.exit}): recovering its open shards")
            try:
                host.kill(a)
                host.recover(a)
                if not host.remote:
                    self.adopt()
                q = host.poll(a)
            except Unreachable as e:
                if self.monotonic() - self.recovering[a["id"]] > self.timeout:
                    self.give_up(a, f"its recovery failed for {self.timeout:.0f} s: {e}")
                else:
                    self.log(f"  [{a['host']}] {a['id']}: recovery failed ({e}); trying again")
                return
            for name in q.shards:
                if not self.take(a, host, name):
                    return
            for i in q.runaways:
                self.ledger.runaway(a["id"], i)
            left = self.ledger.release(a)
            a["state"] = "crashed"
            self.recovering.pop(a["id"], None)
            self.log(f"  [{a['host']}] {a['id']} recovered; {len(left)} unplayed games go back to the rest")
        if host.remote:
            host.fetch_logs(a, self.dist / "logs")
            # The shards it pulled since its last poll: deleted there now (no poll comes after its end).
            if a.get("to_delete") and hasattr(host, "delete"):
                with contextlib.suppress(Unreachable):
                    host.delete(a, a["to_delete"])
                    a.pop("to_delete")

    # Granting.

    def prepare(self):
        """Every worker made ready before any attempt starts (the bundle pushed, orphans of this root stopped, old
        bundles removed), so a long push never holds up the others' grants."""
        for name, host in self.hosts.items():
            if name in self.prepared or not hasattr(host, "prepare") or self.backoff.get(name, 0) > self.monotonic():
                continue
            live = [a for a in self.ledger.attempts.values() if a["host"] == name and (a["state"] in LIVE or a in self.unconfirmed(name))]
            try:
                host.prepare(live)
                self.prepared.add(name)
            except Unreachable as e:
                self.backoff[name] = self.monotonic() + self.retry_seconds
                self.log(f"  [{name}] not ready ({e}); trying again in {self.retry_seconds:.0f} s")

    def launch(self):
        """A new attempt on every host that has none, while games are pending."""
        if not self.ledger.pending():
            return
        self.prepare()
        for name, host in self.hosts.items():
            if self.live(name) or self.unconfirmed(name) or self.failures(name) >= self.max_failures:
                continue
            if self.backoff.get(name, 0) > self.monotonic() or (hasattr(host, "prepare") and name not in self.prepared):
                continue
            pend = self.ledger.pending()
            if not pend:
                return
            a = self.ledger.new_attempt(name, started=self.clock())
            self.ledger.grant(a, pend[:self.rounded(self.initial)])
            self.ledger.save()
            try:
                a.update(host.launch(a, attempt_grant(a)) or {})
                self.log(f"  [{name}] {a['id']} started with games {fmt_ranges(a['granted'])}")
            except Unreachable as e:
                # Never started (a launch that did start shows itself at the next poll, and is then taken up).
                self.backoff[name] = self.monotonic() + self.retry_seconds
                self.log(f"  [{name}] could not start ({e}); trying again in {self.retry_seconds:.0f} s")
            self.ledger.save()

    def rounded(self, n: float) -> int:
        return self.chunk * max(1, math.ceil(n / self.chunk))

    def top_up(self):
        """Each running attempt's backlog kept at its lanes and `lookahead` hours of its rate; `end` to all once nothing
        is pending. An attempt whose last grant was not written gets no more until it is."""
        for a in self.live():
            if a["grant_ended"] or a["state"] != "running" or a.get("grant_unwritten"):
                continue
            s = a.get("status") or {}
            g = s.get("Governor") or {}
            backlog = a["granted_count"] - int(s.get("Taken") or 0)
            want = (g.get("LanesAllowed") or 1) + float(s.get("GamesPerHour") or 0) * self.lookahead
            pend = self.ledger.pending()
            if backlog < want and pend:
                self.ledger.grant(a, pend[:self.rounded(want - backlog)])
                self.ledger.save()
                self.write_grant(a)
        if not self.ledger.pending():
            for a in self.live():
                if not a["grant_ended"] and a["state"] != "stopping":
                    a["grant_ended"] = True
                    self.ledger.save()
                    self.write_grant(a)

    def write_grant(self, a: dict):
        try:
            self.hosts[a["host"]].grant(a, attempt_grant(a))
            a.pop("grant_unwritten", None)
        except Unreachable as e:
            # Each poll compares the file with the ledger's and writes it again; past the timeout the attempt is lost.
            a["grant_unwritten"] = True
            self.log(f"  [{a['host']}] grant not written ({e}); written again at the next poll")

    # Stopping.

    def stop_all(self, now: bool):
        self.stopping = True
        for a in self.live():
            if self.attempt(lambda a=a: self.hosts[a["host"]].stop(a, now), a, "ask it to stop"):
                a["state"] = "stopping"
        self.ledger.save()

    def stop_requested(self) -> str | None:
        """The local stop file's word (`generation.py --stop`): '' or 'now', or None when there is none."""
        try:
            return self.stop_file.read_text(encoding="utf-8").strip()
        except FileNotFoundError:
            return None
        except OSError:
            return ""

    # Progress.

    def progress(self, force: bool = False):
        now = self.monotonic()
        if not force and now - self.reported < self.report_every:
            return
        self.reported = now
        led = self.ledger
        self.log(f"  self-play: {len(led.done)} of {led.games} games in, {len(led.runaways)} runaways, {len(led.dropped)} "
                 f"dropped, {len(led.pending())} not yet granted ({(now - self.started) / 60:.0f} min)")
        for name, h in host_rows(led).items():
            live = self.live(name)
            s = (live[0].get("status") if live else None) or {}
            g = s.get("Governor") or {}
            self.log(f"    {name}: {h['games']} games in" + (f", {s.get('GamesPerHour', 0):.0f} games/h now" if s else "")
                     + (f", {g.get('LanesAllowed')} of {g.get('LanesCeiling')} lanes ({g.get('Running')} running)" if g else "")
                     + (f", held by {g.get('Limiter')}" if g.get("Limiter") else "")
                     + (f"; {live[0]['id']} has {live[0]['granted_count'] - int(s.get('Taken') or 0)} granted to start"
                        if live else "; idle"))

    # The loop.

    def run(self) -> str:
        self.dir.mkdir(parents=True, exist_ok=True)
        self.ledger.verify(self.dir, self.dist / "incoming", [self.heldout] if self.heldout else (), log=self.log)
        self.ledger.save()
        self.adopt()
        while True:
            try:
                self.step()
                if self.stopping and not self.live():
                    self.progress(force=True)
                    return "stopped"
                if not self.stopping and self.ledger.complete() and not self.live():
                    self.progress(force=True)
                    return "done"
                if not self.stopping and not self.live() and not self.ledger.complete():
                    able = [n for n in self.hosts if self.failures(n) < self.max_failures]
                    if not able:
                        raise SystemExit("distributed self-play: every host has failed; nothing left to play the "
                                         f"{len(self.ledger.pending())} games pending (see the log above)")
                self.progress()
                self.sleep(self.poll_seconds)
            except KeyboardInterrupt:
                self.interrupts += 1
                if self.interrupts >= 3:
                    self.log("interrupted a third time: killing the local self-play (the workers were asked to stop "
                             "their games in flight)")
                    for a in self.live():
                        h = self.hosts[a["host"]]
                        if not h.remote:
                            h.kill(a)
                    raise
                self.log("interrupted: every host stops and closes its shards"
                         + (" (Ctrl-C again stops their games in flight)" if self.interrupts == 1 else
                            "; their games in flight are stopped (Ctrl-C once more kills the local one)"))
                self.stop_all(now=self.interrupts >= 2)

    def step(self):
        word = self.stop_requested()
        if word is not None and (not self.stopping or (word == "now" and not self.stopped_now)):
            self.log(f"  stop asked ({self.stop_file}{': now' if word == 'now' else ''}): every host stops")
            self.stopped_now = word == "now"
            self.stop_all(now=word == "now")
        for a in list(self.ledger.attempts.values()):
            if a["state"] in LIVE or (a["state"] == "lost" and not a.get("lost_over")):
                self.check(a)
        if self.stopping:
            # One whose stop did not reach it: asked again.
            if any(a["state"] != "stopping" for a in self.live()):
                self.stop_all(now=self.interrupts >= 2 or self.stopped_now)
            return
        self.adopt()
        self.launch()
        self.top_up()


def fmt_ranges(ranges) -> str:
    text = ", ".join(f"{a}..{b - 1}" if b - a > 1 else f"{a}" for a, b in ranges[:6])
    return text + (f" and {len(ranges) - 6} more ranges" if len(ranges) > 6 else "")


def summary(report: dict | None) -> dict | None:
    """What the ledger keeps of a self-play report."""
    if not report:
        return None
    g = report.get("Governor") or {}
    auto = g.get("Autoscale") or {}
    keep = ("Host", "Games", "GamesThisRun", "Dropped", "Runaways", "Errors", "NetworkFallbacks", "Seconds",
            "GamesPerHour", "SearchedPerSecond", "EvaluationsPerSecond", "Failures")
    out = {k: report.get(k) for k in keep}
    out.update(Lanes=auto.get("Lanes"), Limiter=auto.get("Limiter"), LanesRunning=g.get("LanesRunning"),
               Stopped=report.get("Stopped"))
    return out


def host_rows(led: Ledger) -> dict[str, dict]:
    """Each host's part: games accepted, its attempts, its games an hour (its games over its attempts' time), and the
    server settings and GPU it played with."""
    rows: dict[str, dict] = {}
    for s in led.shards.values():
        r = rows.setdefault(s.get("host") or "local", dict(games=0, attempts=0, seconds=0.0))
        r["games"] += s.get("games", 0)
    for a in led.attempts.values():
        r = rows.setdefault(a["host"], dict(games=0, attempts=0, seconds=0.0))
        r["attempts"] += 1
        st = a.get("status") or {}
        r["seconds"] += float(st.get("Seconds") or 0)
        g = st.get("Governor") or {}
        if g:
            r["lanes"], r["limiter"] = g.get("LanesAllowed"), g.get("Limiter")
        for k in ("gpu", "server_args", "precision"):
            if a.get(k):
                r[k] = a[k]
    for r in rows.values():
        r["games_per_hour"] = round(r["games"] * 3600 / r["seconds"], 1) if r["seconds"] else None
    return rows


def hosts_manifest(led: Ledger) -> dict:
    """`genN/selfplay/HOSTS.json`: which host (with its GPU and server settings) played each shard's games, for replays
    (a game replays exactly only on the hardware, and at the precision, that played it)."""
    by_shard = {}
    for name, s in sorted(led.shards.items()):
        a = led.attempts.get(s.get("attempt") or "", {}) or {}
        by_shard[name] = dict(host=s.get("host"), attempt=s.get("attempt"), games=s["indices"], gpu=a.get("gpu"),
                              server_args=a.get("server_args"), precision=a.get("precision"))
    return dict(note="Which machine played each shard's games: a game is dealt from its seed alone, but a different GPU "
                     "(or precision) can round differently, so it replays exactly only on the host and settings that "
                     f"played it. Indices are game numbers: seed {led.seed} + index.", seed=led.seed, games=led.games,
                hosts=host_rows(led), shards=by_shard)


def numbers(led: Ledger, simulations, seconds: float, pairings_count: int, decks_note: str | None) -> dict:
    """The self-play stage's numbers, from the shards accepted and the attempts' reports (`generation.json`)."""
    tot = dict(games=0, samples=0, visit_samples=0, capped=0, decided=0, first_wins=0, turns=0)
    wins: dict[str, int] = {}
    for s in led.shards.values():
        for k in tot:
            tot[k] += s.get(k, 0)
        for d, n in (s.get("wins") or {}).items():
            wins[d] = wins.get(d, 0) + n
    reports = [a.get("report") or {} for a in led.attempts.values()]
    errors: dict[str, int] = {}
    for r in reports:
        for k, n in (r.get("Errors") or {}).items():
            errors[k] = errors.get(k, 0) + n
    hosts = host_rows(led)
    out = dict(games=tot["games"], resumed=0, games_this_run=tot["games"], samples=tot["samples"],
               visit_samples=tot["visit_samples"], capped=tot["capped"], dropped=len(led.dropped),
               runaways=len(led.runaways), fallbacks=sum(r.get("NetworkFallbacks") or 0 for r in reports),
               simulations=simulations, lanes=sum(h.get("lanes") or 0 for h in hosts.values()),
               games_per_hour=tot["games"] * 3600 / max(seconds, 1e-9),
               first_player=tot["first_wins"] / tot["decided"] if tot["decided"] else 0.0,
               mean_turns=tot["turns"] / tot["games"] if tot["games"] else 0.0,
               seeds=f"{led.seed}..{led.seed + led.games}", producer=(led.header or {}).get("producer"),
               wins=wins, errors=errors, rejected=len(led.rejected), hosts=hosts,
               attempts={a["id"]: dict(state=a["state"], granted=a["granted_count"], exit=a.get("exit"),
                                       report=a.get("report")) for a in led.attempts.values()})
    if decks_note:
        out["decks"] = decks_note
    return out


# --- Processes ----------------------------------------------------------------------------------------------------


def process_start(pid: int | None) -> int | None:
    """A process's start time (Windows: its creation FILETIME; elsewhere its start in clock ticks since boot), or None
    when there is no such process."""
    if not pid:
        return None
    if os.name == "nt":
        import ctypes  # noqa: PLC0415
        from ctypes import wintypes  # noqa: PLC0415

        k = ctypes.windll.kernel32
        h = k.OpenProcess(0x1000, False, int(pid))  # PROCESS_QUERY_LIMITED_INFORMATION
        if not h:
            return None
        try:
            code = wintypes.DWORD()
            if not k.GetExitCodeProcess(h, ctypes.byref(code)) or code.value != 259:  # STILL_ACTIVE
                return None
            c, e, kt, u = (wintypes.FILETIME() for _ in range(4))
            if not k.GetProcessTimes(h, ctypes.byref(c), ctypes.byref(e), ctypes.byref(kt), ctypes.byref(u)):
                return None
            return (c.dwHighDateTime << 32) | c.dwLowDateTime
        finally:
            k.CloseHandle(h)
    try:
        return int(Path(f"/proc/{int(pid)}/stat").read_text().rsplit(")", 1)[1].split()[19])
    except (OSError, ValueError, IndexError):
        return None


def process_matches(pid: int | None, start: int | None) -> bool:
    """Whether process `pid` is alive and the one that started at `start` (a reused id is another process)."""
    now = process_start(pid)
    return now is not None and (start is None or now == start)


def write_atomic(path: Path, text: str):
    tmp = path.with_name(path.name + ".tmp")
    tmp.write_text(text, encoding="utf-8", newline="\n")
    os.replace(tmp, path)


def read_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return None


def touch(path: Path):
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a", encoding="utf-8"):
        pass
    os.utime(path, None)


class LocalHost:
    """This machine: `cantrip-selfplay` in the generation's self-play directory, as the driver always ran it, with a
    grant file (`genN/dist/<attempt>.grant`) and a lease (`genN/dist/<attempt>.heartbeat`, touched at each poll).
    `command` is the self-play command line without `--grant`; its shards land where the window reads them, so the
    coordinator adopts them in place. A process is known by its id and start time, so a coordinator that restarted
    takes up a run it did not start."""

    remote = False

    def __init__(self, name: str, command: list, selfplay_dir: Path, dist_dir: Path, logs: Path, env: dict | None = None,
                 cwd: Path | None = None, lease: float = 900.0, identity: dict | None = None):
        self.name, self.command, self.dir, self.dist, self.logs, self.env, self.cwd = name, command, selfplay_dir, dist_dir, logs, env, cwd
        self.lease, self.identity = lease, identity or {}
        self.processes: dict[str, subprocess.Popen] = {}
        self.files = {}

    def grant_path(self, a: dict) -> Path:
        return self.dist / f"{a['id']}.grant"

    def heartbeat(self, a: dict) -> Path:
        return self.dist / f"{a['id']}.heartbeat"

    def launch(self, a: dict, grant: str) -> dict:
        self.dist.mkdir(parents=True, exist_ok=True)
        self.logs.mkdir(parents=True, exist_ok=True)
        write_atomic(self.grant_path(a), grant)
        touch(self.heartbeat(a))
        # A stop file there now is a stop asked (`--stop`), not a stale one (the stage removes those as it starts):
        # the run sees it and plays nothing.
        log = self.logs / f"selfplay-{a['id']}.log"
        f = log.open("w")
        cmd = [str(c) for c in self.command] + ["--grant", str(self.grant_path(a)), "--host", self.name,
                                                "--lease", str(self.heartbeat(a)), "--lease-seconds", str(self.lease)]
        p = subprocess.Popen(cmd, cwd=self.cwd, stdout=f, stderr=subprocess.STDOUT, env={**os.environ, **(self.env or {})})
        self.processes[a["id"]], self.files[a["id"]] = p, f
        return dict(pids=dict(selfplay=dict(pid=p.pid, start=process_start(p.pid))), log=str(log), dir=str(self.dir),
                    **self.identity)

    def grant(self, a: dict, text: str):
        write_atomic(self.grant_path(a), text)

    def poll(self, a: dict) -> Poll:
        sp = (a.get("pids") or {}).get("selfplay") or {}
        if not sp:
            return Poll(alive=False, started=False)
        if self.heartbeat(a).exists() or a["state"] in LIVE:
            touch(self.heartbeat(a))
        p = self.processes.get(a["id"])
        # Its liveness first, then its files: a run that exited wrote them before it did.
        if p is not None:
            code = p.poll()
            alive = code is None
        else:
            alive, code = process_matches(sp.get("pid"), sp.get("start")), None
        status = read_json(self.dir / STATUS)
        if status and status.get("Pid") != sp.get("pid"):
            status = None
        if p is None and not alive:
            # Not ours (a coordinator restarted): its end by its last status.
            if status and status.get("Finished"):
                report = read_json(self.dir / "selfplay-report.json") or {}
                code = SELFPLAY_STOPPED if report.get("Stopped") else 0
        report = read_json(self.dir / "selfplay-report.json") if not alive else None
        if code is not None and p is not None and a["id"] in self.files:
            self.files.pop(a["id"]).close()
        try:
            grant = self.grant_path(a).read_text(encoding="utf-8")
        except OSError:
            grant = ""
        return Poll(status=status, exit=code, alive=alive, shards=[], runaways=runaway_indices(self.dir / RUNAWAYS),
                    report=report, started=True, grant=grant)

    def fetch(self, a, name, dest):
        raise AssertionError("a local shard is adopted in place")

    def recover(self, a: dict):
        cmd = [str(c) for c in self.command] + ["--recover"]
        self.logs.mkdir(parents=True, exist_ok=True)
        with (self.logs / f"recover-{a['id']}.log").open("w") as f:
            code = subprocess.call(cmd, cwd=self.cwd, stdout=f, stderr=subprocess.STDOUT, env={**os.environ, **(self.env or {})})
        if code != 0:
            raise Unreachable(f"cantrip-selfplay --recover exited {code}")

    def stop(self, a: dict, now: bool):
        (self.dir / STOP_FILE).write_text("now\n" if now else "", encoding="utf-8")

    def kill(self, a: dict):
        p = self.processes.get(a["id"])
        if p is not None:
            if p.poll() is None:
                p.kill()
                p.wait()
            if a["id"] in self.files:
                self.files.pop(a["id"]).close()
            return
        sp = (a.get("pids") or {}).get("selfplay") or {}
        if process_matches(sp.get("pid"), sp.get("start")):
            os.kill(int(sp["pid"]), signal.SIGTERM if os.name != "nt" else signal.SIGTERM)


def runaway_indices(path: Path) -> list[int]:
    out = []
    try:
        for line in path.read_text(encoding="utf-8").splitlines():
            if line.strip():
                try:
                    out.append(int(json.loads(line)["index"]))
                except (ValueError, KeyError, TypeError):
                    continue
    except OSError:
        pass
    return out


# --- Workers over SSH ---------------------------------------------------------------------------------------------

# Reads a script from stdin, up to its end line, and runs it: PowerShell's own `-Command -` runs stdin a line at a time,
# which breaks blocks; and reading to the end of the stream waits on an EOF Windows' OpenSSH does not always pass on (a
# launch hung for minutes on a worker that way).
_END = "#cantrip-end-of-script"
_BOOTSTRAP = ("$ErrorActionPreference='Stop'; $ProgressPreference='SilentlyContinue'; "
              "$b=New-Object Text.StringBuilder; while (($l=[Console]::In.ReadLine()) -ne $null -and $l -ne '" + _END + "') "
              "{ [void]$b.AppendLine($l) }; & ([scriptblock]::Create($b.ToString()))")
_ENCODED = base64.b64encode(_BOOTSTRAP.encode("utf-16-le")).decode()


def load_workers(path: Path) -> list[dict]:
    """The workers file (docs/ai-training.md, "Distributed self-play"): `{"key": PATH, "workers": [{"name", "host",
    "user", "root", ...}]}`; a worker's own `key` wins, `"enabled": false` leaves it out."""
    d = json.loads(Path(path).read_text(encoding="utf-8"))
    out = []
    for w in d.get("workers", []):
        if w.get("enabled", True) is False:
            continue
        for k in ("name", "host", "user", "root"):
            if not w.get(k):
                raise SystemExit(f"{path}: a worker without {k}: {w}")
        if w.get("os", "windows") not in ("windows", "linux"):
            raise SystemExit(f"{path}: a worker's os is windows or linux: {w}")
        w = dict(w)
        w.setdefault("key", d.get("key"))
        out.append(w)
    names = [w["name"] for w in out]
    if len(set(names)) != len(names):
        raise SystemExit(f"{path}: two workers share a name")
    return out


def worker_server_args(worker: dict, default: str) -> list[str]:
    """A worker's cantrip-infer arguments: its own `server_args`, else `default`. `--shapes` is refused: a batch run
    at a smaller shape makes a game depend on which games shared its batch, which self-play must not."""
    words = (worker.get("server_args") if worker.get("server_args") is not None else default).split()
    if any(w == "--shapes" or w.startswith("--shapes=") for w in words):
        raise SystemExit(f"worker {worker['name']}: --shapes in its server arguments: smaller batch shapes make a game "
                         "depend on its batch companions, which self-play does not allow")
    return words


def precision_of(words: list[str]) -> str:
    """The precision a cantrip-infer command line serves at (its default fp32)."""
    return words[words.index("--precision") + 1] if "--precision" in words[:-1] else "fp32"


def sha256(path: Path, cache: dict) -> str:
    st = path.stat()
    key = f"{path.resolve()}|{st.st_size}|{st.st_mtime_ns}"
    if key not in cache:
        h = hashlib.sha256()
        with path.open("rb") as f:
            while chunk := f.read(1 << 22):
                h.update(chunk)
        cache[key] = h.hexdigest()
    return cache[key]


def files_hash(paths, cache: dict | None = None) -> str:
    """One hash of several files' names and contents (the decks, a model's graphs)."""
    cache = {} if cache is None else cache
    h = hashlib.sha256()
    for p in sorted(Path(p) for p in paths):
        h.update(f"{p.name} {sha256(p, cache)}\n".encode())
    return h.hexdigest()[:16]


def bundle_files(selfplay: Path, infer: Path, ort_dir: Path, cuda_dir: Path, model: Path, decks: Path | None,
                 prior: Path | None, pairings: Path | None, cuda_dlls=CUDA_DLLS) -> dict[str, Path]:
    """What a worker needs, by its path in the bundle: the two binaries and the DLLs beside cantrip-selfplay
    (DirectML.dll), onnxruntime-gpu's, the CUDA libraries cantrip-infer loads (`CUDA_DLLS`), the model (its graphs,
    manifest, cards and references; not its checkpoint), the decklists, the prior's decklists, the pairings."""
    files = {"bin/cantrip-selfplay.exe": selfplay, "bin/cantrip-infer.exe": infer}
    for p in sorted(selfplay.parent.glob("*.dll")):
        files[f"bin/{p.name}"] = p
    for n in ORT_DLLS:
        files[f"bin/{n}"] = ort_dir / n
    for n in cuda_dlls:
        files[f"bin/{n}"] = cuda_dir / n
    return _with_data(files, model, decks, prior, pairings)


def linux_bundle_files(source: Path, model: Path, decks: Path | None, prior: Path | None,
                       pairings: Path | None) -> dict[str, Path]:
    """What a Linux worker needs (`LinuxSshHost`): the engine's source (`source_tar`), which it builds its binaries
    from, and the same model, decklists and pairings as a Windows worker's bundle; its GPU libraries are its own."""
    return _with_data({"src.tar.gz": source}, model, decks, prior, pairings)


def _with_data(files: dict[str, Path], model: Path, decks: Path | None, prior: Path | None,
               pairings: Path | None) -> dict[str, Path]:
    for p in sorted(model.rglob("*")):
        rel = p.relative_to(model)
        if p.is_file() and rel.parts[0] != "train" and p.name != "model.pt":
            files[f"model/{model.name}/{rel.as_posix()}"] = p
    for sub, d in (("decks", decks), ("prior", prior if prior and decks and prior.resolve() != decks.resolve() else None)):
        if d is not None:
            for p in sorted(d.glob("*.txt")):
                files[f"{sub}/{p.name}"] = p
    if pairings is not None:
        files["pairings.txt"] = pairings
    missing = [str(p) for p in files.values() if not p.exists()]
    if missing:
        raise SystemExit("the worker bundle is missing " + ", ".join(missing))
    return files


# What a Linux worker builds from (`source_tar`): the Cargo workspace and what its build scripts read.
SOURCE_PATHS = ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "crates", "content", "scenarios")
# What it builds (`LINUX_BUILD`), as this machine's driver builds them: cantrip-selfplay and cantrip-explore with
# onnxruntime (`--features ort`), cantrip-infer with the GPU engine (`--features gpu`).
LINUX_BINARIES = ("cantrip-selfplay", "cantrip-explore", "cantrip-infer")


def source_tar(repo: Path, out_dir: Path, keep: int = 3) -> Path:
    """The engine's source as this working tree has it - git's tracked files under `SOURCE_PATHS`, with their contents
    now, which is what this machine's own binaries were built from - as a gzip'd tar made the same way every time
    (sorted, no times or owners), so the same tree gives the same file: `out_dir/src-<hash>.tar.gz`, the last `keep`
    kept."""
    listed = subprocess.run(["git", "ls-files", "-z", "--", *SOURCE_PATHS], cwd=repo, capture_output=True, check=True).stdout
    present = []
    for rel in sorted(n for n in listed.decode("utf-8").split("\0") if n):
        try:
            st = (repo / rel).stat()
        except OSError:  # deleted in the working tree
            continue
        present.append((rel, st.st_size, st.st_mtime_ns))
    # Made again only when a file's size or time moved: reading every file takes minutes.
    key = hashlib.sha256("".join(f"{r} {s} {t}\n" for r, s, t in present).encode()).hexdigest()
    index = out_dir / "index.json"
    known = read_json(index) or {}
    if known.get(key) and (out_dir / known[key]).exists():
        os.utime(out_dir / known[key])
        return out_dir / known[key]
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w", format=tarfile.GNU_FORMAT) as tar:
        for rel, _, _ in present:
            p = repo / rel
            data = p.read_bytes()
            info = tarfile.TarInfo(rel)
            info.size, info.mode, info.mtime = len(data), 0o644, 0
            tar.addfile(info, io.BytesIO(data))
    raw = gzip.compress(buf.getvalue(), compresslevel=6, mtime=0)
    out_dir.mkdir(parents=True, exist_ok=True)
    out = out_dir / f"src-{hashlib.sha256(raw).hexdigest()[:16]}.tar.gz"
    if not out.exists():
        tmp = out.with_suffix(".tmp")
        tmp.write_bytes(raw)
        os.replace(tmp, out)
    os.utime(out)
    for old in sorted(out_dir.glob("src-*.tar.gz"), key=lambda p: p.stat().st_mtime, reverse=True)[keep:]:
        old.unlink(missing_ok=True)
    write_atomic(index, json.dumps({k: v for k, v in {**known, key: out.name}.items() if (out_dir / v).exists()}))
    return out


def bundle_manifest(files: dict[str, Path], cache: dict) -> tuple[str, dict]:
    """The bundle's manifest (each file's hash and size) and the bundle's hash, which names its directory."""
    manifest = {rel: dict(sha256=sha256(p, cache), size=p.stat().st_size) for rel, p in sorted(files.items())}
    h = hashlib.sha256("".join(f"{rel} {m['sha256']}\n" for rel, m in manifest.items()).encode()).hexdigest()[:16]
    return h, manifest


def ps_quote(s: str) -> str:
    return "'" + str(s).replace("'", "''") + "'"


def b64(text: str) -> str:
    return base64.b64encode(text.encode("utf-8")).decode()


def shielded_run(cmd: list[str], input: bytes | None = None, timeout: float = 120.0):
    """Runs `cmd` in a process group of its own, so a Ctrl-C at the console does not break it mid-transfer: an
    interrupt meanwhile waits for it to end, and is raised after. Returns (code, stdout, stderr)."""
    group = dict(creationflags=subprocess.CREATE_NEW_PROCESS_GROUP) if os.name == "nt" else dict(start_new_session=True)
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE if input is not None else subprocess.DEVNULL,
                         stdout=subprocess.PIPE, stderr=subprocess.PIPE, **group)
    interrupted = False
    deadline = time.monotonic() + timeout
    while True:
        try:
            out, err = p.communicate(input=input, timeout=max(0.1, deadline - time.monotonic()))
            break
        except KeyboardInterrupt:
            interrupted, input = True, None
        except subprocess.TimeoutExpired:
            p.kill()
            p.communicate()
            raise
    if interrupted:
        raise KeyboardInterrupt
    return p.returncode, out, err


class _Ran:
    def __init__(self, code, out, err):
        self.returncode, self.stdout, self.stderr = code, out, err


def _shielded(cmd, input=None, stdout=None, stderr=None, timeout=120.0):
    return _Ran(*shielded_run(cmd, input=input, timeout=timeout))


# The supervisor a worker's attempt runs, detached from the SSH session: cantrip-infer (unless it plays on the CPU),
# then cantrip-selfplay on it; every process's id, start time and image in pids.json (written whole, then renamed), and
# whatever ends it, the self-play's exit code in exit.json and the server stopped (a supervisor killed outright leaves
# neither: the coordinator's poll finds it dead and recovers the run).
RUN_PS1 = r"""
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$job = Get-Content -Raw job.json | ConvertFrom-Json
foreach ($p in $job.env.PSObject.Properties) { Set-Item -Path ("env:" + $p.Name) -Value $p.Value }
function Who($proc) { [ordered]@{ pid = $proc.Id; start = $proc.StartTime.ToFileTimeUtc(); path = $proc.Path } }
$pids = [ordered]@{ supervisor = (Who (Get-Process -Id $PID)) }
function Save-Pids { $pids | ConvertTo-Json | Set-Content -Encoding Ascii pids.json.tmp; Move-Item -Force pids.json.tmp pids.json }
function Save-Exit($code, $why) {
  [ordered]@{ code = $code; why = $why; at = (Get-Date -Format s) } | ConvertTo-Json | Set-Content -Encoding Ascii exit.json.tmp
  Move-Item -Force exit.json.tmp exit.json
}
$infer = $null
$sp = $null
$code = -1
$why = ''
try {
  Save-Pids
  $arguments = @($job.selfplay_args)
  if ($job.infer) {
    Remove-Item -ErrorAction SilentlyContinue infer.ready
    $infer = Start-Process -FilePath $job.infer -ArgumentList $job.infer_args -RedirectStandardOutput infer.out.log -RedirectStandardError infer.log -PassThru -NoNewWindow
    $null = $infer.Handle
    $pids.infer = (Who $infer); Save-Pids
    $deadline = (Get-Date).AddMinutes(10)
    while (-not (Test-Path infer.ready)) {
      if ($infer.HasExited -or (Get-Date) -gt $deadline) { $code = -2; throw 'the inference server did not start (infer.log)' }
      Start-Sleep -Milliseconds 300
    }
    $arguments += @('--server', (Get-Content -Raw infer.ready).Trim())
  }
  $sp = Start-Process -FilePath $job.selfplay -ArgumentList $arguments -RedirectStandardOutput selfplay.out.log -RedirectStandardError selfplay.log -PassThru -NoNewWindow
  $null = $sp.Handle
  $pids.selfplay = (Who $sp); Save-Pids
  $sp.WaitForExit()
  $code = $sp.ExitCode
} catch {
  $why = "$_"
} finally {
  if ($sp -and -not $sp.HasExited) { Stop-Process -Id $sp.Id -Force -ErrorAction SilentlyContinue; $why = "$why (self-play stopped by its supervisor)" }
  Save-Exit $code $why
  if ($infer -and -not $infer.HasExited) { Stop-Process -Id $infer.Id -Force -ErrorAction SilentlyContinue }
}
"""

# PowerShell: whether a recorded process (`pid`, `start`, `path`) is the one running now.
_SAME = r"""
function Same($w) {
  if (-not $w -or -not $w.pid) { return $false }
  $p = Get-Process -Id $w.pid -ErrorAction SilentlyContinue
  if (-not $p) { return $false }
  try { $s = $p.StartTime.ToFileTimeUtc() } catch { return $false }
  if ($w.start -and [int64]$w.start -ne $s) { return $false }
  if ($w.path -and $p.Path -and ($w.path -ne $p.Path)) { return $false }
  return $true
}
"""


class SshHost:
    """A Windows worker over SSH (OpenSSH Server, a key for a local administrator, cmd its default shell): PowerShell
    scripts piped to it, files over scp, both in process groups of their own (`shielded_run`). Its tree under the
    worker's `root`: `blobs/<sha256>` (every file pushed once), `bundle/<hash>/` (a generation's binaries, libraries,
    model, decks and pairings, hard links to the blobs, its `MANIFEST.json` written last; the last two kept, and the
    blobs nothing names removed), and `gens/<root tag>/genN/<host>/<attempt>-<nonce>/` (a self-play run: `job.json`,
    `run.ps1`, `grant.txt`, `heartbeat`, `selfplay/`, the logs, `pids.json`, `exit.json`)."""

    remote = True

    def __init__(self, worker: dict, tag: str, gen: int, files: dict[str, Path], manifest: dict, bundle_hash: str,
                 job, log=print, run=_shielded, lease: float = 900.0):
        self.w, self.name, self.tag, self.gen = worker, worker["name"], tag, gen
        self.root = worker["root"].rstrip("/\\").replace("\\", "/")
        self.files, self.manifest, self.hash = files, manifest, bundle_hash
        self.job, self.log, self._run, self.lease = job, log, run, lease
        self.pushed = False
        self.gpu = None

    # Transport.

    def ssh_base(self) -> list[str]:
        # A host seen for the first time (a rented machine is new each time) is trusted and remembered; one whose key
        # changed is still refused.
        cmd = ["ssh", "-o", "BatchMode=yes", "-o", "ConnectTimeout=10", "-o", "ServerAliveInterval=15",
               "-o", "ServerAliveCountMax=4", "-o", "StrictHostKeyChecking=accept-new"]
        if self.w.get("key"):
            cmd += ["-i", self.w["key"]]
        if self.w.get("port"):
            cmd += ["-p", str(self.w["port"])]
        return cmd + [f"{self.w['user']}@{self.w['host']}"]

    def ps(self, script: str, timeout: float = 120) -> str:
        """Runs a PowerShell script on the worker; its stdout."""
        cmd = self.ssh_base() + [f"powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand {_ENCODED}"]
        try:
            r = self._run(cmd, input=(script + "\n" + _END + "\n").encode("utf-8"), stdout=subprocess.PIPE,
                          stderr=subprocess.PIPE, timeout=timeout)
        except subprocess.TimeoutExpired as e:
            raise Unreachable(f"{self.name}: no answer in {timeout:.0f} s") from e
        except OSError as e:
            raise Unreachable(f"{self.name}: {e}") from e
        if r.returncode != 0:
            raise Unreachable(f"{self.name}: exit {r.returncode}: {r.stderr.decode(errors='replace').strip()[-600:]}")
        return r.stdout.decode(errors="replace")

    def ps_json(self, script: str, timeout: float = 120):
        out = self.ps(script, timeout).strip()
        try:
            return json.loads(out.splitlines()[-1]) if out else None
        except ValueError as e:
            raise Unreachable(f"{self.name}: not JSON: {out[-300:]}") from e

    def scp(self, src: str, dst: str, timeout: float = 3600):
        cmd = ["scp", "-q", "-B", "-o", "ConnectTimeout=10", "-o", "StrictHostKeyChecking=accept-new"]
        if self.w.get("key"):
            cmd += ["-i", self.w["key"]]
        if self.w.get("port"):
            cmd += ["-P", str(self.w["port"])]
        try:
            r = self._run(cmd + [src, dst], stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
        except subprocess.TimeoutExpired as e:
            raise Unreachable(f"{self.name}: scp timed out") from e
        except OSError as e:
            raise Unreachable(f"{self.name}: {e}") from e
        if r.returncode != 0:
            raise Unreachable(f"{self.name}: scp exit {r.returncode}: {r.stderr.decode(errors='replace').strip()[-400:]}")

    def remote_path(self, rel: str) -> str:
        return f"{self.w['user']}@{self.w['host']}:{self.root}/{rel}"

    # The bundle.

    def bundle_dir(self) -> str:
        return f"{self.root}/bundle/{self.hash}"

    def prepare(self, live: list[dict]):
        """Ready for this generation: the bundle pushed, every run of this worker's under this root's tag that is not
        one of `live` stopped (orphans of a coordinator that went away; another worker's runs, sharing its root, are
        left alone), and all but the last two bundles and the blobs nothing names removed."""
        self.push()
        keep = [self.attempt_dir(a) for a in live]
        self.ps(f"""
$root = {ps_quote(self.root)}
{_SAME}
$keep = @({", ".join(ps_quote(k.replace("/", "\\").lower()) for k in keep) or ""})
$gens = Join-Path $root {ps_quote("gens/" + self.tag)}
if (Test-Path $gens) {{
  # Self-play attempts only: a pair-mining job (`mining.RemoteJudge`, a `mine-` directory) is no attempt's orphan.
  Get-ChildItem -Path $gens -Recurse -Filter pids.json -File | Where-Object {{ (Split-Path (Split-Path $_.DirectoryName) -Leaf) -eq {ps_quote(self.name)} -and (Split-Path $_.DirectoryName -Leaf) -notlike 'mine-*' }} | ForEach-Object {{
    $dir = $_.DirectoryName
    if ($keep -notcontains $dir.ToLower()) {{
      try {{ $p = Get-Content -Raw $_.FullName | ConvertFrom-Json }} catch {{ return }}
      foreach ($k in 'selfplay', 'supervisor', 'infer') {{ if (Same $p.$k) {{ Stop-Process -Id $p.$k.pid -Force -ErrorAction SilentlyContinue }} }}
    }}
  }}
}}
$bundles = @(Get-ChildItem -Path "$root/bundle" -Directory -ErrorAction SilentlyContinue | Where-Object {{ Test-Path (Join-Path $_.FullName 'MANIFEST.json') }} | Sort-Object {{ (Get-Item (Join-Path $_.FullName 'MANIFEST.json')).LastWriteTime }} -Descending)
$kept = @($bundles | Where-Object {{ $_.Name -eq {ps_quote(self.hash)} }}) + @($bundles | Where-Object {{ $_.Name -ne {ps_quote(self.hash)} }} | Select-Object -First 1)
foreach ($b in $bundles) {{ if ($kept.Name -notcontains $b.Name) {{ Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $b.FullName }} }}
$named = @{{}}
foreach ($b in $kept) {{
  $m = Get-Content -Raw (Join-Path $b.FullName 'MANIFEST.json') | ConvertFrom-Json
  foreach ($f in $m.PSObject.Properties) {{ $named[$f.Value.sha256] = 1 }}
}}
Get-ChildItem -Path "$root/blobs" -File -ErrorAction SilentlyContinue | Where-Object {{ -not $named.ContainsKey($_.Name) -and ($_.Name -notlike '*.tmp' -or $_.LastWriteTime -lt (Get-Date).AddDays(-1)) }} | Remove-Item -Force -ErrorAction SilentlyContinue
""", timeout=600)

    def push(self):
        """The bundle, pushed once: the blobs the worker lacks over scp (each checked by its hash there), then the
        bundle directory made of hard links to them, its manifest last."""
        if self.pushed:
            return
        have = self.ps_json(f"""
$root = {ps_quote(self.root)}
$b = {ps_quote(self.bundle_dir())}
if (Test-Path "$b/MANIFEST.json") {{ ConvertTo-Json -Compress @{{ complete = $true }}; return }}
New-Item -ItemType Directory -Force "$root/blobs" | Out-Null
$names = @(Get-ChildItem "$root/blobs" -File | Where-Object {{ $_.Name -notlike '*.tmp' }} | ForEach-Object {{ $_.Name }})
ConvertTo-Json -Compress -InputObject @{{ complete = $false; blobs = $names }}
""")
        if not have.get("complete"):
            blobs = set(have.get("blobs") or [])
            want: dict[str, Path] = {}
            for rel, m in self.manifest.items():
                if m["sha256"] not in blobs:
                    want[m["sha256"]] = self.files[rel]
            size = sum(p.stat().st_size for p in want.values())
            self.log(f"  [{self.name}] pushing {len(want)} files ({size / 2**20:.0f} MB) of bundle {self.hash}; "
                     f"{len(self.manifest) - len(want)} there already")
            for sha, p in want.items():
                self.scp(str(p), self.remote_path(f"blobs/{sha}.tmp"))
            checks = "\n".join(f"Check {ps_quote(sha)}" for sha in want)
            links = "\n".join(f"Link {ps_quote(rel)} {ps_quote(m['sha256'])}" for rel, m in self.manifest.items())
            self.ps(f"""
$root = {ps_quote(self.root)}
function Check($sha) {{
  $t = "$root/blobs/$sha.tmp"
  $h = (Get-FileHash -Algorithm SHA256 $t).Hash.ToLower()
  if ($h -ne $sha) {{ Remove-Item $t; throw "blob $sha arrived as $h" }}
  Move-Item -Force $t "$root/blobs/$sha"
}}
{checks}
$b = {ps_quote(self.bundle_dir())}
function Link($rel, $sha) {{
  $dst = "$b/$rel"
  New-Item -ItemType Directory -Force (Split-Path $dst) | Out-Null
  if (Test-Path $dst) {{ Remove-Item -Force $dst }}
  New-Item -ItemType HardLink -Path $dst -Target "$root/blobs/$sha" | Out-Null
}}
{links}
[IO.File]::WriteAllText("$b/MANIFEST.json", [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String({ps_quote(b64(json.dumps(self.manifest, indent=1)))})))
""", timeout=1800)
        self.pushed = True

    # Attempts.

    def attempt_dir(self, a: dict) -> str:
        return f"{self.root}/gens/{self.tag}/gen{self.gen}/{self.name}/{a['id']}-{a.get('nonce', 'x')}"

    def launch(self, a: dict, grant: str) -> dict:
        self.push()
        d = self.attempt_dir(a)
        job = self.job(self, a, d)
        job["selfplay_args"] = list(job["selfplay_args"]) + ["--lease", f"{d}/heartbeat", "--lease-seconds", str(self.lease)]
        for v in [job.get("infer") or "", job["selfplay"], *job.get("infer_args", []), *job["selfplay_args"]]:
            if " " in str(v) or '"' in str(v):
                raise SystemExit(f"a worker's argument with a space or a quote cannot be passed: {v!r}")
        r = self.ps_json(f"""
$d = {ps_quote(d)}
New-Item -ItemType Directory -Force "$d/selfplay" | Out-Null
function Put($name, $text) {{ [IO.File]::WriteAllText("$d/$name.tmp", [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($text))); Move-Item -Force "$d/$name.tmp" "$d/$name" }}
Put 'run.ps1' {ps_quote(b64(RUN_PS1))}
Put 'grant.txt' {ps_quote(b64(grant))}
Put 'heartbeat' ''
Remove-Item -ErrorAction SilentlyContinue "$d/selfplay/{STOP_FILE}", "$d/exit.json"
$gpu = ''
try {{ $gpu = (& nvidia-smi --query-gpu=name,driver_version --format=csv,noheader 2>$null | Select-Object -First 1) }} catch {{ $gpu = '' }}
# job.json last: an attempt directory with one was launched (or is being launched); one without never started.
Put 'job.json' {ps_quote(b64(json.dumps(job, indent=1)))}
$line = 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + ($d -replace '/', '\\') + '\\run.ps1"'
$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{ CommandLine = $line; CurrentDirectory = ($d -replace '/', '\\') }}
if ($r.ReturnValue -ne 0) {{ throw "Win32_Process.Create returned $($r.ReturnValue)" }}
$p = Get-Process -Id $r.ProcessId -ErrorAction SilentlyContinue
$who = @{{ pid = [int]$r.ProcessId; start = $(if ($p) {{ $p.StartTime.ToFileTimeUtc() }} else {{ $null }}); path = $(if ($p) {{ $p.Path }} else {{ $null }}) }}
ConvertTo-Json -Compress -InputObject @{{ supervisor = $who; gpu = "$gpu" }}
""")
        self.gpu = (r.get("gpu") or "").strip() or None
        return dict(launched=dict(supervisor=r.get("supervisor")), remote_dir=d, gpu=self.gpu, **(job.get("record") or {}))

    def grant(self, a: dict, text: str):
        d = self.attempt_dir(a)
        self.ps(f"""
$d = {ps_quote(d)}
[IO.File]::WriteAllText("$d/grant.txt.tmp", [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String({ps_quote(b64(text))})))
Move-Item -Force "$d/grant.txt.tmp" "$d/grant.txt"
""")

    def poll(self, a: dict) -> Poll:
        """Its heartbeat touched, the shards accepted at the last poll deleted, then: its processes' liveness (from
        pids.json, or the supervisor its launch recorded), and only then its exit, status, shards, runaways, report and
        grant file."""
        d = self.attempt_dir(a)
        launched = json.dumps(((a.get("launched") or {}).get("supervisor")) or {})
        deletes = ", ".join(ps_quote(n) for n in a.get("to_delete") or [])
        r = self.ps_json(f"""
$d = {ps_quote(d)}
{_SAME}
function Raw($p) {{ try {{ [IO.File]::ReadAllText($p) }} catch {{ $null }} }}
$job = Test-Path "$d/job.json"
if ($job) {{ if (Test-Path "$d/heartbeat") {{ [IO.File]::SetLastWriteTimeUtc("$d/heartbeat", [DateTime]::UtcNow) }} else {{ [IO.File]::WriteAllText("$d/heartbeat", '') }} }}
foreach ($n in @({deletes})) {{ Remove-Item -Force -ErrorAction SilentlyContinue "$d/selfplay/$n" }}
$pids = Raw "$d/pids.json"
$alive = $false
$known = $false
if ($pids) {{
  $known = $true
  $p = $pids | ConvertFrom-Json
  foreach ($k in 'supervisor', 'selfplay') {{ if (Same $p.$k) {{ $alive = $true }} }}
}} else {{
  $l = {ps_quote(launched)} | ConvertFrom-Json
  if ($l.pid) {{ $known = $true; if (Same $l) {{ $alive = $true }} }}
}}
$exit = Raw "$d/exit.json"
$shards = @(Get-ChildItem "$d/selfplay" -Filter *.shard.zst -File -ErrorAction SilentlyContinue | ForEach-Object {{ $_.Name }})
$o = [ordered]@{{ job = $job; known = $known; status = (Raw "$d/selfplay/{STATUS}"); exit = $exit; pids = $pids; alive = $alive;
  shards = $shards; runaways = (Raw "$d/selfplay/{RUNAWAYS}"); grant = (Raw "$d/grant.txt");
  report = $(if ($exit) {{ Raw "$d/selfplay/selfplay-report.json" }} else {{ $null }}) }}
ConvertTo-Json -Compress -InputObject $o
""")

        return poll_from(r, a)

    def fetch(self, a: dict, name: str, dest: Path):
        rel = f"{self.attempt_dir(a)[len(self.root) + 1:]}/selfplay/{name}"
        self.scp(self.remote_path(rel), str(dest), timeout=1800)

    def fetch_logs(self, a: dict, dest: Path):
        """The attempt's logs, beside the generation's (best effort)."""
        dest.mkdir(parents=True, exist_ok=True)
        base = self.attempt_dir(a)[len(self.root) + 1:]
        for n in ("selfplay.log", "infer.log"):
            try:
                self.scp(self.remote_path(f"{base}/{n}"), str(dest / f"{a['id']}-{n}"), timeout=300)
            except Unreachable:
                pass

    def delete(self, a: dict, names: list[str]):
        """Its shards accepted here, deleted there."""
        d = self.attempt_dir(a)
        self.ps(f"""
$d = {ps_quote(d)}
foreach ($n in @({", ".join(ps_quote(n) for n in names)})) {{ Remove-Item -Force -ErrorAction SilentlyContinue "$d/selfplay/$n" }}
""")

    def stop(self, a: dict, now: bool):
        d = self.attempt_dir(a)
        self.ps(f"""
$d = {ps_quote(d)}
if (Test-Path "$d/selfplay") {{ Set-Content -Encoding Ascii -Path "$d/selfplay/{STOP_FILE}" -Value {ps_quote('now' if now else '')} }}
""")

    def kill(self, a: dict):
        """Its processes, each only if it is the one recorded (id, start time and image)."""
        d = self.attempt_dir(a)
        launched = json.dumps(((a.get("launched") or {}).get("supervisor")) or {})
        self.ps(f"""
$d = {ps_quote(d)}
{_SAME}
if (Test-Path "$d/selfplay") {{ Set-Content -Encoding Ascii -Path "$d/selfplay/{STOP_FILE}" -Value 'now' }}
$p = $null
if (Test-Path "$d/pids.json") {{ $p = Get-Content -Raw "$d/pids.json" | ConvertFrom-Json }}
$all = @()
if ($p) {{ $all = @($p.supervisor, $p.selfplay, $p.infer) }} else {{ $all = @(({ps_quote(launched)} | ConvertFrom-Json)) }}
foreach ($w in $all) {{ if (Same $w) {{ Stop-Process -Id $w.pid -Force -ErrorAction SilentlyContinue }} }}
""")

    def recover(self, a: dict):
        """Its `.part`s recovered by `cantrip-selfplay --recover` (needs no server). An attempt whose job.json is not
        there never started anything: nothing to recover."""
        d = self.attempt_dir(a)
        self.ps(f"""
$d = {ps_quote(d)}
if (-not (Test-Path "$d/job.json")) {{ return }}
$job = Get-Content -Raw "$d/job.json" | ConvertFrom-Json
$given = @($job.recover_args)
& $job.selfplay @given *> "$d/recover.log"
if ($LASTEXITCODE -ne 0) {{ throw "cantrip-selfplay --recover exited $LASTEXITCODE (recover.log)" }}
""", timeout=1800)


# Linux workers.

# A Linux worker's toolchain and GPU libraries, installed once (`bash setup.sh ROOT`, detached; `setup.log`): Rust, a
# C compiler, and onnxruntime-gpu with the CUDA libraries its provider loads (cuDNN, cuRAND, cuFFT, cuBLAS, cudart) in
# a venv, an image's own CUDA being whatever it is; `env.json` (ORT_DYLIB_PATH, LD_LIBRARY_PATH) written last, only
# once the provider's libraries all resolve.
LINUX_SETUP = r"""#!/bin/bash
set -u
R=$1
echo "[$(date -Is)] setup"
# A bare CUDA image (nvidia/cuda's) has no curl, compiler or venv: what is missing is installed.
if ! command -v curl >/dev/null || ! command -v cc >/dev/null || ! python3 -c "import ensurepip" 2>/dev/null; then
  export DEBIAN_FRONTEND=noninteractive
  apt-get -qq update && apt-get -qq install -y curl ca-certificates build-essential python3 python3-venv >/dev/null || exit 1
fi
if [ ! -x "$HOME/.cargo/bin/cargo" ]; then
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable || exit 1
fi
if [ ! -x "$R/venv/bin/pip" ]; then
  rm -rf "$R/venv"
  python3 -m venv "$R/venv" || { rm -rf "$R/venv"; apt-get -qq update && apt-get -qq install -y python3-venv >/dev/null && python3 -m venv "$R/venv"; } || exit 1
fi
"$R/venv/bin/pip" -q install --upgrade pip || exit 1
"$R/venv/bin/pip" -q install onnxruntime-gpu==1.24.4 nvidia-cudnn-cu12 nvidia-curand-cu12 nvidia-cufft-cu12 \
  nvidia-cublas-cu12 nvidia-cuda-runtime-cu12 || exit 1
"$R/venv/bin/python" - "$R" <<'PY' || exit 1
import glob, json, os, subprocess, sys
r = sys.argv[1]
sp = glob.glob(f"{r}/venv/lib/python3*/site-packages")[0]
capi = f"{sp}/onnxruntime/capi"
libs = ":".join([capi] + sorted(glob.glob(f"{sp}/nvidia/*/lib")))
missing = [l for l in subprocess.run(["ldd", f"{capi}/libonnxruntime_providers_cuda.so"], env=dict(os.environ, LD_LIBRARY_PATH=libs),
                                     capture_output=True, text=True).stdout.splitlines() if "not found" in l]
if missing:
    sys.exit("onnxruntime's CUDA provider is missing " + "; ".join(m.strip() for m in missing))
with open(f"{r}/env.json.tmp", "w") as f:
    json.dump({"ORT_DYLIB_PATH": sorted(glob.glob(f"{capi}/libonnxruntime.so.*"))[0], "LD_LIBRARY_PATH": libs}, f)
os.replace(f"{r}/env.json.tmp", f"{r}/env.json")
PY
echo "[$(date -Is)] done"
"""

# A Linux worker's binaries from a bundle's source (`bash build.sh ROOT SHA`, detached; `builds/SHA/build.log`):
# unpacked into `src/SHA`, built with cargo's `target/` shared by every build (a change rebuilds only what it
# touches), the binaries copied into `builds/SHA/` and `done` written last.
LINUX_BUILD = r"""#!/bin/bash
set -eu
R=$1; S=$2
source "$HOME/.cargo/env"
echo "[$(date -Is)] build $S"
rm -rf "$R/src/$S"; mkdir -p "$R/src/$S"
tar -xzf "$R/blobs/$S" -C "$R/src/$S"
cd "$R/src/$S"
export CARGO_TARGET_DIR="$R/target"
cargo build --release -p cantrip-ai-host --features ort --bin cantrip-selfplay --bin cantrip-explore
cp "$R/target/release/cantrip-selfplay" "$R/target/release/cantrip-explore" "$R/builds/$S/"
cargo build --release -p cantrip-ai-host --features gpu --bin cantrip-infer
cp "$R/target/release/cantrip-infer" "$R/builds/$S/"
touch "$R/builds/$S/done"
echo "[$(date -Is)] done"
"""

# Where a Linux worker's self-play and server run (`pin_plan`). A lane waits on the server for a third of its time or
# more, so a box needs more lanes than cores to keep its cores busy; but two lanes on one core's two hardware threads
# make little more than one, and a container's CPU quota counts thread-seconds, so lanes left for Linux to place double
# up on siblings while other cores sit idle, burn the quota for half the work and get the whole container throttled,
# the server too (measured on a 5090 with 64 cores: 2,533 requests a second as Linux placed 60 lanes, 3,246 with 100
# lanes on one thread each of 58 cores). Self-play gets one hardware thread on each of as many cores as the quota allows
# less `PIN_SERVER_CORES`, the server both threads of those, and the lanes `LANES_PER_CORE` a pinned core.
PIN_SERVER_CORES = 3
LANES_PER_CORE = 1.7


def parse_cpus(text: str) -> set[int]:
    """A kernel CPU list ("0-3,8,10-11") as a set."""
    out = set()
    for part in text.strip().split(","):
        if part:
            a, _, b = part.partition("-")
            out.update(range(int(a), int(b or a) + 1))
    return out


def read_topology():
    """This process's CPUs, each one's core (the hardware threads it shares one with), and the cgroup's CPU quota in
    CPUs (cgroup v2's `cpu.max`, or v1's quota over its period; None without one)."""
    allowed = set(os.sched_getaffinity(0))
    siblings = {}
    for c in allowed:
        with open(f"/sys/devices/system/cpu/cpu{c}/topology/thread_siblings_list") as f:
            siblings[c] = frozenset(parse_cpus(f.read()))
    quota = None
    try:
        q, p = open("/sys/fs/cgroup/cpu.max").read().split()[:2]
        quota = int(q) / int(p) if q != "max" else None
    except (OSError, ValueError):
        try:
            q = int(open("/sys/fs/cgroup/cpu/cpu.cfs_quota_us").read())
            quota = q / int(open("/sys/fs/cgroup/cpu/cpu.cfs_period_us").read()) if q > 0 else None
        except (OSError, ValueError):
            quota = None
    return allowed, siblings, quota


def pin_plan(allowed, siblings, quota, server_cores=PIN_SERVER_CORES, lanes_per_core=None):
    """Self-play's CPUs (one hardware thread of each of its cores), the server's (every thread of `server_cores` cores
    of its own), and the lanes (`lanes_per_core` a self-play core, if given): self-play has as many cores as the
    quota's whole CPUs (all of them without one) less the server's, so the two together never reach the quota. None
    on a box too small to split."""
    cores = sorted({tuple(sorted(siblings[c] & allowed)) for c in allowed})
    n = min(len(cores), len(cores) if quota is None else int(quota)) - server_cores
    if n < 1 or len(cores) < n + server_cores:
        return None
    return dict(selfplay=sorted(core[0] for core in cores[:n]),
                server=sorted(c for core in cores[n:n + server_cores] for c in core),
                cores=n, physical=len(cores), quota=quota,
                lanes=max(1, round(lanes_per_core * n)) if lanes_per_core else None)


def with_lanes(arguments: list, lanes: int) -> list:
    """Self-play's arguments with `lanes` lanes, all of them from the start (the governor's growth off)."""
    out = [a for a in arguments if a != "--autoscale"]
    if "--lanes" in out:
        out[out.index("--lanes") + 1] = str(lanes)
    else:
        out += ["--lanes", str(lanes)]
    return out if "--no-autoscale" in out else out + ["--no-autoscale"]


# A Linux attempt's supervisor (`run.py` in its directory, started in a session of its own): job.json's cantrip-infer
# (until its ready file) and cantrip-selfplay, with job.json's environment and the worker's `env.json`; pids.json as
# each starts, exit.json when self-play ends; the server stopped with it. What `RUN_PS1` does on Windows. With
# job.json's `pin`, the two are pinned to the box's cores (`pin_plan`, written to pin.json; nothing is pinned where the
# topology cannot be read), and with its `lanes_per_core` the lanes are that many a self-play core.
RUN_PY = r'''
import json, os, subprocess, sys, time

@PIN@

os.chdir(os.path.dirname(os.path.abspath(__file__)))
job = json.load(open("job.json", encoding="utf-8"))
env = dict(os.environ)
env.update(job.get("env") or {})
if job.get("env_file"):
    e = json.load(open(job["env_file"], encoding="utf-8"))
    env["ORT_DYLIB_PATH"] = e["ORT_DYLIB_PATH"]
    env["LD_LIBRARY_PATH"] = e["LD_LIBRARY_PATH"] + (":" + env["LD_LIBRARY_PATH"] if env.get("LD_LIBRARY_PATH") else "")


def who(pid):
    try:
        s = open(f"/proc/{pid}/stat").read()
        start = int(s[s.rindex(")") + 2:].split()[19])
    except (OSError, ValueError, IndexError):
        start = None
    try:
        path = os.readlink(f"/proc/{pid}/exe").removesuffix(" (deleted)")
    except OSError:
        path = None
    return dict(pid=pid, start=start, path=path)


def save(name, o):
    with open(name + ".tmp", "w", encoding="utf-8") as f:
        json.dump(o, f, indent=1)
    os.replace(name + ".tmp", name)


pids = dict(supervisor=who(os.getpid()))
infer = sp = None
code, why = -1, ""
plan = None
if job.get("pin"):
    try:
        plan = pin_plan(*read_topology(), lanes_per_core=job.get("lanes_per_core"))
        save("pin.json", plan or dict(pinned=False, why="too few cores to split"))
    except Exception as e:
        save("pin.json", dict(pinned=False, why=str(e)))


def on(cpus):
    """A child's start: pinned to `cpus` (none: where Linux places it)."""
    return (lambda: os.sched_setaffinity(0, cpus)) if cpus else None


try:
    save("pids.json", pids)
    arguments = list(job["selfplay_args"])
    if plan and plan.get("lanes"):
        arguments = with_lanes(arguments, plan["lanes"])
    if job.get("infer"):
        if os.path.exists("infer.ready"):
            os.remove("infer.ready")
        infer = subprocess.Popen([job["infer"], *job["infer_args"]], stdin=subprocess.DEVNULL, stdout=open("infer.out.log", "wb"),
                                 stderr=open("infer.log", "wb"), env=env, preexec_fn=on(plan and plan["server"]))
        pids["infer"] = who(infer.pid)
        save("pids.json", pids)
        deadline = time.time() + 600
        while not os.path.exists("infer.ready"):
            if infer.poll() is not None or time.time() > deadline:
                code = -2
                raise RuntimeError("the inference server did not start (infer.log)")
            time.sleep(0.3)
        arguments += ["--server", open("infer.ready", encoding="utf-8").read().strip()]
    sp = subprocess.Popen([job["selfplay"], *arguments], stdin=subprocess.DEVNULL, stdout=open("selfplay.out.log", "wb"),
                          stderr=open("selfplay.log", "wb"), env=env, preexec_fn=on(plan and plan["selfplay"]))
    pids["selfplay"] = who(sp.pid)
    save("pids.json", pids)
    code = sp.wait()
except Exception as e:
    why = str(e)
finally:
    if sp is not None and sp.poll() is None:
        sp.kill()
        sp.wait()
        why += " (self-play stopped by its supervisor)"
    save("exit.json", dict(code=code, why=why, at=time.strftime("%Y-%m-%dT%H:%M:%S")))
    if infer is not None and infer.poll() is None:
        infer.kill()
        infer.wait()
'''
# The pinning's functions, as they are here (and tested here), in the script the worker runs.
RUN_PY = RUN_PY.replace("@PIN@", "\n\n".join(
    [f"PIN_SERVER_CORES = {PIN_SERVER_CORES}"] + [inspect.getsource(f) for f in (parse_cpus, read_topology, pin_plan, with_lanes)]))

# What every script sent to a Linux worker starts with: its arguments (`A`, from base64'd JSON), and the helpers. A
# process is recorded by its id, its start (clock ticks since boot, /proc/PID/stat) and its executable, and is the one
# recorded only while all three still match (a zombie is not running).
_PY_PRELUDE = r'''
import base64, glob, hashlib, json, os, shutil, signal, subprocess, sys, time
A = json.loads(base64.b64decode("@ARGS@"))
ROOT = A["root"]


def read(p):
    try:
        with open(p, encoding="utf-8") as f:
            return f.read()
    except OSError:
        return None


def put(p, text):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p + ".tmp", "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    os.replace(p + ".tmp", p)


def remove(p):
    try:
        os.remove(p)
    except OSError:
        pass


def fields(pid):
    try:
        s = open(f"/proc/{int(pid)}/stat").read()
        return s[s.rindex(")") + 2:].split()
    except (OSError, ValueError):
        return None


def exe(pid):
    try:
        return os.readlink(f"/proc/{int(pid)}/exe").removesuffix(" (deleted)")
    except OSError:
        return None


def who(pid):
    f = fields(pid)
    return dict(pid=pid, start=int(f[19]) if f else None, path=exe(pid))


def same(w):
    if not w or not w.get("pid"):
        return False
    f = fields(w["pid"])
    if not f or f[0] in "ZX":
        return False
    if w.get("start") is not None and int(w["start"]) != int(f[19]):
        return False
    p = exe(w["pid"])
    return not (w.get("path") and p and p != w["path"])


def kill(w):
    if same(w):
        try:
            os.kill(int(w["pid"]), signal.SIGKILL)
        except OSError:
            pass


def detached(argv, cwd, log):
    """A process in a session of its own, which outlives this script and the SSH session."""
    return subprocess.Popen(argv, cwd=cwd, stdin=subprocess.DEVNULL, stdout=open(log, "ab"), stderr=subprocess.STDOUT,
                            start_new_session=True)


def running(pidfile):
    try:
        return same(json.loads(read(pidfile) or "null"))
    except ValueError:
        return False


def tail(p, n=1):
    lines = [l for l in (read(p) or "").splitlines() if l.strip()]
    return " | ".join(lines[-n:])


def out(o):
    print(json.dumps(o))
'''

# Its orphans stopped (every run of this worker's under this root's tag but `keep`, pair mining's jobs aside), all but
# the current bundle and the newest other complete one removed, and the blobs, sources and builds nothing kept names.
_LINUX_PRUNE = r'''
for pj in glob.glob(f"{ROOT}/gens/{A['tag']}/gen*/{A['name']}/*/pids.json"):
    d = os.path.dirname(pj)
    if os.path.basename(d).startswith("mine-") or d in A["keep"]:
        continue
    try:
        p = json.loads(read(pj) or "{}")
    except ValueError:
        continue
    for k in ("selfplay", "supervisor", "infer"):
        kill(p.get(k))
done = sorted((b for b in glob.glob(f"{ROOT}/bundle/*") if os.path.exists(f"{b}/MANIFEST.json")),
              key=lambda b: os.path.getmtime(f"{b}/MANIFEST.json"), reverse=True)
kept = [b for b in done if os.path.basename(b) == A["hash"]] + [b for b in done if os.path.basename(b) != A["hash"]][:1]
for b in done:
    if b not in kept:
        shutil.rmtree(b, ignore_errors=True)
named, sources = set(A["want"]), {A["source"]}
for b in kept:
    for rel, m in json.loads(read(f"{b}/MANIFEST.json")).items():
        named.add(m["sha256"])
        if rel == "src.tar.gz":
            sources.add(m["sha256"])
for f in glob.glob(f"{ROOT}/blobs/*"):
    n = os.path.basename(f)
    if (n.endswith(".tmp") and time.time() - os.path.getmtime(f) > 86400) or (not n.endswith(".tmp") and n not in named):
        remove(f)
for sub in ("builds", "src"):
    for d in glob.glob(f"{ROOT}/{sub}/*"):
        if os.path.basename(d) not in sources and not running(f"{ROOT}/builds/{os.path.basename(d)}/pid.json"):
            shutil.rmtree(d, ignore_errors=True)
out({})
'''

_LINUX_HAVE = r'''
b = f"{ROOT}/bundle/{A['hash']}"
if os.path.exists(f"{b}/MANIFEST.json"):
    out(dict(complete=True))
else:
    os.makedirs(f"{ROOT}/blobs", exist_ok=True)
    out(dict(complete=False, blobs=[n for n in os.listdir(f"{ROOT}/blobs") if not n.endswith(".tmp")]))
'''

# The blobs just sent checked and moved into place, the bundle linked; then the worker made ready in turn - set up
# (`LINUX_SETUP`), the source built (`LINUX_BUILD`), each started detached and reported until it is done - and the
# binaries linked into the bundle's `bin/`, its manifest last.
_LINUX_ASSEMBLE = r'''
for sha in A["check"]:
    t = f"{ROOT}/blobs/{sha}.tmp"
    h = hashlib.sha256()
    with open(t, "rb") as f:
        while chunk := f.read(1 << 22):
            h.update(chunk)
    if h.hexdigest() != sha:
        remove(t)
        sys.exit(f"blob {sha} arrived as {h.hexdigest()}")
    os.replace(t, f"{ROOT}/blobs/{sha}")
b = f"{ROOT}/bundle/{A['hash']}"
for rel, m in A["manifest"].items():
    dst = f"{b}/{rel}"
    os.makedirs(os.path.dirname(dst), exist_ok=True)
    remove(dst)
    os.link(f"{ROOT}/blobs/{m['sha256']}", dst)


def started(script, argv, pidfile, log):
    put(script, A[os.path.basename(script).split(".")[0]])
    p = detached(["bash", script, *argv], ROOT, log)
    put(pidfile, json.dumps(who(p.pid)))


if not os.path.exists(f"{ROOT}/env.json"):
    if not running(f"{ROOT}/setup.pid.json"):
        before = tail(f"{ROOT}/setup.log")
        started(f"{ROOT}/setup.sh", [ROOT], f"{ROOT}/setup.pid.json", f"{ROOT}/setup.log")
        out(dict(state="setting up", note=f"started (the last setup ended: {before})" if before else "started"))
    else:
        out(dict(state="setting up", note=tail(f"{ROOT}/setup.log")))
    sys.exit(0)
s = A["source"]
built = f"{ROOT}/builds/{s}"
if not os.path.exists(f"{built}/done"):
    os.makedirs(built, exist_ok=True)
    if not running(f"{built}/pid.json"):
        before = tail(f"{built}/build.log", 3)
        started(f"{ROOT}/build.sh", [ROOT, s], f"{built}/pid.json", f"{built}/build.log")
        out(dict(state="building", note=f"started (the last build ended: {before})" if before else "started"))
    else:
        out(dict(state="building", note=tail(f"{built}/build.log")))
    sys.exit(0)
for n in A["binaries"]:
    remove(f"{b}/bin/{n}")
    os.makedirs(f"{b}/bin", exist_ok=True)
    os.link(f"{built}/{n}", f"{b}/bin/{n}")
    os.chmod(f"{b}/bin/{n}", 0o755)
put(f"{b}/MANIFEST.json", json.dumps(A["manifest"], indent=1))
out(dict(state="ready"))
'''

_LINUX_LAUNCH = r'''
d = A["d"]
os.makedirs(f"{d}/selfplay", exist_ok=True)
put(f"{d}/run.py", A["run"])
put(f"{d}/grant.txt", A["grant"])
put(f"{d}/heartbeat", "")
remove(f"{d}/selfplay/{A['stop_file']}")
remove(f"{d}/exit.json")
try:
    gpu = subprocess.run(["nvidia-smi", "--query-gpu=name,driver_version", "--format=csv,noheader"], capture_output=True,
                         text=True, timeout=30).stdout.strip().splitlines()[0]
except (OSError, IndexError, subprocess.SubprocessError):
    gpu = ""
# job.json last: an attempt directory with one was launched (or is being launched); one without never started.
put(f"{d}/job.json", json.dumps(A["job"], indent=1))
p = detached([sys.executable, f"{d}/run.py"], d, f"{d}/supervisor.log")
out(dict(supervisor=who(p.pid), gpu=gpu))
'''

_LINUX_POLL = r'''
d = A["d"]
job = os.path.exists(f"{d}/job.json")
if job:
    try:
        os.utime(f"{d}/heartbeat")
    except OSError:
        put(f"{d}/heartbeat", "")
for n in A["deletes"]:
    remove(f"{d}/selfplay/{n}")
pids = read(f"{d}/pids.json")
alive = known = False
if pids:
    known = True
    try:
        p = json.loads(pids)
    except ValueError:
        p = {}
    alive = same(p.get("supervisor")) or same(p.get("selfplay"))
elif (A["launched"] or {}).get("pid"):
    known = True
    alive = same(A["launched"])
ex = read(f"{d}/exit.json")
out(dict(job=job, known=known, status=read(f"{d}/selfplay/{A['status']}"), exit=ex, pids=pids, alive=alive,
         shards=sorted(os.path.basename(f) for f in glob.glob(f"{d}/selfplay/*.shard.zst")),
         runaways=read(f"{d}/selfplay/{A['runaways']}"), grant=read(f"{d}/grant.txt"),
         report=read(f"{d}/selfplay/selfplay-report.json") if ex else None))
'''

_LINUX_KILL = r'''
d = A["d"]
if os.path.isdir(f"{d}/selfplay"):
    put(f"{d}/selfplay/{A['stop_file']}", "now")
try:
    p = json.loads(read(f"{d}/pids.json") or "null")
except ValueError:
    p = None
for w in ([p.get("supervisor"), p.get("selfplay"), p.get("infer")] if p else [A["launched"]]):
    kill(w)
out({})
'''

_LINUX_RECOVER = r'''
d = A["d"]
if os.path.exists(f"{d}/job.json"):
    job = json.loads(read(f"{d}/job.json"))
    env = dict(os.environ, **(job.get("env") or {}))
    with open(f"{d}/recover.log", "wb") as log:
        code = subprocess.run([job["selfplay"], *job["recover_args"]], cwd=d, stdin=subprocess.DEVNULL, stdout=log,
                              stderr=subprocess.STDOUT, env=env).returncode
    if code != 0:
        sys.exit(f"cantrip-selfplay --recover exited {code} (recover.log)")
out({})
'''


class LinuxSshHost(SshHost):
    """A Linux worker over SSH (a rented GPU container: bash and root, python3 installed if missing), with the same
    tree under its `root` as a Windows worker's (`SshHost`) and the same attempt files, its scripts Python piped to
    `python3 -`. Its bundle carries the engine's source in place of binaries and libraries (`linux_bundle_files`):
    the worker sets itself up once (`LINUX_SETUP`: Rust, onnxruntime-gpu and CUDA's libraries; `env.json`) and builds
    each source once (`LINUX_BUILD`: `builds/<sha>/`), both detached, and until they are done `prepare` says it is not
    ready - so the coordinator's other hosts play on and it asks again later - then the binaries are linked into the
    bundle's `bin/` and its manifest written."""

    def py(self, script: str, timeout: float = 120, **args) -> str:
        """Runs a Python script on the worker (after `_PY_PRELUDE`, `args` its `A`); its stdout."""
        args = dict(args, root=self.root)
        text = _PY_PRELUDE.replace("@ARGS@", b64(json.dumps(args))) + script
        cmd = self.ssh_base() + ["command -v python3 >/dev/null || { apt-get -qq update && apt-get -qq install -y python3; } "
                                 ">/dev/null 2>&1; exec python3 -"]
        try:
            r = self._run(cmd, input=text.encode("utf-8"), stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=timeout)
        except subprocess.TimeoutExpired as e:
            raise Unreachable(f"{self.name}: no answer in {timeout:.0f} s") from e
        except OSError as e:
            raise Unreachable(f"{self.name}: {e}") from e
        if r.returncode != 0:
            raise Unreachable(f"{self.name}: exit {r.returncode}: {r.stderr.decode(errors='replace').strip()[-600:]}")
        return r.stdout.decode(errors="replace")

    def py_json(self, script: str, timeout: float = 120, **args):
        # The last line: a login banner may come first.
        out = self.py(script, timeout, **args).strip()
        try:
            return json.loads(out.splitlines()[-1]) if out else None
        except ValueError as e:
            raise Unreachable(f"{self.name}: not JSON: {out[-300:]}") from e

    def source(self) -> str:
        return self.manifest["src.tar.gz"]["sha256"]

    def prepare(self, live: list[dict]):
        """Its orphans stopped and old bundles, blobs and builds removed first (a worker still building is pruned
        too), then the bundle pushed and the worker readied (`push`)."""
        self.py(_LINUX_PRUNE, timeout=600, tag=self.tag, name=self.name, keep=[self.attempt_dir(a) for a in live],
                hash=self.hash, want=sorted({m["sha256"] for m in self.manifest.values()}), source=self.source())
        self.push()

    def push(self):
        if self.pushed:
            return
        have = self.py_json(_LINUX_HAVE, hash=self.hash)
        if not have.get("complete"):
            blobs = set(have.get("blobs") or [])
            want = {m["sha256"]: self.files[rel] for rel, m in self.manifest.items() if m["sha256"] not in blobs}
            if want:
                size = sum(p.stat().st_size for p in want.values())
                self.log(f"  [{self.name}] pushing {len(want)} files ({size / 2**20:.0f} MB) of bundle {self.hash}; "
                         f"{len(self.manifest) - len(want)} there already")
            for sha, p in want.items():
                self.scp(str(p), self.remote_path(f"blobs/{sha}.tmp"))
            r = self.py_json(_LINUX_ASSEMBLE, timeout=1800, hash=self.hash, manifest=self.manifest, check=sorted(want),
                             source=self.source(), binaries=list(LINUX_BINARIES), setup=LINUX_SETUP, build=LINUX_BUILD)
            if r.get("state") != "ready":
                raise Unreachable(f"{self.name}: {r.get('state')}: {r.get('note') or '...'}")
            self.log(f"  [{self.name}] bundle {self.hash} ready (built from source {self.source()[:16]})")
        self.pushed = True

    def launch(self, a: dict, grant: str) -> dict:
        self.push()
        d = self.attempt_dir(a)
        job = self.job(self, a, d)
        job["selfplay_args"] = list(job["selfplay_args"]) + ["--lease", f"{d}/heartbeat", "--lease-seconds", str(self.lease)]
        job["env_file"] = f"{self.root}/env.json"
        # The box's cores (`pin_plan`): pinned unless the worker says `"pin": false`, and the lanes `LANES_PER_CORE`
        # a self-play core unless it names its `lanes`. mimalloc keeps what a search frees instead of returning it to
        # the system and faulting it back (each full search's tree: ~250,000 faults a second over a box, 9% of the CPU).
        job["pin"] = self.w.get("pin", True)
        job["lanes_per_core"] = None if self.w.get("lanes") else self.w.get("lanes_per_core", LANES_PER_CORE)
        job["env"] = {**(job.get("env") or {}), "MIMALLOC_PURGE_DELAY": "-1"}
        r = self.py_json(_LINUX_LAUNCH, d=d, job=job, grant=grant, run=RUN_PY, stop_file=STOP_FILE)
        self.gpu = (r.get("gpu") or "").strip() or None
        return dict(launched=dict(supervisor=r.get("supervisor")), remote_dir=d, gpu=self.gpu, **(job.get("record") or {}))

    def grant(self, a: dict, text: str):
        self.py('put(f"{A[\'d\']}/grant.txt", A["text"])\n', d=self.attempt_dir(a), text=text)

    def poll(self, a: dict) -> Poll:
        r = self.py_json(_LINUX_POLL, d=self.attempt_dir(a), deletes=list(a.get("to_delete") or []),
                         launched=((a.get("launched") or {}).get("supervisor")) or {}, status=STATUS, runaways=RUNAWAYS)
        return poll_from(r, a)

    def delete(self, a: dict, names: list[str]):
        self.py('for n in A["names"]:\n    remove(f"{A[\'d\']}/selfplay/{n}")\n', d=self.attempt_dir(a), names=list(names))

    def stop(self, a: dict, now: bool):
        self.py('if os.path.isdir(f"{A[\'d\']}/selfplay"):\n    put(f"{A[\'d\']}/selfplay/{A[\'stop_file\']}", A["text"])\n',
                d=self.attempt_dir(a), stop_file=STOP_FILE, text="now" if now else "")

    def kill(self, a: dict):
        self.py(_LINUX_KILL, d=self.attempt_dir(a), stop_file=STOP_FILE,
                launched=((a.get("launched") or {}).get("supervisor")) or {})

    def recover(self, a: dict):
        self.py(_LINUX_RECOVER, timeout=1800, d=self.attempt_dir(a))


def host_class(worker: dict):
    """A worker's host: `"os": "linux"` a `LinuxSshHost`, a Windows one (the default) an `SshHost`."""
    return LinuxSshHost if worker.get("os") == "linux" else SshHost


def judge_worker(workers: list[dict]) -> dict | None:
    """The worker pair mining judges on: the first Windows one (`mining.RemoteJudge` is PowerShell); none, and it
    judges on this machine."""
    return next((w for w in workers if w.get("os", "windows") == "windows"), None)


def poll_from(r: dict, a: dict) -> Poll:
    """A worker's poll script's answer (`job`, `known`, `alive`, `pids`, `shards`, and the raw texts of its status,
    exit, runaways, report and grant files), read: the same for a Windows worker's PowerShell and a Linux one's
    Python."""
    def parsed(text):
        if isinstance(text, dict):  # a string PowerShell decorated with its provider's properties
            text = text.get("value")
        try:
            return json.loads(text) if text else None
        except ValueError:
            return None

    shards = r.get("shards") or []
    if isinstance(shards, str):
        shards = [shards]
    ex = parsed(r.get("exit"))
    runaways = []
    runaway_text = r.get("runaways")
    if isinstance(runaway_text, dict):
        runaway_text = runaway_text.get("value")
    for line in (runaway_text or "").splitlines():
        v = parsed(line.strip())
        if v and "index" in v:
            runaways.append(int(v["index"]))
    pids = parsed(r.get("pids")) or {}
    if pids:
        a["pids"] = pids
    grant = r.get("grant")
    if isinstance(grant, dict):
        grant = grant.get("value")
    if not r.get("job"):
        started = False  # nothing was started: job.json is written just before the process
    elif r.get("known") or ex is not None:
        started = True
    else:
        # Being launched this moment, or launched by a coordinator that stopped before it recorded the process:
        # pids.json comes within seconds; until the grace is over it is not given up.
        started = None
    return Poll(status=parsed(r.get("status")), exit=None if ex is None else int(ex["code"]),
                alive=bool(r.get("alive")), shards=sorted(shards), runaways=runaways, report=parsed(r.get("report")),
                started=started, grant=(grant or "").replace("\r\n", "\n") if r.get("job") else None)


def root_tag(root: Path) -> str:
    """A generation root's name on a worker: its directory's name and a hash of its path, so two roots (a scratch
    one and the real one) never share a worker's directories."""
    return f"{root.name}-{hashlib.sha256(str(root.resolve()).lower().encode()).hexdigest()[:6]}"


def local_name() -> str:
    return socket.gethostname().lower()
