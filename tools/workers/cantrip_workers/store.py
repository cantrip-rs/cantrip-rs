"""Where the worker manager keeps what it knows, all of it outside the repository (docs/workers.md, "The local
directory"): the pool's configuration, the leases, the rented instances, the cost ledger and the log of every call.

    pool.json       the machines, the SSH key, the vast.ai limits (written by hand, by `init` and by `import`)
    state.json      the live leases and the rented instances (written whole, then renamed, under `state.lock`)
    ledger.jsonl    every rental and destruction, with its price and duration (appended, never rewritten)
    log.jsonl       every lease, release, expiry, exec, push, pull and forward (appended)
    known_hosts     the host keys of the pool's machines (ssh's own file, apart from the user's)
    daemon.lock     held by the daemon while it runs

The directory is `CANTRIP_WORKERS_HOME`, else `%LOCALAPPDATA%\\cantrip-workers` on Windows and
`$XDG_CONFIG_HOME/cantrip-workers` (`~/.config/cantrip-workers`) elsewhere.
"""

from __future__ import annotations

import contextlib
import json
import os
import time
from pathlib import Path

HOME_VARIABLE = "CANTRIP_WORKERS_HOME"


class Refused(Exception):
    """What the tool will not do, said in one line; `code` is the command's exit code (docs/workers.md, "Exit codes")."""

    code = 1

    def __init__(self, message: str, code: int | None = None):
        super().__init__(message)
        if code is not None:
            self.code = code


class ConfigError(Refused):
    code = 73


# What the tool's own "this lease is not live" carries on stderr, with exit 70: a command run by `exec` can exit 70
# too, and only the tool writes this.
LEASE_ENDED = "[lease-ended]"


class LeaseError(Refused):
    """No such lease, or one that ended."""

    code = 70

    def __str__(self) -> str:
        return f"{LEASE_ENDED} {super().__str__()}"


class Unmet(Refused):
    """A request no machine can satisfy now."""

    code = 71


class LimitError(Refused):
    """A rental the limits do not allow."""

    code = 72


def default_home() -> Path:
    if os.environ.get(HOME_VARIABLE):
        return Path(os.environ[HOME_VARIABLE])
    if os.name == "nt":
        base = os.environ.get("LOCALAPPDATA") or str(Path.home() / "AppData" / "Local")
        return Path(base) / "cantrip-workers"
    return Path(os.environ.get("XDG_CONFIG_HOME") or Path.home() / ".config") / "cantrip-workers"


def read_json(path: Path, default=None):
    for attempt in range(20):
        try:
            return json.loads(Path(path).read_text(encoding="utf-8"))
        except FileNotFoundError:
            return default
        except PermissionError:  # Windows: a writer is putting it in place this instant
            if attempt == 19:
                raise
            time.sleep(0.05)


def write_atomic(path: Path, text: str):
    path = Path(path)
    tmp = path.with_name(path.name + f".{os.getpid()}.tmp")
    tmp.write_text(text, encoding="utf-8")
    for attempt in range(20):
        try:
            os.replace(tmp, path)
            return
        except PermissionError:  # Windows: a reader has it open this instant
            if attempt == 19:
                raise
            time.sleep(0.05)


def append_line(path: Path, record: dict):
    """One JSON line appended in a single write, so concurrent writers' lines never interleave."""
    path.parent.mkdir(parents=True, exist_ok=True)
    data = (json.dumps(record, sort_keys=True) + "\n").encode("utf-8")
    fd = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o600)
    try:
        os.write(fd, data)
    finally:
        os.close(fd)


def read_lines(path: Path) -> list[dict]:
    try:
        text = Path(path).read_text(encoding="utf-8")
    except FileNotFoundError:
        return []
    out = []
    for line in text.splitlines():
        if line.strip():
            with contextlib.suppress(ValueError):
                out.append(json.loads(line))
    return out


class FileLock:
    """An exclusive lock on a file, the operating system's: it is let go when its process ends, however it ends.
    `blocking=False` raises `BlockingIOError` when another process holds it."""

    def __init__(self, path: Path, blocking: bool = True, timeout: float = 60.0):
        self.path, self.blocking, self.timeout = Path(path), blocking, timeout
        self.f = None

    def _try(self) -> bool:
        try:
            if os.name == "nt":
                import msvcrt  # noqa: PLC0415

                self.f.seek(0)
                msvcrt.locking(self.f.fileno(), msvcrt.LK_NBLCK, 1)
            else:
                import fcntl  # noqa: PLC0415

                fcntl.flock(self.f.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            return True
        except OSError:
            return False

    def acquire(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        self.f = open(self.path, "a+b")  # noqa: SIM115
        deadline = time.monotonic() + self.timeout
        while not self._try():
            if not self.blocking or time.monotonic() > deadline:
                self.f.close()
                self.f = None
                raise BlockingIOError(f"{self.path} is held by another process")
            time.sleep(0.05)
        return self

    def release(self):
        if self.f is None:
            return
        with contextlib.suppress(OSError):
            if os.name == "nt":
                import msvcrt  # noqa: PLC0415

                self.f.seek(0)
                msvcrt.locking(self.f.fileno(), msvcrt.LK_UNLCK, 1)
            else:
                import fcntl  # noqa: PLC0415

                fcntl.flock(self.f.fileno(), fcntl.LOCK_UN)
        self.f.close()
        self.f = None

    def __enter__(self):
        return self.acquire()

    def __exit__(self, *_):
        self.release()


class Store:
    """The local directory's files."""

    def __init__(self, home: Path | None = None):
        self.home = Path(home) if home else default_home()

    @property
    def pool(self) -> Path:
        return self.home / "pool.json"

    @property
    def state(self) -> Path:
        return self.home / "state.json"

    @property
    def ledger(self) -> Path:
        return self.home / "ledger.jsonl"

    @property
    def log(self) -> Path:
        return self.home / "log.jsonl"

    @property
    def known_hosts(self) -> Path:
        return self.home / "known_hosts"

    def lock(self) -> FileLock:
        return FileLock(self.home / "state.lock")

    def daemon_lock(self) -> FileLock:
        return FileLock(self.home / "daemon.lock", blocking=False)

    def read_state(self) -> dict:
        s = read_json(self.state) or {}
        s.setdefault("leases", {})
        s.setdefault("rented", {})
        return s

    def write_state(self, state: dict):
        self.home.mkdir(parents=True, exist_ok=True)
        write_atomic(self.state, json.dumps(state, indent=1, sort_keys=True) + "\n")
