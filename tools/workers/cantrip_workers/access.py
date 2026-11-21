"""Reaching a leased machine (docs/workers.md, "Access"): a command run on it, a file pushed or pulled, a port
forwarded - each by the system's own `ssh` or `scp`, started here with the address and key only the pool knows, its
input and output the caller's own handles (no byte passes through Python), and ended when its lease ends.

A caller names a lease, never a machine's address. Every call is checked against the lease first and written to
`log.jsonl`. On a Linux machine a command runs with `CANTRIP_LEASE` set to its lease (`marked`), which is how a
lease's processes are found again when it ends (`sweep_line`).
"""

from __future__ import annotations

import contextlib
import os
import shutil
import socket
import subprocess
import sys
from pathlib import Path

from .store import LEASE_ENDED, LeaseError, Refused

SSH_OPTIONS = ["-o", "BatchMode=yes", "-o", "ConnectTimeout=10", "-o", "ServerAliveInterval=15",
               "-o", "ServerAliveCountMax=4", "-o", "StrictHostKeyChecking=accept-new"]
LEASE_VARIABLE = "CANTRIP_LEASE"
# The exit codes of a call the tool ended itself: its lease ended, or its --timeout passed.
ENDED, TIMED_OUT = 70, 124


def common_options(pool, m: dict) -> list[str]:
    """What ssh and scp share: the pool's own known-hosts file (a host seen for the first time is trusted and
    remembered there; one whose key changed is refused), a rented box under a name of its own (its address may have
    been another box's), and the key."""
    known = str(pool.store.known_hosts).replace("\\", "/")
    # Quoted only when it has to be: Windows' OpenSSH refuses a quoted value that reaches it through the C runtime's
    # escaped quotes ("command-line line 0: invalid quotes"), and a path without spaces needs none.
    out = [*SSH_OPTIONS, "-o", f'UserKnownHostsFile="{known}"' if any(c.isspace() for c in known) else f"UserKnownHostsFile={known}"]
    if m.get("alias"):
        out += ["-o", f"HostKeyAlias={m['alias']}"]
    key = pool.key_path(m)
    if key:
        out += ["-i", str(key)]
    return out


def ssh_command(pool, m: dict) -> list[str]:
    return ["ssh", *common_options(pool, m), *(["-p", str(m["port"])] if m.get("port") else []), f"{m['user']}@{m['host']}"]


def scp_command(pool, m: dict, recursive: bool = False) -> list[str]:
    return ["scp", "-q", "-B", *common_options(pool, m), *(["-P", str(m["port"])] if m.get("port") else []),
            *(["-r"] if recursive else [])]


def at(m: dict, path: str) -> str:
    return f"{m['user']}@{m['host']}:{path}"


def remote(pool, m: dict, line: str, timeout: float = 60.0, run=subprocess.run) -> tuple[int, str]:
    """The tool's own errands on a machine (a heartbeat, a sweep, a box's status): (exit code, output); 255 when it
    cannot be reached."""
    if m.get("local"):
        cmd = line if os.name == "nt" else ["bash", "-c", line]
        try:
            p = run(cmd, shell=os.name == "nt", capture_output=True, timeout=timeout, stdin=subprocess.DEVNULL)
        except (subprocess.TimeoutExpired, OSError):
            return 255, ""
        return p.returncode, p.stdout.decode(errors="replace")
    try:
        p = run([*ssh_command(pool, m), line], capture_output=True, timeout=timeout, stdin=subprocess.DEVNULL)
    except (subprocess.TimeoutExpired, OSError):
        return 255, ""
    return p.returncode, p.stdout.decode(errors="replace")


def marked(lease: str, line: str) -> str:
    """A command line for a Linux machine, run with its lease in its environment from its first process on: the
    user's own shell is started by `env`, in place of the login shell that ssh starts (which runs this one command
    and is replaced by it), so no shell is left outside the mark for a sweep to miss."""
    quoted = "'" + line.replace("'", "'\\''") + "'"
    return f'exec env {LEASE_VARIABLE}={lease} "${{SHELL:-sh}}" -c {quoted}'


def sweep_line(lease: str) -> str:
    """Kills every process on a Linux machine that was started under a lease, detached ones too: those whose
    environment names it."""
    return ("for f in /proc/[0-9]*/environ; do if grep -qaz '^" + LEASE_VARIABLE + "=" + lease + "$' \"$f\" 2>/dev/null; "
            "then p=${f#/proc/}; kill -9 \"${p%/environ}\" 2>/dev/null; fi; done; true")


def own_group() -> dict:
    """How a call's process is started so that all of it can be ended (`kill_tree`): a session of its own, where
    there are sessions."""
    return {} if os.name == "nt" else dict(start_new_session=True)


def kill_tree(proc):
    """Ends a process and everything it started: a shell's command with the shell, an ssh's helpers with the ssh."""
    pid = getattr(proc, "pid", None)
    if pid is not None:
        with contextlib.suppress(OSError, subprocess.SubprocessError):
            if os.name == "nt":
                subprocess.run(["taskkill", "/T", "/F", "/PID", str(pid)], capture_output=True, timeout=30)
            else:
                import signal  # noqa: PLC0415

                os.killpg(pid, signal.SIGKILL)
    with contextlib.suppress(OSError):
        proc.kill()


def spawn_detached(argv: list[str]):
    """A process that outlives this one, with no console and none of its handles."""
    quiet = dict(stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, close_fds=True)
    if os.name != "nt":
        return subprocess.Popen(argv, start_new_session=True, **quiet)
    flags = subprocess.CREATE_NO_WINDOW | subprocess.CREATE_NEW_PROCESS_GROUP
    try:
        # Out of the caller's job object where that is allowed, so it is not ended with the caller's process tree.
        return subprocess.Popen(argv, creationflags=flags | 0x01000000, **quiet)  # CREATE_BREAKAWAY_FROM_JOB
    except OSError:
        return subprocess.Popen(argv, creationflags=flags, **quiet)


class Access:
    """The calls a lease allows. `popen` starts a process (a test hands in its own); `interval` is how often a
    running call looks at its lease."""

    def __init__(self, manager, popen=subprocess.Popen, interval: float = 2.0, spawn=spawn_detached):
        self.m, self.popen, self.interval, self.spawn = manager, popen, interval, spawn

    def watch(self, lease: str, proc, timeout: float | None, alive=lambda: True) -> int:
        """Waits for `proc`; kills it when its lease ends (exit 70), `timeout` passes (124) or `alive` says no."""
        deadline = None if timeout is None else self.m.clock() + timeout
        try:
            while True:
                try:
                    return proc.wait(timeout=self.interval)
                except subprocess.TimeoutExpired:
                    pass
                over = ENDED if not (self.m.live(lease) and alive()) else TIMED_OUT if deadline is not None and self.m.clock() >= deadline else None
                if over is not None:
                    kill_tree(proc)
                    with contextlib.suppress(Exception):
                        proc.wait(timeout=10)
                    print("cantrip-workers: " + (f"{LEASE_ENDED} the lease ended: its call was stopped" if over == ENDED else
                                                 f"no end in {timeout:g} s: stopped"), file=sys.stderr)
                    return over
        except KeyboardInterrupt:
            kill_tree(proc)
            raise

    def logged(self, kind: str, lease: dict, started: float, code: int, **what):
        self.m.event(kind, lease=lease["id"], machine=lease["machine"], holder=lease["holder"], code=code,
                     seconds=round(self.m.clock() - started, 3), **what)

    def exec(self, lease_id: str, words: list[str], timeout: float | None = None) -> int:
        """Runs a command line on the lease's machine (the words joined by spaces, as ssh joins them), this process's
        input, output and errors its own. Its exit code; 255 when the machine cannot be reached."""
        if not words:
            raise Refused("exec needs a command", 2)
        lease, machine = self.m.resolve(lease_id)
        line = " ".join(words)
        started = self.m.clock()
        if machine.get("local"):
            proc = self.popen(line, shell=True, env={**os.environ, LEASE_VARIABLE: lease["id"]}, **own_group())
        else:
            if machine["os"] == "linux":
                line = marked(lease["id"], line)
            proc = self.popen([*ssh_command(self.m.pool, machine), line], **own_group())
        code = self.watch(lease_id, proc, timeout)
        self.logged("exec", lease, started, code, command=" ".join(words)[:400])
        return code

    def copy(self, kind: str, lease_id: str, src: str, dst: str, recursive: bool, timeout: float | None) -> int:
        lease, machine = self.m.resolve(lease_id)
        started = self.m.clock()
        if machine.get("local"):
            try:
                if Path(src).is_dir():
                    if not recursive:
                        raise Refused(f"{src} is a directory: -r", 2)
                    shutil.copytree(src, dst, dirs_exist_ok=True)
                else:
                    shutil.copyfile(src, dst)
                code = 0
            except OSError as e:
                print(f"cantrip-workers: {e}", file=sys.stderr)
                code = 1
        else:
            pair = [src, at(machine, dst)] if kind == "push" else [at(machine, src), dst]
            code = self.watch(lease_id, self.popen([*scp_command(self.m.pool, machine, recursive), *pair], **own_group()), timeout)
        self.logged(kind, lease, started, code, source=src, destination=dst)
        return code

    def push(self, lease_id: str, src: str, dst: str, recursive: bool = False, timeout: float | None = None) -> int:
        """A file (or with `recursive` a directory) of this machine's, copied to a path on the lease's machine."""
        return self.copy("push", lease_id, src, dst, recursive, timeout)

    def pull(self, lease_id: str, src: str, dst: str, recursive: bool = False, timeout: float | None = None) -> int:
        """A file or directory on the lease's machine, copied here."""
        return self.copy("pull", lease_id, src, dst, recursive, timeout)

    # Forwards: each a detached `forward-run` of this tool, which holds the ssh and ends it with the lease.

    def forward_file(self, lease_id: str, local: int) -> Path:
        return self.m.store.home / "forwards" / f"{lease_id}-{local}.json"

    def forward(self, lease_id: str, local: int, remote_port: int, launcher: list[str], wait: float = 20.0) -> int:
        """A port of this machine's loopback forwarded to one of the leased machine's loopback, until the lease ends
        or `unforward`. Returns the local port (`local` 0: a free one)."""
        lease, machine = self.m.resolve(lease_id)
        if machine.get("local"):
            raise Refused(f"{machine['name']} is this machine: its ports are already here", 2)
        if not local:
            with socket.socket() as s:
                s.bind(("127.0.0.1", 0))
                local = s.getsockname()[1]
        mark = self.forward_file(lease_id, local)
        if mark.exists():
            raise Refused(f"port {local} is already forwarded under this lease", 2)
        mark.parent.mkdir(parents=True, exist_ok=True)
        mark.write_text("{}", encoding="utf-8")
        self.spawn([*launcher, "forward-run", lease_id, str(local), str(remote_port)])
        deadline = self.m.clock() + wait
        while self.m.clock() < deadline and mark.exists():
            with contextlib.suppress(OSError), socket.create_connection(("127.0.0.1", local), timeout=1):
                self.m.event("forward", lease=lease["id"], machine=lease["machine"], holder=lease["holder"], local=local,
                             remote=remote_port)
                return local
            self.m.sleep(0.5)
        mark.unlink(missing_ok=True)
        raise Refused(f"the forward to {machine['name']}:{remote_port} did not come up", 1)

    def forward_run(self, lease_id: str, local: int, remote_port: int) -> int:
        """The process a forward lives in: `ssh -N -L`, ended with the lease or when its mark is removed."""
        _, machine = self.m.resolve(lease_id)
        mark = self.forward_file(lease_id, local)
        proc = self.popen([*ssh_command(self.m.pool, machine)[:-1], "-N", "-o", "ExitOnForwardFailure=yes",
                           "-L", f"127.0.0.1:{local}:127.0.0.1:{remote_port}", ssh_command(self.m.pool, machine)[-1]],
                          stdin=subprocess.DEVNULL, **own_group())
        try:
            return self.watch(lease_id, proc, None, alive=mark.exists)
        finally:
            mark.unlink(missing_ok=True)

    def unforward(self, lease_id: str, local: int) -> bool:
        mark = self.forward_file(lease_id, local)
        if not mark.exists():
            return False
        mark.unlink()
        return True

    def forwards(self) -> list[dict]:
        out = []
        for p in sorted((self.m.store.home / "forwards").glob("*.json")):
            lease, _, port = p.stem.rpartition("-")
            try:
                self.m.resolve(lease)
            except LeaseError:
                p.unlink(missing_ok=True)  # its process ended with the lease, or never ran
                continue
            out.append(dict(lease=lease, local=int(port)))
        return out
