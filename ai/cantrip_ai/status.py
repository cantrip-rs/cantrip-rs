"""Whether a generation is alive (docs/ai-training.md, "Watching a run"): the driver keeps `ROOT/genN/status.json` up
to date while it works, and this reads it and says running, stalled, dead, failed, stopped or finished.

    python -m cantrip_ai.status --root ROOT [--generation N] [--json] [--tail 15] [--stall-factor 0.25] [--stall-floor 900]

The file (`Heartbeat`, a thread of the driver's): the stage and when it began, the driver's pid and host, how long
the stage took in the generations before (`expected`, from their stage records in `generation.json`), a progress
counter and when it last moved, and when the file was last written. The counter is read from what the stages already
write, never from the words of a log: the games in the generation's distributed ledgers (self-play's and each
duel's, `distributed.Ledger`), the closed shards of its self-play, the bytes of its stage logs (`genN/logs`, the
inference servers' aside: they speak whether or not a game moves) and the files of its training directory.

What the reading says (`judge`):

- `finished`, `failed` (with the error; `transient` names a failure the supervisor may retry, `TRANSIENT`),
  `stopped` (asked to: Ctrl-C, a stop file): the driver's own last word.
- `dead`: the file says running and its process is gone - the driver was killed, or its machine restarted.
- `stalled`: the process lives and the counter has not moved for `--stall-factor` of the stage's expected time (at
  least `--stall-floor` seconds; with no earlier generation to go by, `UNKNOWN_STALL`), or the file itself has not been
  written for `SILENT` seconds (the driver hangs).
- `running` otherwise, `overdue` noted once the stage has run `OVERDUE` times its expected time.

The exit code: 0 running or finished, 1 failed, 2 stalled, 3 dead, 4 stopped, 5 nothing to read. No torch and no
numpy: the driver and the supervisor import this.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import socket
import statistics
import sys
import threading
import time
from pathlib import Path

from . import distributed

STATUS = "status.json"
LOOP = "loop.json"
# How often the driver writes the file.
BEAT = 15.0
# A driver whose file has not been written for this long hangs (its thread writes every `BEAT`).
SILENT = distributed.SILENT
# No progress for this share of the stage's expected time is a stall, at least the floor; with nothing to expect, this.
STALL_FACTOR = 0.25
STALL_FLOOR = 900.0
UNKNOWN_STALL = 3600.0
OVERDUE = 2.0
EXIT = dict(running=0, finished=0, failed=1, stalled=2, dead=3, stopped=4, none=5)

# Failures the supervisor may run a generation again after (`loop.py`), by what the driver's error says; anything
# else is a failure for a person. Self-play's governor stopping on memory leaves its games in closed shards and a
# rerun resumes it; a stop that was asked for (a stop file, an interrupt) is not one of these.
TRANSIENT = (
    ("selfplay-memory", re.compile(r"stopped by its governor \(memory under the floor")),
    ("server-start", re.compile(r"the inference server did not start")),
    ("ledger-locked", re.compile(r"another driver \(pid \d+\) is running")),
)


# A stop somebody asked for (`generation.py --stop`, Ctrl-C): no failure, and nothing to run again unasked.
ASKED = re.compile(r"stopped by its governor \((stop file|interrupted)|stopped on every host")


def asked_to_stop(error: str) -> bool:
    return bool(ASKED.search(error or ""))


# What a server or the parity test says when a model served is outside the tolerance: the server's own check of each
# model's references as it starts (cantrip-infer, serve.py), and the Rust test's assertion on the served ("remote")
# evaluator. Only this makes the parity check fall back to the next serving option (`generation.serving_check`); and
# it is never transient: the same models served the same way fail the same way.
PARITY_FAILED = re.compile(r"references FAIL|is not its references|^remote: Parity \{", re.M)


def transient(error: str) -> str | None:
    # A parity failure, wherever it shows - the parity stage finding no option, or a later server (the gate's)
    # refusing to start on a model that is not its references - quotes "the inference server did not start", and is
    # not that: running it again changes nothing.
    if "no serving option passes" in (error or "") or PARITY_FAILED.search(error or ""):
        return None
    for name, pattern in TRANSIENT:
        if pattern.search(error or ""):
            return name
    return None


def write_atomic(path: Path, data: dict, seconds: float = 0.25) -> bool:
    """`data` written whole and renamed into place, tried for `seconds` (on Windows a reader that has the file open
    this instant refuses the rename). Whether it was written: a status that could not be written never fails a run
    or a supervisor's step, so nothing here raises."""
    tmp = path.with_name(path.name + f".{os.getpid()}.tmp")
    try:
        tmp.write_text(json.dumps(data, indent=1, default=str), encoding="utf-8")
        deadline = time.monotonic() + seconds
        while True:
            try:
                os.replace(tmp, path)
                return True
            except PermissionError:
                if time.monotonic() >= deadline:
                    break
                time.sleep(0.05)
    except (OSError, TypeError, ValueError):
        pass
    try:
        tmp.unlink(missing_ok=True)
    except OSError:
        pass
    return False


def read_json(path: Path):
    for _ in range(3):
        try:
            return json.loads(path.read_text(encoding="utf-8"))
        except FileNotFoundError:
            return None
        except (OSError, ValueError):
            time.sleep(0.05)
    return None


def alive(record: dict | None, now: float | None = None) -> bool:
    """Whether the process a status or loop file names (`pid`, `pid_start`) is still that process
    (`distributed.process_alive`: a reused pid is another process; one that may not be asked about is alive, unless
    the file's own `updated` - a status file's heartbeat - is older than `SILENT`)."""
    return bool(record) and distributed.process_alive(record.get("pid"), record.get("pid_start"), record.get("updated"), now)


def process_table() -> list[tuple[int, str]] | None:
    """Every process's id and command line, or None where they cannot be had."""
    try:
        if os.name == "nt":
            import subprocess  # noqa: PLC0415

            p = subprocess.run(["powershell", "-NoProfile", "-NonInteractive", "-Command",
                                "Get-CimInstance Win32_Process | Select-Object ProcessId,CommandLine | ConvertTo-Json -Compress"],
                               capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=60,
                               stdin=subprocess.DEVNULL)
            rows = json.loads(p.stdout) if p.returncode == 0 and p.stdout.strip() else None
            if rows is None:
                return None
            rows = [rows] if isinstance(rows, dict) else rows
            return [(int(r["ProcessId"]), r.get("CommandLine") or "") for r in rows]
        import subprocess  # noqa: PLC0415

        p = subprocess.run(["ps", "-eo", "pid=,args="], capture_output=True, text=True, encoding="utf-8",
                           errors="replace", timeout=60, stdin=subprocess.DEVNULL)
        if p.returncode != 0:
            return None
        out = []
        for line in p.stdout.splitlines():
            pid, _, args = line.strip().partition(" ")
            if pid.isdigit():
                out.append((int(pid), args))
        return out
    except Exception:  # noqa: BLE001 (no table: the caller says so and goes by the status files)
        return None


DRIVER_ROOT = re.compile(r"""--root(?:=|\s+)(?:"([^"]+)"|'([^']+)'|(\S+))""")


def driver_processes(root: Path, table=None, cwd: Path | None = None) -> list[int] | None:
    """The processes running the training driver (`cantrip_ai.generation`) on `root` now, by the process table: a
    driver from before the status file writes none, and is found here. A dry run, a preflight and a `--stop` are not
    drivers; this process is not counted. None where the table cannot be read."""
    table = process_table() if table is None else table
    if table is None:
        return None

    def same(text: str) -> bool:
        path = Path(text)
        if not path.is_absolute():
            path = (cwd or Path.cwd()) / path
        return os.path.normcase(os.path.abspath(path)) == os.path.normcase(os.path.abspath(root))

    found = []
    for pid, line in table:
        if pid == os.getpid() or "cantrip_ai.generation" not in line:
            continue
        if any(flag in line.split() for flag in ("--dry-run", "--preflight", "--stop", "--stop-now")):
            continue
        m = DRIVER_ROOT.search(line)
        if m and same(next(g for g in m.groups() if g)):
            found.append(pid)
    return found


# --- What the driver has done -------------------------------------------------------------------------------------


def measure(gen_dir: Path, model_dir: Path | None = None) -> dict:
    """The generation's progress as numbers that only grow while it works: the games its distributed ledgers hold, its
    self-play's closed shards, the bytes of its stage logs, and its training directory's files (their count and
    newest time)."""
    ledgers = 0
    dist = gen_dir / "dist"
    if dist.is_dir():
        for path in [dist / "ledger.json", *dist.glob("*/ledger.json")]:
            d = read_json(path) if path.exists() else None
            if isinstance(d, dict):
                # A shard's `indices` are half-open ranges (`distributed.to_ranges`): its games are counted.
                for shard in (d.get("shards") or {}).values():
                    for r in shard.get("indices") or []:
                        ledgers += (r[1] - r[0]) if isinstance(r, list) and len(r) == 2 else 1
                ledgers += len(d.get("attempts") or {})
    shards = 0
    selfplay = gen_dir / "selfplay"
    if selfplay.is_dir():
        with os.scandir(selfplay) as it:
            shards = sum(1 for e in it if e.name.endswith(".shard.zst"))
    logs = 0
    log_dir = gen_dir / "logs"
    if log_dir.is_dir():
        with os.scandir(log_dir) as it:
            for e in it:
                if e.is_file() and not e.name.startswith("server-"):
                    try:
                        # Asked of the file, not the directory entry: Windows keeps an open file's entry at the size
                        # it had when it was last closed, and a stage's log is open for as long as the stage runs.
                        logs += os.stat(e.path).st_size
                    except OSError:
                        pass
    training = 0
    train = model_dir / "train" if model_dir is not None else None
    if train is not None and train.is_dir():
        with os.scandir(train) as it:
            for e in it:
                try:
                    st = os.stat(e.path)
                    training += (int(st.st_mtime) % 10_000_000) + st.st_size % 1_000_003
                except OSError:
                    pass
    return dict(ledger_games=ledgers, selfplay_shards=shards, log_bytes=logs, training=training)


def expected(root: Path, gen: int, stage: str, last: int = 5) -> float | None:
    """How long `stage` took in the generations before `gen` (the median of the last `last` that finished it), from
    their stage records; None with none."""
    seconds = []
    for n in range(gen - 1, 0, -1):
        d = read_json(root / f"gen{n}" / "generation.json")
        s = ((d or {}).get("stages") or {}).get(stage) or {}
        # Only a stage that ran from its start to its end in one go: a resumed stage's seconds are its last part's.
        whole = s.get("runs", 1) == 1 and not s.get("resumed") and s.get("init") != "resumed"
        if s.get("done") and whole and isinstance(s.get("seconds"), (int, float)):
            seconds.append(float(s["seconds"]))
            if len(seconds) >= last:
                break
    return statistics.median(seconds) if seconds else None


class Heartbeat:
    """The driver's status file, kept by a thread of its own so it is written while a stage's process is waited for.
    `begin` at each stage; `close` with the driver's last word."""

    def __init__(self, root: Path, gen: int, model_dir: Path | None = None, interval: float = BEAT, argv=None,
                 clock=time.time):
        self.root, self.gen, self.model_dir, self.interval, self.clock = root, gen, model_dir, interval, clock
        self.gen_dir = root / f"gen{gen}"
        self.path = self.gen_dir / STATUS
        now = clock()
        self.data = dict(generation=gen, state="running", stage=None, pid=os.getpid(),
                         pid_start=distributed.process_start(os.getpid()), host=socket.gethostname(),
                         started=now, stage_started=now, expected_seconds=None, progress=0, progress_at=now,
                         progress_detail={}, updated=now, error=None, transient=None, warnings=[],
                         argv=list(argv) if argv is not None else None,
                         driver_log=os.environ.get("CANTRIP_AI_DRIVER_LOG"))
        self.lock = threading.Lock()
        self.stop = threading.Event()
        self.thread = None

    def start(self):
        try:
            self.gen_dir.mkdir(parents=True, exist_ok=True)
            self.beat()
        except Exception:  # noqa: BLE001 (the status never fails a run)
            pass
        self.thread = threading.Thread(target=self._run, name="status", daemon=True)
        self.thread.start()
        return self

    def _run(self):
        while not self.stop.wait(self.interval):
            try:
                self.beat()
            except Exception:  # noqa: BLE001 (the status never fails a run)
                pass

    def beat(self):
        detail = measure(self.gen_dir, self.model_dir)
        with self.lock:
            now = self.clock()
            if detail != self.data["progress_detail"]:
                self.data.update(progress=self.data["progress"] + 1, progress_at=now, progress_detail=detail)
            self.data["updated"] = now
            write_atomic(self.path, self.data)

    def begin(self, stage: str):
        with self.lock:
            now = self.clock()
            self.data.update(stage=stage, stage_started=now, progress_at=now)
        try:
            with self.lock:
                self.data["expected_seconds"] = expected(self.root, self.gen, stage)
            self.beat()
        except Exception:  # noqa: BLE001 (the status never fails a run)
            pass

    def warn(self, text: str):
        with self.lock:
            self.data["warnings"].append(text)

    def close(self, state: str, error: str | None = None):
        """The driver's last word: `finished`, `failed` (with why), `stopped`, or `idle` (a part of a generation done,
        as `--stop-after` asks)."""
        self.stop.set()
        if self.thread is not None:
            self.thread.join(timeout=5)
        with self.lock:
            self.data.update(state=state, error=error[-4000:] if error else None,
                             transient=transient(error) if error else None, updated=self.clock())
            # The last word matters most (without it a failure reads as a dead driver): tried for a while.
            write_atomic(self.path, self.data, seconds=10.0)


# --- Reading it ---------------------------------------------------------------------------------------------------


def latest_generation(root: Path) -> int | None:
    gens = [int(m.group(1)) for p in root.glob("gen*") if (m := re.fullmatch(r"gen(\d+)", p.name)) and (p / STATUS).exists()]
    return max(gens) if gens else None


def read(root: Path, gen: int | None = None) -> dict | None:
    gen = gen if gen is not None else latest_generation(root)
    return read_json(root / f"gen{gen}" / STATUS) if gen is not None else None


def judge(s: dict | None, now: float | None = None, stall_factor: float = STALL_FACTOR, stall_floor: float = STALL_FLOOR,
          alive=alive) -> dict:
    """What a status file says now: `state` (running, stalled, dead, failed, stopped, finished, none) and `why`, with
    `overdue` where a running stage has taken far longer than it did before."""
    if not s:
        return dict(state="none", why="no status file: no generation has run under this root with a driver that writes one")
    now = time.time() if now is None else now
    if s.get("state") in ("finished", "idle"):
        return dict(state="finished", why="the generation finished" if s["state"] == "finished"
                    else f"the driver did what it was asked and stopped after {s.get('stage')}")
    if s.get("state") == "failed":
        first = (s.get("error") or "").strip().splitlines()[:1]
        return dict(state="failed", why=f"stage {s.get('stage')} failed: {first[0] if first else 'no error recorded'}"
                    + (f" (transient: {s['transient']})" if s.get("transient") else ""))
    if s.get("state") == "stopped":
        return dict(state="stopped", why=f"stopped at stage {s.get('stage')}, as asked (a rerun resumes it)")
    here = s.get("host") == socket.gethostname()
    def at(key: str, default: float) -> float:
        return default if s.get(key) is None else s[key]

    silent = now - at("updated", 0.0)
    if here and not alive(s):
        return dict(state="dead", why=f"the driver (pid {s.get('pid')}) is gone and left no final word; it was in stage "
                                      f"{s.get('stage')}, last heard {silent / 60:.0f} min ago")
    if silent > SILENT:
        return dict(state="dead" if not here else "stalled",
                    why=f"the driver (pid {s.get('pid')} on {s.get('host')}) has not written its status for "
                        f"{silent / 60:.0f} min" + ("" if here else "; it runs on another machine, so its process cannot be seen from here"))
    exp = s.get("expected_seconds")
    limit = max(stall_floor, stall_factor * exp) if exp else max(stall_floor, UNKNOWN_STALL)
    quiet = now - at("progress_at", now)
    elapsed = now - at("stage_started", now)
    took = f"{elapsed / 60:.0f} min in" + (f", {exp / 60:.0f} min expected" if exp else ", no earlier generation to go by")
    if quiet > limit:
        return dict(state="stalled", why=f"stage {s.get('stage')} has made no progress for {quiet / 60:.0f} min "
                                         f"(the limit is {limit / 60:.0f} min; {took})")
    out = dict(state="running", why=f"stage {s.get('stage')}, {took}; progress {quiet / 60:.1f} min ago")
    if exp and elapsed > OVERDUE * exp:
        out["overdue"] = f"stage {s.get('stage')} has run {elapsed / exp:.1f} times its expected {exp / 60:.0f} min"
    return out


def tail(path: Path, lines: int = 15) -> list[str]:
    try:
        with path.open("rb") as f:
            f.seek(0, os.SEEK_END)
            f.seek(max(0, f.tell() - 16384))
            return f.read().decode("utf-8", errors="replace").splitlines()[-lines:]
    except OSError:
        return []


def stage_log(root: Path, gen: int) -> Path | None:
    """The log the generation's running stage writes: the newest of `genN/logs` that is no inference server's."""
    logs = [p for p in (root / f"gen{gen}" / "logs").glob("*.log") if not p.name.startswith("server-")]
    return max(logs, key=lambda p: p.stat().st_mtime) if logs else None


def report(root: Path, gen: int | None = None, lines: int = 15, **limits) -> dict:
    """Everything `status` shows: the file, the judgement, the loop's own state and the tails of the stage's log and
    the driver's."""
    s = read(root, gen)
    out = dict(root=str(root), status=s, **judge(s, **limits))
    loop = read_json(root / LOOP)
    if loop:
        out["loop"] = dict(loop, alive=alive(loop) if loop.get("host") == socket.gethostname() else None)
    if s:
        log = stage_log(root, s["generation"])
        out["log"] = dict(path=str(log), tail=tail(log, lines)) if log else None
        if s.get("driver_log"):
            out["driver_log"] = dict(path=s["driver_log"], tail=tail(Path(s["driver_log"]), lines))
    return out


def main(argv=None):
    a = argparse.ArgumentParser(description="Whether a generation is running, stalled, dead, failed or finished.")
    a.add_argument("--root", required=True, help="the ai-data directory")
    a.add_argument("--generation", type=int, help="default: the newest with a status file")
    a.add_argument("--json", action="store_true")
    a.add_argument("--tail", type=int, default=15, help="lines of the stage's log shown")
    a.add_argument("--stall-factor", type=float, default=STALL_FACTOR,
                   help="no progress for this share of the stage's expected time is a stall")
    a.add_argument("--stall-floor", type=float, default=STALL_FLOOR, metavar="S", help="and for at least this long")
    args = a.parse_args(argv)
    r = report(Path(args.root).resolve(), args.generation, args.tail, stall_factor=args.stall_factor,
               stall_floor=args.stall_floor)
    if args.json:
        print(json.dumps(r, indent=1))
        return EXIT[r["state"]]
    s = r["status"] or {}
    print(f"gen{s.get('generation', '?')}: {r['state'].upper()} - {r['why']}")
    if r.get("overdue"):
        print(f"  overdue: {r['overdue']}")
    for w in s.get("warnings") or []:
        print(f"  warning: {w}")
    if s.get("state") == "failed" and s.get("error"):
        print("  error:")
        for line in s["error"].strip().splitlines()[-12:]:
            print(f"    {line}")
    if loop := r.get("loop"):
        dead = " (ITS PROCESS IS GONE)" if loop.get("alive") is False and loop.get("state") not in ("stopped", "failed") else ""
        print(f"  loop: {loop.get('state')}{dead} - {loop.get('why', '')} (pid {loop.get('pid')}, since {loop.get('since')})")
    for name in ("log", "driver_log"):
        if r.get(name) and r[name]["tail"]:
            print(f"  --- {r[name]['path']} ---")
            for line in r[name]["tail"]:
                print(f"  {line}")
    return EXIT[r["state"]]


if __name__ == "__main__":
    sys.exit(main())
