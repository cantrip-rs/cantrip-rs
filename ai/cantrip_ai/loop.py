"""Generations one after another, watched (docs/ai-training.md, "The loop"): the supervisor of a training line.

    python -m cantrip_ai.loop --root ROOT [--args-file FILE] [--update origin/main] [--retries 2] [-- DRIVER ARGS...]

It runs `cantrip_ai.generation` with one set of arguments (`--args-file`, default `ROOT/generation.args`: the
driver's arguments, on as many lines as wanted, `#` comments; and whatever follows `--`), generation after generation,
and at each turn

1. ends if `ROOT/LOOP.stop` exists, and waits while `ROOT/LOOP.pause` exists (a person's pause, or its own hold);
2. attaches to a driver already running the generation (`Loop.pending`) and does nothing else beside it;
3. with `--update REMOTE/BRANCH`, and only between generations - never while the next is begun and not finished -
   fetches and moves the working tree to it, by fast-forward only and only when the tree is clean: code never
   changes under a generation, and each generation pins the engine it started with (`engine.py`);
4. runs the driver's `--preflight` (it builds, checks the arguments, the disk, the memory, the workers, the mined
   targets and the champion's parity, and prints the plan), and holds if it fails: a flag the driver no longer
   knows, a build that broke, stops the loop here and not three hours into a generation;
5. runs the generation, its output in `ROOT/genN-driver.log`, and watches it (`status.py`): a stage that stalls and a
   stage far past its usual time are told (`notify.py`) while it runs; a driver that dies, fails or is stopped is told
   when it does. A stage that lives and moves, and at the rate it moves is projected to take `--overrun-factor` (2)
   times what it should (`status.project`: watched for `--overrun-window` seconds, against the earlier generations'
   pace with the same settings, or `--expect STAGE=HOURS`) at every look for `--overrun-persist` seconds (600), is
   not let run on: it is told (`overrun`), its work stopped as `generation.py --stop` stops it (resumable; as
   `--stop-now` if the driver has not ended `--overrun-stop-timeout` seconds later), and the loop held with the
   projection. `--allow-slow STAGE`
   - or the stage's name in `ROOT/LOOP.allow-slow`, read at every look, so no restart - lets a stage be slow on
   purpose. With no generation of the same settings to go by, the stage is held against the last that played its
   ledger at all (over this run's games), and its projection is told once besides (`projected`).

One supervisor a root (`ROOT/loop.lock`). Nothing it runs hangs it unnoticed: a git command and the preflight have
time limits, past which they are killed with what they started, and told.

A failure the driver names as transient (`status.TRANSIENT`: self-play stopped by its memory governor, an inference
server that did not start) is run again, `--retries` times a generation, after `--retry-wait` seconds. Anything else
**holds** the loop: it writes the reason into `ROOT/LOOP.pause`, tells it, and waits there until the file is removed
(the generation is then resumed where it stopped) or `LOOP.stop` ends it. Its own state is `ROOT/loop.json`
(`status` shows it), its log `ROOT/loop.log`.

The process that loops is small and never changes while it runs; each step is a fresh process of this module
(`--phase update`, `--phase run`), so a code update is in force from the step after it. A supervisor started while
a generation's driver is already running attaches to it instead of starting another.

No torch and no numpy.
"""

from __future__ import annotations

import argparse
import json
import os
import shlex
import socket
import subprocess
import sys
import time
from pathlib import Path

from . import distributed, notify, status

REPO = Path(__file__).resolve().parents[2]
PAUSE, STOP, LOG, ARGS = "LOOP.pause", "LOOP.stop", "loop.log", "generation.args"
# A step's exit codes: done (go on), the loop is to end, the loop is held (LOOP.pause is written).
OK, END, HELD = 0, 3, 4
LOCK = "loop.lock"
# How long the supervisor waits on a git command (a fetch from a remote that does not answer) and on the driver's
# preflight (it builds) before it gives it up: neither may hang the loop unnoticed.
GIT_TIMEOUT, FETCH_TIMEOUT = 120.0, 600.0
# A stage is stopped for its projection once the reading has held this long, and its games in flight are stopped too
# once its driver has not ended this long after.
PERSIST, STOP_TIMEOUT = 600.0, 1200.0


def unquoted_backslashes(text: str) -> list[str]:
    """The lines of an arguments file with a backslash outside quotes that is not a line's last character: the shell
    words' rules drop it (`D:\\data\\decks` reads as `D:datadecks`), which is never what a Windows path meant."""
    found = []
    for line in text.splitlines():
        quote, k = None, 0
        while k < len(line):
            c = line[k]
            if quote:
                if c == "\\" and quote == '"':
                    k += 1
                elif c == quote:
                    quote = None
            elif c in "'\"":
                quote = c
            elif c == "#" and (k == 0 or line[k - 1].isspace()):
                break
            elif c == "\\":
                if k + 1 < len(line):
                    found.append(line.strip())
                    break
            k += 1
    return found


def read_args(path: Path, warn=None) -> list[str]:
    """The driver's arguments from a file: shell words on any number of lines, `#` starting a comment. Windows paths
    are written with forward slashes, or quoted; a line with an unquoted backslash is told to `warn`."""
    text = path.read_text(encoding="utf-8")
    if warn is not None:
        for line in unquoted_backslashes(text):
            warn(f"{path}: an unquoted backslash is dropped, as a shell drops it - write the path with forward "
                 f"slashes or in quotes: {line[:200]}")
    return shlex.split(text, comments=True)


class Loop:
    def __init__(self, root: Path, args, driver_args: list[str], supervisor: int | None = None, sleep=time.sleep,
                 clock=time.time):
        self.root, self.args, self.driver_args = root, args, driver_args
        self.supervisor = supervisor or os.getpid()
        self.sleep, self.clock = sleep, clock

    # What it says, and where.

    def log(self, text: str):
        line = f"[{time.strftime('%Y-%m-%d %H:%M:%S')}] {text}"
        print(line, flush=True)
        try:
            with (self.root / LOG).open("a", encoding="utf-8") as f:
                f.write(line + "\n")
        except OSError:
            pass

    def state(self, state: str, why: str = "", **more):
        """The loop's own state, for `status`: never a reason for a step to fail (`status.write_atomic` does not raise)."""
        try:
            status.write_atomic(self.root / status.LOOP, dict(
                state=state, why=why, pid=self.supervisor, pid_start=distributed.process_start(self.supervisor),
                host=socket.gethostname(), since=time.strftime("%Y-%m-%d %H:%M:%S"), watch=self.limits(), **more))
        except Exception as e:  # noqa: BLE001
            self.log(f"the loop's state was not written ({type(e).__name__}: {e})")

    def limits(self) -> dict:
        """What a stage's projection is judged with (`status.judge`): written into the loop's state, so that
        `status` says what the supervisor would."""
        a = self.args
        # (How long a reading must last before the stage is stopped, and how long its stop is waited for, are the
        # supervisor's own: `--overrun-persist`, `--overrun-stop-timeout`.)
        return dict(overrun_factor=getattr(a, "overrun_factor", status.OVERRUN_FACTOR),
                    overrun_window=getattr(a, "overrun_window", status.OVERRUN_WINDOW),
                    expect=status.parse_expect(getattr(a, "expect", None)),
                    allow_slow=sorted(set(getattr(a, "allow_slow", None) or ())))

    def tell(self, event: str, message: str, generation=None, stage=None, **details):
        notify.send(event, message, root=self.root, generation=generation, stage=stage, log=self.log, **details)

    def hold(self, why: str, generation=None, event: str = "held", **details) -> int:
        """The loop held for a person: the reason in LOOP.pause, told; it goes on when the file is removed."""
        for _ in range(20):
            try:
                (self.root / PAUSE).write_text(f"held {time.strftime('%Y-%m-%d %H:%M:%S')}: {why}\n\nRemove this file to "
                                               f"go on (the generation resumes where it stopped); create {STOP} to end "
                                               "the loop.\n", encoding="utf-8")
                break
            except OSError as e:  # the hold must be written: without it the loop would run on
                self.log(f"{PAUSE} was not written ({e}); trying again")
                self.sleep(1.0)
        self.state("held", why, generation=generation)
        self.tell(event, f"the loop is HELD: {why} (remove {self.root / PAUSE} to go on)", generation, **details)
        return HELD

    # Between generations.

    def boundary(self) -> int:
        """Ends on the stop file; waits out the pause file."""
        told = False
        while True:
            if (self.root / STOP).exists():
                self.log(f"{STOP}: the loop ends")
                self.state("stopped", f"{STOP} was found between generations")
                return END
            try:
                # Read, not asked for and then read: the file may go between the two.
                text = (self.root / PAUSE).read_text(encoding="utf-8", errors="replace")
            except FileNotFoundError:
                if told:
                    self.log(f"{PAUSE} is gone: going on")
                return OK
            except OSError:
                text = ""  # there, and not readable this instant: still a pause
            if not told:
                first = (text.strip().splitlines() or [""])[0]
                self.log(f"waiting while {PAUSE} exists" + (f" ({first})" if first else ""))
                if not first.startswith("held "):
                    self.state("paused", first or "LOOP.pause exists")
                told = True
            self.sleep(self.args.poll)

    def git(self, *words, timeout: float = GIT_TIMEOUT) -> subprocess.CompletedProcess:
        """A git command in the working tree, given up (and told) after `timeout`: it never prompts, and takes no
        lock it does not need."""
        env = {**os.environ, "GIT_TERMINAL_PROMPT": "0", "GIT_OPTIONAL_LOCKS": "0"}
        p = subprocess.Popen(["git", *words], cwd=REPO, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
                             encoding="utf-8", errors="replace", stdin=subprocess.DEVNULL, env=env,
                             **(dict(creationflags=subprocess.CREATE_NEW_PROCESS_GROUP) if os.name == "nt"
                                else dict(start_new_session=True)))
        try:
            out, err = p.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            distributed.kill_tree(p)
            try:
                p.communicate(timeout=10)
            except (subprocess.TimeoutExpired, OSError, ValueError):
                pass
            self.tell("warning", f"the supervisor's `git {' '.join(words)}` did not end in {timeout:.0f} s and was killed")
            return subprocess.CompletedProcess(p.args, 124, "", f"timed out after {timeout:.0f} s")
        return subprocess.CompletedProcess(p.args, p.returncode, out, err)

    def pending(self) -> tuple[int, bool, dict | None]:
        """The generation the driver would run now, whether it is unfinished (begun, and not recorded to its end),
        and its status when a driver is running it on this machine right now."""
        from . import generation  # noqa: PLC0415 (the driver's own reading of a root; it imports no torch)

        gen = generation.next_generation(self.root)
        if "--generation" in self.driver_args[:-1]:
            gen = int(self.driver_args[self.driver_args.index("--generation") + 1])
        s = status.read(self.root, gen)
        unfinished = (((self.root / f"gen{gen}" / "generation.json").exists() or s is not None)
                      and not generation.completed(self.root, gen))
        live = bool(s and s.get("state") == "running" and s.get("host") == socket.gethostname() and status.alive(s))
        return gen, unfinished, s if live else None

    def update(self) -> int:
        """The working tree moved to `--update`'s ref: fetched, by fast-forward only, a clean tree only - and only
        between generations: never under a generation that is begun and not finished (a driver may be running it, and
        it resumes with the code it began with)."""
        ref = self.args.update
        gen, unfinished, _ = self.pending()
        if unfinished:
            self.log(f"update: gen{gen} is begun and not finished; the code is updated only between generations")
            return OK
        remote = ref.split("/", 1)[0] if "/" in ref else None
        if remote:
            p = self.git("fetch", remote, timeout=FETCH_TIMEOUT)
            if p.returncode != 0:
                # No network is no reason to stop training: the code stays as it is.
                self.log(f"update: git fetch {remote} failed ({(p.stderr or p.stdout).strip()[-200:]}); the code stays as it is")
                return OK
        head, target = self.git("rev-parse", "HEAD").stdout.strip(), self.git("rev-parse", ref).stdout.strip()
        if not target:
            return self.hold(f"code update: no such ref {ref}")
        if head == target:
            self.log(f"update: already at {ref} ({head[:10]})")
            return OK
        if self.git("merge-base", "--is-ancestor", target, "HEAD").returncode == 0:
            self.log(f"update: the tree ({head[:10]}) is ahead of {ref} ({target[:10]}); nothing to do")
            return OK
        dirty = self.git("status", "--porcelain", "--untracked-files=no").stdout.strip()
        if dirty:
            return self.hold(f"code update to {ref}: the working tree {REPO} has local changes ({len(dirty.splitlines())} "
                             "file(s)); commit or remove them")
        p = self.git("merge", "--ff-only", ref)
        if p.returncode != 0:
            return self.hold(f"code update to {ref}: not a fast-forward from {head[:10]} "
                             f"({(p.stderr or p.stdout).strip()[-300:]})")
        self.log(f"update: {head[:10]} -> {target[:10]} ({ref}), fast-forward")
        return OK

    # A generation.

    def driver(self, *extra: str) -> list[str]:
        return [sys.executable, "-m", "cantrip_ai.generation", "--root", str(self.root), *self.driver_args, *extra]

    def environment(self, log_path: Path | None = None) -> dict:
        env = {**os.environ, "PYTHONUNBUFFERED": "1",
               "PYTHONPATH": os.pathsep.join(x for x in (str(REPO / "ai"), os.environ.get("PYTHONPATH")) if x)}
        if log_path is not None:
            env["CANTRIP_AI_DRIVER_LOG"] = str(log_path)
        return env

    def preflight_once(self, out: Path) -> tuple[int | None, list[str]]:
        """The driver's `--preflight`, its output in `out`: its exit code (None: it did not end in time, and was
        killed) and the tail of what it said. It runs in a process group of its own (so that its time limit can end
        everything it started), which a Ctrl-C at the console therefore does not reach: whatever ends this step -
        an interrupt, an error - its tree is killed on the way out."""
        with out.open("w", encoding="utf-8") as f:
            p = subprocess.Popen(self.driver("--preflight"), cwd=REPO / "ai", stdout=f, stderr=subprocess.STDOUT,
                                 env=self.environment(), stdin=subprocess.DEVNULL,
                                 **(dict(creationflags=subprocess.CREATE_NEW_PROCESS_GROUP) if os.name == "nt"
                                    else dict(start_new_session=True)))
            try:
                code = p.wait(timeout=self.args.preflight_timeout)
            except subprocess.TimeoutExpired:
                code = None
            finally:
                if p.poll() is None:
                    distributed.kill_tree(p)
                    try:
                        p.wait(timeout=30)
                    except (subprocess.TimeoutExpired, OSError):
                        pass
        return code, status.tail(out, 40)

    def preflight(self) -> int:
        out = self.root / "preflight" / "preflight.log"
        out.parent.mkdir(parents=True, exist_ok=True)
        self.state("preflight", "the driver's --preflight before the next generation")
        retries = 0
        while True:
            self.log(f"preflight: {' '.join(self.driver('--preflight'))} > {out}")
            code, tail = self.preflight_once(out)
            if code == 0:
                self.log("preflight: passed")
                return OK
            if code is None:
                return self.hold(f"the preflight of the next generation did not end in "
                                 f"{self.args.preflight_timeout:.0f} s and was killed (its output: {out})",
                                 log_tail=tail[-12:])
            # The same failures a generation is run again after (`status.TRANSIENT`: a server that did not start, and
            # not for parity) are tried again here, as often and after as long.
            kind = status.transient("\n".join(tail))
            if kind and retries < self.args.retries:
                retries += 1
                self.tell("retry", f"the preflight of the next generation failed with a known transient failure ({kind}); "
                                   f"running it again in {self.args.retry_wait:.0f} s (retry {retries} of {self.args.retries})")
                self.sleep(self.args.retry_wait)
                continue
            return self.hold(f"the preflight of the next generation failed (exit {code}"
                             + (f", after {retries} retries" if retries else "") + "): "
                             + " / ".join(line.strip() for line in tail[-4:] if line.strip())[:600]
                             + f" (its output: {out})", log_tail=tail[-12:])

    def strangers(self) -> list[int]:
        """Drivers of this root that are running and that no status file accounts for (`status.driver_processes`: a
        driver from before the status file, or one started by hand a moment ago): never run beside."""
        found = status.driver_processes(self.root, cwd=REPO / "ai")
        if found is None:
            self.log("the process table could not be read: going by the status files alone")
            return []
        return found

    def watch(self, gen: int, alive, told: dict):
        """One look at a running generation: a stall, a stage far past its time and their end are told, each once
        (a stall again every `--renotify` seconds while it lasts); a stage projected to overrun is told and its work
        stopped (`told["overrun"]`: why, for the hold that follows its stop); a projection no generation of the same
        settings stands behind is told once, held or not."""
        s = status.read(self.root, gen)
        if not s or s.get("state") != "running":
            return
        asked = told.get("overrun")
        wait = getattr(self.args, "overrun_stop_timeout", STOP_TIMEOUT)
        if asked and not asked.get("now") and self.clock() - asked["at"] >= wait:
            # Asked to stop, and still running (its games in play are long, or a host does not answer): its games
            # in flight are stopped too, as `--stop-now` stops them. What was finished is kept either way.
            # (A stop once asked is not taken back: a reading that recovers while the stage is stopping changes nothing.)
            asked["now"] = True
            self.tell("overrun", f"gen{gen}: stage {asked['stage']} was asked to stop {wait / 60:.0f} min ago and its "
                                 f"driver has not ended: its games in flight are stopped too (as --stop-now does; what "
                                 f"was finished is kept)", gen, asked["stage"])
            self.stop_stage(gen, now=True)
        limits = self.limits()
        limits["allow_slow"] = status.allowed_slow(self.root, limits["allow_slow"])
        j = status.judge(s, self.clock(), self.args.stall_factor, self.args.stall_floor, alive=lambda pid: alive(), **limits)
        stage = s.get("stage")
        projection = j.get("projection")
        if projection and not projection.get("known"):
            # Nothing of these settings to go by: where it is going is told once, held or not.
            key = (stage, (s.get("work") or {}).get("ledger"))
            if key not in told.setdefault("projected", set()):
                told["projected"].add(key)
                self.tell("projected", f"gen{gen}: {projection['text']}", gen, stage, projection=projection)
        if j["state"] == status.PROJECTED:
            # One look is no reason to stop a stage: a part delivered late, a sample at a bad moment. The reading
            # has to hold at every look for `--overrun-persist` seconds.
            persist = getattr(self.args, "overrun_persist", PERSIST)
            # The same watch throughout: a ledger seen anew (other hosts playing it, the next ledger of the stage)
            # begins the count again, as a look that is not over does.
            work = s.get("work") or {}
            key = (stage, work.get("ledger"), work.get("since"))
            if told.get("over_key") != key and "overrun" not in told:
                told["over_key"] = key
                told.pop("over_since", None)
            since = told.setdefault("over_since", self.clock())
            if "overrun" not in told and self.clock() - since >= persist:
                told["overrun"] = dict(stage=stage, why=projection["text"], at=self.clock())
                self.tell("overrun", f"gen{gen}: {j['why']}"
                                     + (f", at every look for {(self.clock() - since) / 60:.0f} min" if persist else "")
                                     + f". Its work is being stopped (as --stop does: what is played is kept; if its "
                                       f"driver has not ended in {wait / 60:.0f} min its games in flight are stopped "
                                       f"too) and the loop will be held", gen, stage, projection=projection)
                self.stop_stage(gen)
            return
        if "overrun" not in told:
            told.pop("over_since", None)
        if j["state"] == "stalled":
            last = told.get("stalled")
            if last is None or self.clock() - last >= self.args.renotify:
                told["stalled"] = self.clock()
                log = status.stage_log(self.root, gen)
                self.tell("stalled", f"gen{gen} is STALLED: {j['why']}", gen, stage,
                          log_file=str(log) if log else None, log_tail=status.tail(log, 8) if log else [])
        elif told.pop("stalled", None) is not None:
            self.tell("resumed", f"gen{gen} is making progress again: {j['why']}", gen, stage)
        if j.get("overdue") and told.get("overdue") != stage:
            told["overdue"] = stage
            self.tell("warning", f"gen{gen}: {j['overdue']}", gen, stage)

    def stop_stage(self, gen: int, now: bool = False):
        """The running stage's work stopped as `generation.py --stop` stops it: its games in play end and are kept,
        and the driver ends `stopped`, to be resumed. With `now`, as `--stop-now`: its games in flight stop too."""
        from . import generation  # noqa: PLC0415 (the driver's own reading of a root; it imports no torch)

        generation.request_stop(self.root, gen, now=now, log=self.log)

    def run_once(self, gen: int) -> tuple[str, dict]:
        """The generation's driver run (or, one already running, attached to) and watched to its end: how it ended
        (`finished`, `failed`, `stopped`, `dead`, `unfinished`) and its last status."""
        from . import generation  # noqa: PLC0415 (the driver's own reading of a root; it imports no torch)

        log_path = self.root / f"gen{gen}-driver.log"
        began = self.clock()
        before = status.read(self.root, gen)
        attached = bool(before and before.get("state") == "running" and before.get("host") == socket.gethostname()
                        and status.alive(before))
        if attached:
            pid = before["pid"]
            self.log(f"gen{gen}: its driver is running already (pid {pid}): attached to it")
            alive, code = (lambda: status.alive(before)), (lambda: None)
        else:
            self.log(f"gen{gen}: {' '.join(self.driver())} >> {log_path}")
            f = log_path.open("a", encoding="utf-8")
            f.write(f"\n===== {time.strftime('%Y-%m-%d %H:%M:%S')} supervisor: gen{gen} =====\n")
            f.flush()
            p = subprocess.Popen(self.driver(), cwd=REPO / "ai", stdout=f, stderr=subprocess.STDOUT,
                                 env=self.environment(log_path), stdin=subprocess.DEVNULL)
            alive, code = (lambda: p.poll() is None), (lambda: p.returncode)
        self.state("running", f"gen{gen}", generation=gen, driver_log=str(log_path))
        told: dict = {}
        self.told = told
        while alive():
            self.sleep(self.args.poll)
            try:
                self.watch(gen, alive, told)
            except Exception as e:  # noqa: BLE001 (watching never ends a run)
                self.log(f"watch: {type(e).__name__}: {e}")
        s = status.read(self.root, gen) or {}
        if not attached:
            f.close()
        if not attached and (s.get("updated") or 0) < began:
            # It ended before it wrote a status of its own (its arguments, its setup): what the file says is an
            # earlier run's.
            return "failed", dict(stage="its start", exit=code(), transient=None,
                                  error=f"the driver exited ({code()}) before it ran a stage: "
                                        + " / ".join(x.strip() for x in status.tail(log_path, 6) if x.strip()))
        said = s.get("state")
        if said == "finished" and generation.completed(self.root, gen):
            return "finished", s
        if said in ("failed", "stopped"):
            return said, s
        if said == "running" or not s:
            return "dead", dict(s, exit=code())
        return "unfinished", dict(s, exit=code())

    def run(self) -> int:
        # A driver already running the generation is attached to before anything else is done: no preflight is run
        # beside it (it builds, and starts a server), and nothing is started twice.
        gen, _, live = self.pending()
        if live is None:
            # No driver with a status file runs it. One without (an older driver, which writes none) may: by the
            # process table, and then nothing is started - no preflight, no second driver.
            others = self.strangers()
            if others:
                return self.hold(f"a driver of this root is running already (pid {', '.join(map(str, others))}) and "
                                 f"writes no status file - one from before this supervisor? Nothing is started beside "
                                 "it: wait for it to end (or stop it)", gen)
            if not self.args.no_preflight:
                code = self.preflight()
                if code != OK:
                    return code
        retries = 0
        while True:
            ended, s = self.run_once(gen)
            log_path = self.root / f"gen{gen}-driver.log"
            if ended == "finished":
                self.log(f"gen{gen} finished")
                self.state("between", f"gen{gen} finished", generation=gen)
                return OK
            error = (s.get("error") or "").strip()
            first = (error.splitlines() or [""])[0][:400]
            if ended == "failed" and s.get("transient") and retries < self.args.retries:
                retries += 1
                self.tell("retry", f"gen{gen} failed at {s.get('stage')} with a known transient failure "
                                   f"({s['transient']}: {first}); running it again in {self.args.retry_wait:.0f} s "
                                   f"(retry {retries} of {self.args.retries})", gen, s.get("stage"))
                self.state("retrying", f"gen{gen}: {s['transient']}", generation=gen)
                self.sleep(self.args.retry_wait)
                continue
            tail = status.tail(log_path, 10)
            if ended == "failed":
                return self.hold(f"gen{gen} failed at stage {s.get('stage')}: {first}"
                                 + (f" (after {retries} retries)" if retries else "") + f" (log: {log_path})", gen,
                                 stage=s.get("stage"), log_tail=tail)
            overrun = getattr(self, "told", {}).get("overrun")
            if ended == "stopped" and overrun:
                return self.hold(f"gen{gen}: {overrun['why']}. Its work was stopped (what was played is kept) and "
                                 f"resumes when this file is removed - and is stopped again if it is as slow: fix "
                                 f"what slows it, or let it be slow on purpose with the line `{overrun['stage']}` in "
                                 f"{self.root / status.ALLOW_SLOW} (or the supervisor's --allow-slow "
                                 f"{overrun['stage']}, or --expect {overrun['stage']}=HOURS)", gen,
                                 stage=overrun["stage"], log_tail=tail)
            if ended == "stopped":
                return self.hold(f"gen{gen} was stopped at stage {s.get('stage')}, as asked: {first}", gen, event="stopped")
            if ended == "dead":
                return self.hold(f"gen{gen}'s driver DIED at stage {s.get('stage')} without a last word (exit "
                                 f"{s.get('exit')}; log: {log_path})", gen, event="dead", stage=s.get("stage"), log_tail=tail)
            return self.hold(f"gen{gen}'s driver ended (exit {s.get('exit')}) and the generation is not finished: "
                             f"its arguments stop it early? (log: {log_path})", gen, log_tail=tail)


def parser() -> argparse.ArgumentParser:
    a = argparse.ArgumentParser(description="Generations one after another, watched: the training loop's supervisor.")
    a.add_argument("--root", required=True, help="the ai-data directory")
    a.add_argument("--args-file", help=f"the driver's arguments (default ROOT/{ARGS}, when it exists)")
    a.add_argument("--update", metavar="REMOTE/BRANCH",
                   help="between generations, fetch and fast-forward the working tree to this ref (never otherwise)")
    a.add_argument("--max-generations", type=int, default=0, help="end after this many (0: until LOOP.stop)")
    a.add_argument("--retries", type=int, default=2, help="reruns of a generation after a known transient failure")
    a.add_argument("--retry-wait", type=float, default=300.0, metavar="S")
    a.add_argument("--poll", type=float, default=30.0, metavar="S", help="seconds between looks at the driver and the files")
    a.add_argument("--stall-factor", type=float, default=status.STALL_FACTOR,
                   help="a stage with no progress for this share of its usual time is stalled")
    a.add_argument("--stall-floor", type=float, default=status.STALL_FLOOR, metavar="S", help="and for at least this long")
    a.add_argument("--renotify", type=float, default=3600.0, metavar="S", help="a stall that lasts is told again this often")
    a.add_argument("--overrun-factor", type=float, default=status.OVERRUN_FACTOR, metavar="X",
                   help="a stage projected, at the rate it goes, to take this many times what it should is stopped "
                        "(resumable) and the loop held")
    a.add_argument("--overrun-window", type=float, default=status.OVERRUN_WINDOW, metavar="S",
                   help="a stage's rate is watched this long before it is projected")
    a.add_argument("--overrun-persist", type=float, default=PERSIST, metavar="S",
                   help="and stopped only once that reading has held at every look for this long")
    a.add_argument("--overrun-stop-timeout", type=float, default=STOP_TIMEOUT, metavar="S",
                   help="a stage asked to stop whose driver has not ended after this long has its games in flight "
                        "stopped too (as --stop-now)")
    a.add_argument("--expect", action="append", default=[], metavar="STAGE=HOURS",
                   help="what a stage should take - each ledger it plays - in place of the earlier generations' pace "
                        "(repeatable): for a stage meant to cost more than the last generation's did")
    a.add_argument("--allow-slow", action="append", default=[], metavar="STAGE",
                   help=f"a stage that may be slow on purpose: never stopped for its projection (repeatable; 'all'; "
                        f"also the words of ROOT/{status.ALLOW_SLOW}, read at every look)")
    a.add_argument("--no-preflight", action="store_true", help="no --preflight before a generation (not advised)")
    a.add_argument("--preflight-timeout", type=float, default=7200.0, metavar="S",
                   help="a preflight (it builds) that has not ended after this long is killed, and the loop held")
    a.add_argument("--phase", choices=["update", "run"], help=argparse.SUPPRESS)
    a.add_argument("--supervisor", type=int, help=argparse.SUPPRESS)
    return a


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    extra = []
    if "--" in argv:
        at = argv.index("--")
        argv, extra = argv[:at], argv[at + 1:]
    args = parser().parse_args(argv)
    status.parse_expect(args.expect)  # refused here, not at the first look at a generation
    status.check_window(args.overrun_window)
    root = Path(args.root).resolve()
    root.mkdir(parents=True, exist_ok=True)
    file = Path(args.args_file) if args.args_file else root / ARGS
    if args.args_file and not file.exists():
        sys.exit(f"--args-file {file}: no such file")
    doubts: list[str] = []
    driver_args = (read_args(file, doubts.append) if file.exists() else []) + extra
    if "--root" in driver_args:
        sys.exit("the driver's arguments name --root: the supervisor passes its own")
    loop = Loop(root, args, driver_args, args.supervisor)
    if args.phase == "update":
        return loop.update()
    if args.phase == "run":
        # Ended by a signal as by Ctrl-C: the step's way out is the same, and ends what it started (its preflight).
        import signal  # noqa: PLC0415

        def interrupted(*_):
            raise KeyboardInterrupt

        for name in ("SIGTERM", "SIGBREAK"):
            if hasattr(signal, name):
                try:
                    signal.signal(getattr(signal, name), interrupted)
                except (OSError, ValueError):
                    pass
        try:
            return loop.run()
        except KeyboardInterrupt:
            loop.log("interrupted: this step ends (its preflight, if one was running, was killed with what it started)")
            return 130
    # One supervisor a root: a second would run a second driver's preflight and update under the first's generation.
    with distributed.LedgerLock(root / LOCK, "another supervisor (pid {pid}) runs this root's loop"):
        for doubt in doubts:
            loop.tell("warning", doubt)
        return supervise(loop, root, args, argv, extra, driver_args)


def supervise(loop: Loop, root: Path, args, argv: list[str], extra: list[str], driver_args: list[str]) -> int:

    # The supervisor itself: this code runs for as long as the loop does, so it does nothing but wait and start steps.
    def step(phase: str) -> int:
        cmd = [sys.executable, "-m", "cantrip_ai.loop", *argv, "--phase", phase, "--supervisor", str(os.getpid()),
               *(["--", *extra] if extra else [])]
        try:
            return subprocess.run(cmd, cwd=REPO / "ai", env=loop.environment(), stdin=subprocess.DEVNULL).returncode
        except OSError as e:
            loop.log(f"the {phase} step did not start: {e}")
            return 1

    loop.log(f"supervisor started (pid {os.getpid()}) for {root}: generation.py {' '.join(driver_args)}"
             + (f"; code updates from {args.update} between generations" if args.update else "; no code updates"))
    done = 0
    try:
        while True:
            if loop.boundary() == END:
                return 0
            code = step("update") if args.update else OK
            if code == OK:
                code = step("run")
                if code == OK:
                    done += 1
                    if args.max_generations and done >= args.max_generations:
                        loop.log(f"{done} generation(s) run, as asked: the loop ends")
                        loop.state("stopped", f"--max-generations {args.max_generations} reached")
                        return 0
            if code not in (OK, HELD, END, 130) and not (root / PAUSE).exists():
                # A step that crashed (new code that does not start, say) holds the loop as a failure does.
                loop.hold(f"the supervisor's own step failed (exit {code}); see {root / LOG} and the console")
            if code == END:
                return 0
            if code == 130:
                loop.log("the step was interrupted: the supervisor ends")
                loop.state("stopped", "interrupted")
                return 130
    except KeyboardInterrupt:
        loop.log("interrupted: the supervisor ends (a running driver hears the interrupt too and stops gracefully)")
        loop.state("stopped", "interrupted")
        return 130


if __name__ == "__main__":
    sys.exit(main())
