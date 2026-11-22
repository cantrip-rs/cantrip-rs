"""Whether a generation is alive (docs/ai-training.md, "Watching a run"): the driver keeps `ROOT/genN/status.json` up
to date while it works, and this reads it and says running, stalled, overdue by projection, dead, failed, stopped or
finished.

    python -m cantrip_ai.status --root ROOT [--generation N] [--json] [--tail 15] [--stall-factor 0.25] [--stall-floor 900]
                                [--overrun-factor 2] [--overrun-window 1800] [--expect STAGE=HOURS] [--allow-slow STAGE]

The file (`Heartbeat`, a thread of the driver's): the stage and when it began, the driver's pid and host, how long
the stage took in the generations before (`expected`, from their stage records in `generation.json`), a progress
counter and when it last moved, and when the file was last written. The counter is read from what the stages already
write, never from the words of a log: the games in the generation's distributed ledgers (self-play's and each
duel's, `distributed.Ledger`), what their coordinators last saw of each attempt (below), the closed shards of its
self-play, the bytes of its stage logs (`genN/logs`, the inference servers' aside: they speak whether or not a game
moves) and the files of its training directory.

A part granted whole to a host (a judgement's, a replay's) returns nothing until it is over, so a ledger's games alone
would call a long part a stall. Each poll that finds an attempt's processes alive is written in its ledger record
(`distributed.Coordinator.check`): `progress`, the games its status file counts, and `heard`, the time of that poll.
An attempt with a `progress` is counted by it (a run that lives and plays nothing is still a stall); one without (a
kind whose runs write no status, a run that has written none yet) by its `heard` and, on this machine, its lease file
(`<attempt>.heartbeat`, touched at each poll): alive is all that is known of it, and it stalls once its coordinator
stops hearing from it.

What the reading says (`judge`):

- `finished`, `failed` (with the error; `transient` names a failure the supervisor may retry, `TRANSIENT`),
  `stopped` (asked to: Ctrl-C, a stop file): the driver's own last word.
- `dead`: the file says running and its process is gone - the driver was killed, or its machine restarted.
- `stalled`: the process lives and the counter has not moved for `--stall-factor` of the stage's expected time (at
  least `--stall-floor` seconds; with no earlier generation to go by, `UNKNOWN_STALL`), or the file itself has not been
  written for `SILENT` seconds (the driver hangs).
- `overdue-projected`: the process lives, the stage moves, and at the rate it moves it will take `--overrun-factor`
  times what it should (below). A run alive at a twentieth of its speed is neither stalled nor dead; this is it.
- `running` otherwise, `overdue` noted once the stage has run `OVERDUE` times its expected time.

Where a stage is going (`Heartbeat.work`, `project`): a stage that plays games through a distributed ledger
(self-play, each duel, the judgements) has a count and a total, so its end can be projected from its rate: the games
the ledger accounts for, those its live attempts count finished on their hosts and have not sent yet (the `status`
their coordinator keeps in the ledger), half of each game in flight, and - a judgement's part is granted whole and
says nothing until it is over - the share of its part a live attempt's time so far is (`ledger_work`). The rate is
the last window's (`project`), and never taken from fewer than three games, or three parts' worth where parts are
granted whole. What is expected is what the hosts playing now made before (`pace`'s `rates`, `expectation`); the
factor is `WIDE` times wider where one of them has no record. The file carries that (`work`: the ledger,
done and total, where it stood when this run of the stage first saw it, a sample a minute) and what the same work took
before (`pace`: seconds a game in the earlier generations whose ledger of the same name was made with the same
settings - `pace_settings`, the ledger's identity less what differs every generation - and played on the same hosts,
each with the same backend and precision).
After `--overrun-window` seconds (30 min: the lanes are still growing before) the projection is in the reading
(`projection`); it is `overdue-projected` when it passes `--overrun-factor` (2) times the expectation - the earlier
generations' pace over this run's games, or an owner's `--expect STAGE=HOURS` (a stage with no rate yet is projected
at the time it has spent). With no generation of these settings (searches four times deeper, other workers) no stage
runs unheld for that: it is held against the last generation that played the ledger at all, whatever its settings,
over this run's games (failing that, the stage's usual time), and the supervisor also tells the projection once
(`projected`), since what it is held against is then a guess. Only a stage nothing ever ran before is told and not
held. `--allow-slow STAGE`
(or the stage's name on a line of `ROOT/LOOP.allow-slow`) lets a stage be slow on purpose. The supervisor writes the
limits it watches with into `ROOT/loop.json` (`watch`), and this command reads them from there when it is not given
its own.

The exit code: 0 running or finished, 1 failed, 2 stalled, 3 dead, 4 stopped, 5 nothing to read, 6 overdue-projected.
No torch and no numpy: the driver and the supervisor import this.
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
# A stage projected, at the rate it has gone for `OVERRUN_WINDOW` seconds or more, to take this many times what it
# should is overdue by projection. A rate wants `OVERRUN_GAMES` games' worth of work behind it.
OVERRUN_FACTOR = 2.0
OVERRUN_WINDOW = 1800.0
OVERRUN_GAMES = 3.0
# Where what is expected is not what these hosts made before (a host with no record plays, or none of them has one),
# the factor is this many times itself: wider, rather than a guess of what that host adds.
WIDE = 1.5
# A whole part at work (an attempt that writes no status) counts for its time so far, up to this share of the part.
PART_CREDIT = 0.9
# The kinds of run (`distributed.Kind.name`, on each attempt's record) that are granted a part whole and write no
# status while they work.
WHOLE_KINDS = ("judge",)
# A host whose attempt ended this lately still counts among those playing: its next attempt is a moment away.
HOST_GAP = 300.0
PROJECTED = "overdue-projected"
ALLOW_SLOW = "LOOP.allow-slow"
# A sample of a stage's work is kept this often, this many of them (three hours).
SAMPLE_EVERY, SAMPLES = 60.0, 180
EXIT = {"running": 0, "finished": 0, "failed": 1, "stalled": 2, "dead": 3, "stopped": 4, "none": 5, PROJECTED: 6}

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
    """The generation's progress as numbers that move while it works: the games its distributed ledgers hold, what
    their attempts' runs were last seen to do (`attempt_progress`, the games their status files count; `attempts_heard`,
    for those that count none, when they were last found alive), its self-play's closed shards, the bytes of its stage
    logs, and its training directory's files (their count and newest time)."""
    ledgers = progress = heard = 0
    dist = gen_dir / "dist"
    if dist.is_dir():
        for path in [dist / "ledger.json", *dist.glob("*/ledger.json")]:
            d = read_json(path) if path.exists() else None
            if isinstance(d, dict):
                # A shard's `indices` are half-open ranges (`distributed.to_ranges`): its games are counted.
                for shard in (d.get("shards") or {}).values():
                    for r in shard.get("indices") or []:
                        ledgers += (r[1] - r[0]) if isinstance(r, list) and len(r) == 2 else 1
                attempts = d.get("attempts") or {}
                ledgers += len(attempts)
                for name, a in attempts.items():
                    if not isinstance(a, dict):
                        continue
                    if isinstance(a.get("progress"), (int, float)):
                        progress += int(a["progress"])
                        continue
                    # No count of its own: the last poll that found it alive, and this machine's lease file.
                    if isinstance(a.get("heard"), (int, float)):
                        heard += int(a["heard"])
                    try:
                        heard += int(os.stat(path.with_name(f"{a.get('id') or name}.heartbeat")).st_mtime)
                    except OSError:
                        pass
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
    return dict(ledger_games=ledgers, attempt_progress=progress, attempts_heard=heard, selfplay_shards=shards,
                log_bytes=logs, training=training)


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


# --- Where a stage is going ---------------------------------------------------------------------------------------

# What of a ledger's identity differs from one generation to the next whatever its settings: never part of what
# decides its pace. (A key this misses makes two generations' settings differ, and the projection is then only told.)
# A server's arguments are among them: the parity check picks each generation's serving option, and what of it
# changes the pace is a host's precision, which is kept.
VARIES = frozenset(("model", "models", "seed", "seeds", "pairings", "games", "plan", "stops", "server_args"))
HOST_PACE = ("backend", "precision")


def pace_settings(identity) -> dict | None:
    """What decides how fast a ledger's games go, from its identity (`distributed.Ledger.identity`): its settings
    less what every generation changes (`VARIES`), and the hosts that play it with each one's backend and precision."""
    if not isinstance(identity, dict) or not isinstance(identity.get("global"), dict):
        return None

    def kept(v):
        if isinstance(v, dict):
            return {k: kept(x) for k, x in sorted(v.items()) if k not in VARIES}
        return [kept(x) for x in v] if isinstance(v, list) else v

    hosts = identity.get("hosts") if isinstance(identity.get("hosts"), dict) else {}
    return dict(settings=kept(identity["global"]),
                hosts={n: {k: h[k] for k in HOST_PACE if isinstance(h, dict) and h.get(k) is not None}
                       for n, h in sorted(hosts.items())})


def _span(ranges) -> int:
    return sum((r[1] - r[0]) if isinstance(r, list) and len(r) == 2 else 1 for r in ranges or [])


def ledger_work(d: dict, now: float | None = None, rates: dict | None = None) -> dict:
    """A ledger's games as counts: `total` (those it plays), `played` (in accepted outputs, runaways and dropped),
    `skipped` (never to be played: a duel closed by its test), `finished` (played, and those its live attempts count
    finished on their hosts and have not sent yet) and `flight` (taken there and not finished).

    A judgement's part is granted whole and writes no status: it returns nothing until it is over, and a ledger of
    such parts moves only when one is delivered. With `now`, the time such a live attempt has run counts as a share
    of its part (`credit`, in games): against what a game of its took in the parts of this ledger its host has
    already delivered, else against the host's earlier record (`rates`, games a second), and never more than
    `PART_CREDIT` of the part - the rest is its delivery's. `grain` is how coarsely the ledger moves: a game, or with
    such parts their mean size. Only a part of such a kind (`WHOLE_KINDS`, by the attempt's `kind`; in a ledger from
    before attempts said theirs, one beside a part delivered with no status) that is at work or delivered is one: a
    self-play's or a duel's attempt that has written no status yet, or never started, is neither a part nor owed
    any credit. `hosts` are those with a live attempt, or (with `now`) one that ended within `HOST_GAP`."""
    total = _span(d.get("only")) if d.get("only") is not None else int(d.get("games") or 0)
    sent: dict = {}
    for s in (d.get("shards") or {}).values():
        sent[s.get("attempt")] = sent.get(s.get("attempt"), 0) + _span(s.get("indices"))
    played = sum(sent.values()) + len(d.get("runaways") or {}) + len(d.get("dropped") or {})
    skipped = len(d.get("skipped") or {})
    attempts = {(a.get("id") or name): a for name, a in (d.get("attempts") or {}).items() if isinstance(a, dict)}
    waiting = flight = 0
    credit, parts, hosts = 0.0, [], set()
    # A ledger from before attempts said their kind: one that delivered with no status shows what its attempts are.
    shown = any(not isinstance(a.get("status"), dict) and sent.get(me) for me, a in attempts.items())

    def whole(a: dict) -> bool:
        return not isinstance(a.get("status"), dict) and (a["kind"] in WHOLE_KINDS if a.get("kind") else shown)

    for me, a in attempts.items():
        st = a.get("status") if isinstance(a.get("status"), dict) else None
        alive = a.get("state") in distributed.LIVE
        if (whole(a) and (alive or sent.get(me)) and isinstance(a.get("granted_count"), (int, float))
                and a["granted_count"] > 0):
            parts.append(a["granted_count"])
        if a.get("host") and (alive or (now is not None and isinstance(a.get("ended"), (int, float))
                                        and now - a["ended"] <= HOST_GAP)):
            hosts.add(a["host"])
        if not alive:
            continue
        if st is None:
            if not whole(a):
                continue  # a run that has written no status yet: nothing is known of it
            # A whole part at work: the share of it its time so far is.
            owed = max(0, int(a.get("granted_count") or 0) - sent.get(me, 0))
            each = _part_seconds(attempts, sent, a.get("host"), me) or (1.0 / rates[a["host"]] if (rates or {}).get(a.get("host")) else None)
            if now is not None and owed and each and isinstance(a.get("started"), (int, float)):
                credit += owed * min(PART_CREDIT, max(0.0, now - a["started"]) / (owed * each))
            continue
        counts = {k: int(st[k]) if isinstance(st.get(k), (int, float)) else 0
                  for k in ("Taken", "GamesThisRun", "Dropped", "Runaways")}
        over = counts["GamesThisRun"] + counts["Dropped"] + counts["Runaways"]
        # What it finished and its coordinator does not hold yet: its outputs accepted, and its runaways and dropped
        # games once the ledger has them, are taken off what it counts.
        mine = sent.get(me, 0) + sum(1 for k in ("runaways", "dropped") for v in (d.get(k) or {}).values() if v == me)
        waiting += max(0, over - mine)
        flight += max(0, counts["Taken"] - over)
    left = max(0, total - skipped - played)
    waiting = min(waiting, left)
    return dict(total=total, played=played, skipped=skipped, finished=played + waiting, flight=min(flight, left - waiting),
                credit=min(credit, PART_CREDIT * max(0, left - waiting)), grain=sum(parts) / len(parts) if parts else 1.0,
                hosts=sorted(hosts))


def _part_seconds(attempts: dict, sent: dict, host, but: str) -> float | None:
    """What a game took in the parts `host` has delivered whole in this ledger (attempts that wrote no status, over,
    with outputs accepted): seconds a game, or None with none."""
    games = seconds = 0.0
    for me, a in attempts.items():
        if (me != but and a.get("host") == host and not isinstance(a.get("status"), dict) and sent.get(me)
                and isinstance(a.get("started"), (int, float)) and isinstance(a.get("ended"), (int, float))
                and a["ended"] > a["started"]):
            games += sent[me]
            seconds += a["ended"] - a["started"]
    return seconds / games if games else None


def ledger_rates(d: dict) -> dict:
    """What each host made of a ledger: the games in its attempts' accepted outputs over the time its attempts ran
    (`ledger_seconds` of them), in games a second. A host that delivered nothing has no rate."""
    sent: dict = {}
    for s in (d.get("shards") or {}).values():
        sent[s.get("attempt")] = sent.get(s.get("attempt"), 0) + _span(s.get("indices"))
    by: dict = {}
    for name, a in (d.get("attempts") or {}).items():
        if isinstance(a, dict) and a.get("host"):
            by.setdefault(a["host"], {})[a.get("id") or name] = a
    rates = {}
    for host, attempts in by.items():
        games, seconds = sum(sent.get(me, 0) for me in attempts), ledger_seconds(dict(attempts=attempts))
        if games > 0 and seconds > 0:
            rates[host] = games / seconds
    return rates


def ledger_seconds(d: dict) -> float:
    """How long a ledger's attempts ran: the time covered by any of them (two hosts at once count once, the wait
    between a stop and its resume not at all)."""
    spans = sorted((a["started"], a["ended"]) for a in (d.get("attempts") or {}).values()
                   if isinstance(a, dict) and isinstance(a.get("started"), (int, float))
                   and isinstance(a.get("ended"), (int, float)) and a["ended"] > a["started"])
    seconds, until = 0.0, None
    for start, end in spans:
        if until is None or start > until:
            seconds, until = seconds + end - start, end
        elif end > until:
            seconds, until = seconds + end - until, end
    return seconds


def pace(root: Path, gen: int, rel: str, settings: dict | None, last: int = 5) -> dict | None:
    """What the ledger `rel` (its path under `genN`) cost in the generations before `gen` that played it to its end
    with these settings (`pace_settings`; its `settings`, whoever played): `seconds_per_game` (the whole ledger's, the
    median of the last `last`), the `generations` counted, and `rates`: each host's games a second there (the median
    of the generations it played in with the backend and precision it has now), so that what is expected of a run is
    what the hosts that play it have made before, not what another set of hosts made together. With no generation of
    these settings, the last that played the ledger at all, whatever its settings (`other`: true): no stage runs with
    nothing to be held against, and an owner who means it to cost more says so (`--expect`, `--allow-slow`). None
    when no generation before played it to its end."""
    mine = (settings or {}).get("settings") if settings is not None else None
    hosts = (settings or {}).get("hosts") or {}
    same, any_ = [], None
    for n in range(gen - 1, 0, -1):
        path = root / f"gen{n}" / rel
        d = read_json(path) if path.exists() else None
        if not isinstance(d, dict):
            continue
        theirs = pace_settings(d.get("identity")) or {}
        alike = mine is not None and theirs.get("settings") == mine
        if not alike and any_ is not None:
            continue
        w, seconds = ledger_work(d), ledger_seconds(d)
        # Only one played to its end (every game played or, a closed duel's, skipped), by attempts whose times are known.
        if w["played"] > 0 and w["played"] + w["skipped"] >= w["total"] and seconds > 0:
            found = (n, seconds / w["played"], ledger_rates(d), theirs.get("hosts") or {})
            if any_ is None:
                any_ = found
            if alike:
                same.append(found)
                if len(same) >= last:
                    break
    if same:
        rates: dict = {}
        for _, _, each, served in same:
            for host, rate in each.items():
                # A host served another way then (another backend, another precision) made another rate.
                if host not in hosts or served.get(host, {}) == hosts[host]:
                    rates.setdefault(host, []).append(rate)
        return dict(seconds_per_game=statistics.median(x[1] for x in same), generations=[x[0] for x in same], other=False,
                    rates={h: statistics.median(r) for h, r in rates.items()})
    if any_:
        return dict(seconds_per_game=any_[1], generations=[any_[0]], other=True, rates=any_[2])
    return None


def active_ledger(gen_dir: Path, since: float) -> tuple[str, dict] | None:
    """The ledger the generation is playing now: the one written last since `since` (its coordinator saves it at every
    poll). Its path under the generation's directory, and what it holds."""
    dist = gen_dir / "dist"
    if not dist.is_dir():
        return None
    newest = None
    for path in [dist / "ledger.json", *dist.glob("*/ledger.json")]:
        try:
            at = os.stat(path).st_mtime
        except OSError:
            continue
        if at >= since and (newest is None or at > newest[0]):
            newest = (at, path)
    if newest is None:
        return None
    d = read_json(newest[1])
    if not isinstance(d, dict):
        return None
    return newest[1].relative_to(gen_dir).as_posix(), d


def hours(seconds: float) -> str:
    if seconds < 5400:
        return f"{seconds / 60:.0f} min"
    return f"{seconds / 3600:.1f} h" if seconds < 360000 else f"{seconds / 3600:.0f} h"


def expectation(s: dict, expect: dict | None = None) -> dict:
    """What the running stage's work should take, nearest first: an owner's word (`--expect STAGE=HOURS`: the time
    of each ledger the stage plays, as it is given); the earlier generations that played this ledger with these
    settings; the last that played it at all; the stage's usual time (each ledger of a stage that plays several is
    held to the whole of it). The generations' pace is that of **the hosts playing now** (`work.hosts`), each at the
    rate it made then, over the games this run has to play - what one host of two made is not held against the two's
    time. A host with no record adds nothing to what is expected and makes it `wide` (so does a pace that is not
    these hosts' at all): the factor is then `WIDE` times itself, rather than a guess made of that host. `known` is
    whether an owner or same-settings generations said it; `seconds` None with nothing to go by."""
    w = s.get("work") or {}
    stage = s.get("stage")
    asked = (expect or {}).get(stage)
    before = w.get("pace") or {}
    usual = s.get("expected_seconds")
    work = max(0.0, w.get("total", 0) - w.get("start", 0))  # the games this run of the stage has to play
    if asked:
        return dict(seconds=asked * 3600.0, basis="as --expect says", known=True, wide=False)
    if before.get("seconds_per_game"):
        gens = ", gen".join(str(g) for g in before.get("generations") or [])
        rates, hosts = before.get("rates") or {}, list(w.get("hosts") or [])
        have, lack = [h for h in hosts if rates.get(h)], [h for h in hosts if not rates.get(h)]
        if have:
            each, wide = 1.0 / sum(rates[h] for h in have), bool(lack)
            basis = f"at gen{gens}'s pace" + ("" if set(have) == set(rates) and not lack else f" on {', '.join(have)}")
            if lack:
                basis += f"; {', '.join(lack)} played no part of it then, so nothing is expected of it and the limit is wider"
        else:
            # No host playing now has a record there (or none is known to play): the whole ledger's pace then.
            each, wide = before["seconds_per_game"], bool(hosts)
            basis = f"at gen{gens}'s pace" + (", made by other hosts: the limit is wider" if wide else "")
        if before.get("other"):
            basis += (f", the last to play {w.get('ledger')}: with other settings, and no generation played it with these")
        return dict(seconds=each * work, basis=basis, known=not before.get("other"), wide=wide)
    if isinstance(usual, (int, float)) and usual > 0:
        return dict(seconds=float(usual), known=False, wide=False,
                    basis=f"the stage's usual time: no earlier generation played {w.get('ledger')}")
    return dict(seconds=None, basis=None, known=False, wide=False)


def project(s: dict, now: float, window: float = OVERRUN_WINDOW, expect: dict | None = None) -> dict | None:
    """Where the running stage of a status file is going, once its work has been watched for `window` seconds:
    `remaining` (seconds at its rate), `total` (this run's time on the work, from first sight to its projected end),
    `expected` (`expectation`), `known` and `wide` (there), `ratio` and `text`.

    The rate is that of the latest stretch of at least `window` seconds over which the work moved by `OVERRUN_GAMES`
    grains (a game; a whole part where parts are granted whole: fewer than three deliveries' worth say nothing of a
    rate): the last window once there is one, so a stage that collapses is seen within about a window, and as far
    back as it takes to see three grains move otherwise. With no such stretch there is no rate, and nothing is
    projected - but that a stage which has already spent more than it should in all is projected at what it has
    spent, the least it can take. None while there is nothing to say."""
    w = s.get("work")
    if not isinstance(w, dict) or w.get("stage") != s.get("stage") or not isinstance(w.get("done"), (int, float)):
        return None
    since, start, done = w["since"], w["start"], w["done"]
    at = min(now, w.get("at") or now)  # the rate is taken up to the last look at the work, not to a later reading
    watched = at - since
    if now - since < window or watched <= 0:
        return None
    left = max(0.0, w["total"] - done)
    stage = s.get("stage")
    e = expectation(s, expect)
    expected, basis = e["seconds"], e["basis"]
    work = max(0.0, w["total"] - start)
    need = OVERRUN_GAMES * max(1.0, float(w.get("grain") or 1.0))
    points = [[since, start], *(x for x in w.get("samples") or [] if isinstance(x, list) and len(x) == 2 and x[0] > since)]
    rate = None
    for t, v in reversed([x for x in points if x[0] <= at - window]):
        if done - v >= need:
            rate = (done - v) / (at - t)
            break
    common = dict(stage=stage, expected=expected, left=left, known=e["known"], wide=e["wide"])
    if rate is None:
        # Nothing to take a rate from (a part granted whole returns nothing until it is over). What it has spent is
        # the least it will take: past what it should with (nearly) nothing to show, that is its projection.
        if expected and left > 0 and watched > expected:
            return dict(common, remaining=None, total=None, ratio=watched / expected, rate=0.0,
                        text=f"{stage} is about {max(0.0, done - start):.0f} games' worth of time into its "
                             f"{work:.0f} games after {hours(watched)}, expected {hours(expected)} for all of them "
                             f"({basis})")
        return None
    remaining = left / rate
    total = watched + remaining
    out = dict(common, remaining=remaining, total=total, rate=rate * 3600.0, ratio=total / expected if expected else None)
    if expected:
        out["text"] = (f"{stage} projected to finish in {hours(remaining)} ({hours(total)} in all: {left:.0f} of "
                       f"{w['total']} games left at {rate * 3600:.1f} an hour), expected {hours(expected)} ({basis})")
    else:
        out["text"] = (f"{stage} projected {hours(total)} at the current rate ({hours(remaining)} more: {left:.0f} of "
                       f"{w['total']} games left at {rate * 3600:.1f} an hour); no earlier generation played "
                       f"{w.get('ledger')} or ran this stage, so nothing says what it should take")
    return out


def held_to(p: dict | None, factor: float = OVERRUN_FACTOR) -> float:
    """The factor a projection is held to: `factor`, `WIDE` times it where what is expected is not these hosts'."""
    return factor * (WIDE if p and p.get("wide") else 1.0)


def overrun(p: dict | None, factor: float = OVERRUN_FACTOR) -> bool:
    """Whether a projection (`project`) is past `factor` times its expectation (`held_to`)."""
    return bool(p and p.get("expected") and p["ratio"] > held_to(p, factor))


def allowed_slow(root: Path | None, flagged=()) -> set[str]:
    """The stages an owner lets be slow: the `--allow-slow` flags and the words of `ROOT/LOOP.allow-slow` (`all`:
    every stage)."""
    names = {x.strip() for x in flagged or () if x and x.strip()}
    if root is not None:
        try:
            for line in (root / ALLOW_SLOW).read_text(encoding="utf-8", errors="replace").splitlines():
                names.update(line.split("#", 1)[0].split())
        except OSError:
            pass
    return names


def check_window(seconds) -> None:
    """`--overrun-window`: more than nothing, and no longer than the samples kept (`SAMPLES` of them, three hours) -
    past that there would never be a window to take a rate over."""
    if seconds is not None and not 0 < seconds <= SAMPLE_EVERY * SAMPLES:
        raise SystemExit(f"--overrun-window {seconds:g}: between 1 and {SAMPLE_EVERY * SAMPLES:.0f} seconds (the "
                         f"work's samples are kept that long)")


def parse_expect(items) -> dict:
    """`--expect STAGE=HOURS`, as often as given: the hours an owner says a stage should take."""
    out = {}
    for item in items or ():
        stage, _, text = str(item).partition("=")
        try:
            value = float(text)
        except ValueError:
            value = 0.0
        if not stage.strip() or not value > 0:
            raise SystemExit(f"--expect {item}: STAGE=HOURS, e.g. selfplay=8")
        out[stage.strip()] = value
    return out


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
                         progress_detail={}, work=None, updated=now, error=None, transient=None, warnings=[],
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
        try:
            work = self.work()
        except Exception:  # noqa: BLE001 (the status never fails a run; a projection less is no reason to)
            work = self.data.get("work")
        with self.lock:
            now = self.clock()
            if detail != self.data["progress_detail"]:
                self.data.update(progress=self.data["progress"] + 1, progress_at=now, progress_detail=detail)
            self.data.update(work=work, updated=now)
            write_atomic(self.path, self.data)

    def work(self) -> dict | None:
        """The running stage's work as numbers a projection is made from (`project`): the ledger it plays, its total,
        the games done (finished, half of each in flight, and the share of a whole part its time so far is), where
        that stood (`start`) and when (`since`) this run of the stage first saw it, a sample of it a minute, the
        hosts playing it, how coarsely it moves (`grain`) and what it cost before (`pace`). A driver run again
        begins a new watch: the window is counted from when it first sees the ledger; so does a change of the hosts
        playing."""
        with self.lock:
            stage, began, kept = self.data.get("stage"), self.data["stage_started"], self.data.get("work")
        now = self.clock()
        if not kept or kept.get("stage") != stage:
            kept = None
        found = active_ledger(self.gen_dir, began) if stage else None
        if found is None:
            # No ledger yet, or one not readable this instant: what was seen stays.
            return kept
        rel, d = found
        settings = pace_settings(d.get("identity"))
        if kept is not None and kept.get("ledger") == rel and settings == kept.get("settings"):
            before = kept.get("pace")
        else:
            # At first sight, and again when a host joins the ledger (its backend and precision are then known).
            before = pace(self.root, self.gen, rel, settings)
        w = ledger_work(d, now, (before or {}).get("rates"))
        if w["played"] + w["skipped"] >= w["total"]:
            # Played to its end: whatever the stage does next (its summary, its next ledger) is not this work.
            return None
        done = w["finished"] + 0.5 * w["flight"] + w["credit"]
        identified = sorted((d.get("identity") or {}).get("hosts") or {}) if isinstance(d.get("identity"), dict) else []
        hosts = w["hosts"] or identified
        if kept is None or kept.get("ledger") != rel or kept.get("hosts") != hosts:
            # What is in flight when a run first sees a ledger is counted from where it is: only what moves from
            # here on is this run's rate. And again when the hosts playing it change (one joins hours late, one is
            # lost): what is left is then held to what these hosts make of it, from this moment - the time spent
            # with other hosts is not theirs, and the rate they made is not this set's. The window begins again.
            kept = dict(stage=stage, ledger=rel, since=now, start=done, samples=[])
        samples = list(kept["samples"])
        if not samples or now - samples[-1][0] >= SAMPLE_EVERY:
            samples.append([now, done])
        return dict(kept, settings=settings, pace=before, total=w["total"] - w["skipped"], done=done,
                    finished=w["finished"], flight=w["flight"], grain=w["grain"], hosts=hosts,
                    at=now, samples=samples[-SAMPLES:])

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
          alive=alive, overrun_factor: float = OVERRUN_FACTOR, overrun_window: float = OVERRUN_WINDOW,
          expect: dict | None = None, allow_slow=()) -> dict:
    """What a status file says now: `state` (running, stalled, overdue-projected, dead, failed, stopped, finished,
    none) and `why`, with `overdue` where a running stage has taken far longer than it did before and `projection`
    (`project`) where its end can be projected. A stage in `allow_slow` (or `all` there) is never overdue by
    projection: its projection is still in the reading."""
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
    try:
        projection = project(s, now, overrun_window, expect)
    except (KeyError, TypeError, ValueError, ZeroDivisionError):  # a file from another version: no projection
        projection = None
    if projection:
        out["projection"] = projection
        slow = set(allow_slow or ())
        if overrun(projection, overrun_factor):
            if s.get("stage") in slow or "all" in slow:
                out["why"] += f"; {projection['text']} - allowed to be slow"
            else:
                out.update(state=PROJECTED, why=f"{projection['text']}: more than {held_to(projection, overrun_factor):g} times over")
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
    loop = read_json(root / LOOP)
    # What the supervisor watches with (`loop.py` writes it into its state), where this was not given its own.
    watch = (loop or {}).get("watch") if isinstance((loop or {}).get("watch"), dict) else {}
    for key in ("overrun_factor", "overrun_window", "expect"):
        if limits.get(key) is None:
            limits.pop(key, None)
            if watch.get(key) is not None:
                limits[key] = watch[key]
    limits["allow_slow"] = allowed_slow(root, [*(limits.get("allow_slow") or ()), *(watch.get("allow_slow") or ())])
    out = dict(root=str(root), status=s, **judge(s, **limits))
    if loop:
        out["loop"] = dict(loop, alive=alive(loop) if loop.get("host") == socket.gethostname() else None)
    if s:
        log = stage_log(root, s["generation"])
        out["log"] = dict(path=str(log), tail=tail(log, lines)) if log else None
        if s.get("driver_log"):
            out["driver_log"] = dict(path=s["driver_log"], tail=tail(Path(s["driver_log"]), lines))
    return out


def main(argv=None):
    a = argparse.ArgumentParser(description="Whether a generation is running, stalled, overdue by projection, dead, "
                                            "failed or finished.")
    a.add_argument("--root", required=True, help="the ai-data directory")
    a.add_argument("--generation", type=int, help="default: the newest with a status file")
    a.add_argument("--json", action="store_true")
    a.add_argument("--tail", type=int, default=15, help="lines of the stage's log shown")
    a.add_argument("--stall-factor", type=float, default=STALL_FACTOR,
                   help="no progress for this share of the stage's expected time is a stall")
    a.add_argument("--stall-floor", type=float, default=STALL_FLOOR, metavar="S", help="and for at least this long")
    a.add_argument("--overrun-factor", type=float, metavar="X",
                   help=f"a stage projected to take this many times what it should is overdue by projection (the "
                        f"supervisor's, or {OVERRUN_FACTOR:g})")
    a.add_argument("--overrun-window", type=float, metavar="S",
                   help=f"once its work has been watched this long (the supervisor's, or {OVERRUN_WINDOW:.0f})")
    a.add_argument("--expect", action="append", metavar="STAGE=HOURS",
                   help="what each ledger of a stage should take, in place of the earlier generations' pace (repeatable)")
    a.add_argument("--allow-slow", action="append", metavar="STAGE",
                   help=f"a stage that may be slow (repeatable; 'all'; also the words of ROOT/{ALLOW_SLOW})")
    args = a.parse_args(argv)
    check_window(args.overrun_window)
    r = report(Path(args.root).resolve(), args.generation, args.tail, stall_factor=args.stall_factor,
               stall_floor=args.stall_floor, overrun_factor=args.overrun_factor, overrun_window=args.overrun_window,
               expect=parse_expect(args.expect) or None, allow_slow=args.allow_slow)
    if args.json:
        print(json.dumps(r, indent=1))
        return EXIT[r["state"]]
    s = r["status"] or {}
    print(f"gen{s.get('generation', '?')}: {r['state'].upper()} - {r['why']}")
    if r.get("overdue"):
        print(f"  overdue: {r['overdue']}")
    if r.get("projection") and r["state"] != PROJECTED:
        print(f"  projection: {r['projection']['text']}")
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
