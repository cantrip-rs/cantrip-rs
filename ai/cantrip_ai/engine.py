"""A generation's engine, pinned (docs/ai-training.md, "The engine a generation runs"): which source its games are
played by, and the binaries built from it, kept apart from the working tree so that neither a commit pulled into the
tree nor a build made there changes what a running generation plays with.

A recording replays only with the engine that made it: the pair miner stands the gate's games in again position by
position, and one card's change makes a replay another game (`MismatchedPositions`). So a generation names its engine
once, when it starts (`generation.json`, `engine`), and keeps it:

- `identify`: the source's identity - the commit, and a hash of what the binaries are built from: git's tracked files
  under the Cargo workspace and what its build scripts read (`distributed.SOURCE_PATHS`, what a Linux worker builds
  from) and under `dotnet/` (BotLab deals the games), with the contents of whatever differs from the commit.
- `Engine`: the build of one identity under `ROOT/builds/<id>/`, as a Linux worker keeps one under `builds/<sha>`
  (`distributed.LINUX_BUILD`): each binary copied in once it is built (written whole, then renamed), never replaced,
  with the source tar a Linux worker builds the same engine from and `engine.json` saying what it is. Every stage
  runs `path(name)`: the pinned file; or, before it is pinned, the working tree's while the tree is still that source.
  A tree that has moved on is never used: a file the pinned build lacks is then an error that says how to get it.

No torch and no numpy: the driver imports this.
"""

from __future__ import annotations

import filecmp
import hashlib
import json
import os
import shutil
import subprocess
import time
from pathlib import Path

from . import distributed

EXE = ".exe" if os.name == "nt" else ""
BUILDS = "builds"
# What an identity covers: the Rust workspace as a worker builds it, and the C# that plays the duels.
PATHS = (*distributed.SOURCE_PATHS, "dotnet")
BOTLAB_RIDS = {"windows": ("win-x64", "cantrip-botlab.exe"), "linux": ("linux-x64", "cantrip-botlab")}


def layout(repo: Path) -> dict[str, tuple[Path, str]]:
    """Each thing a generation runs: where the build leaves it in the working tree, and where it is kept in a pinned
    build. The release binaries share a directory with the libraries beside them (`duels.windows_files` bundles the
    AI host's DLLs from there)."""
    release, botlab = repo / "target" / "release", repo / "dotnet/tools/Cantrip.BotLab/bin"
    out = {name: (release / f"cantrip-{name}{EXE}", f"release/cantrip-{name}{EXE}")
           for name in ("selfplay", "infer", "explore", "ai-host")}
    out["native-host"] = (repo / f"target/cards/cantrip-native-host{EXE}", f"cards/cantrip-native-host{EXE}")
    out["native-host-record"] = (repo / f"target/record/cards/cantrip-native-host{EXE}", f"record/cantrip-native-host{EXE}")
    out["botlab"] = (botlab / "Release/net8.0/cantrip-botlab.dll", "botlab/cantrip-botlab.dll")
    for kind, (rid, exe) in BOTLAB_RIDS.items():
        out[f"botlab-{kind}"] = (botlab / "publish" / rid / exe, f"botlab-publish/{rid}/{exe}")
    return out


# Kept as their whole directory: BotLab's build is its dll and everything beside it.
DIRECTORIES = ("botlab",)


def identify(repo: Path) -> dict:
    """The working tree's engine source: `id` (16 hex digits), the `commit` it is at and whether the tree differs from
    it (`dirty`). `id` is None where git cannot say (not a checkout): nothing is pinned then."""

    def git(*words) -> bytes:
        # No optional lock: `git status` would otherwise refresh the index under whatever else uses the tree.
        return subprocess.run(["git", *words], cwd=repo, capture_output=True, check=True, stdin=subprocess.DEVNULL,
                              env={**os.environ, "GIT_OPTIONAL_LOCKS": "0"}).stdout

    try:
        commit = git("rev-parse", "HEAD").decode().strip()
        h = hashlib.sha256(git("ls-files", "-s", "-z", "--", *PATHS))
        changed = [line[3:] for line in git("status", "--porcelain", "-z", "--untracked-files=all", "--", *PATHS)
                   .decode("utf-8", errors="replace").split("\0") if len(line) > 3]
    except (OSError, subprocess.CalledProcessError):
        return dict(id=None, commit=None, dirty=None)
    for rel in sorted(changed):
        h.update(rel.encode("utf-8"))
        try:
            h.update(hashlib.sha256((repo / rel).read_bytes()).digest())
        except OSError:
            h.update(b"-")  # deleted, or a rename's other name
    return dict(id=h.hexdigest()[:16], commit=commit, dirty=bool(changed))


def describe(ident: dict | None) -> str:
    if not ident or not ident.get("id"):
        return "unpinned"
    return f"{ident['id']} (commit {str(ident.get('commit'))[:10]}{', with local changes' if ident.get('dirty') else ''})"


class Engine:
    """The engine a generation runs: `pinned` its identity (`generation.json`'s), `current` the working tree's now."""

    def __init__(self, root: Path, repo: Path, pinned: dict | None, current: dict | None, log=print):
        self.root, self.repo, self.pinned, self.current, self.log = root, repo, pinned or {}, current or {}, log
        self.layout = layout(repo)

    @property
    def id(self) -> str | None:
        return self.pinned.get("id")

    @property
    def dir(self) -> Path | None:
        return self.root / BUILDS / self.id if self.id else None

    def same(self) -> bool:
        """Whether the working tree is still the pinned source (or nothing is pinned): its builds are then this
        engine's."""
        return not self.id or self.current.get("id") == self.id

    def path(self, name: str) -> Path:
        """What the generation runs for `name` (`layout`)."""
        tree, kept = self.layout[name]
        if self.id and (self.dir / kept).exists():
            if name == "botlab":
                # BotLab finds the repository from where it stands, and a pinned copy stands outside it.
                os.environ.setdefault("CANTRIP_ROOT", str(self.repo))
            return self.dir / kept
        if self.same():
            return tree
        raise SystemExit(
            f"this generation's engine is {describe(self.pinned)}, and its build {self.dir} has no {kept}; the working "
            f"tree is now {describe(self.current)}, another engine, so nothing built there is used (a recording "
            f"replays only with the engine that made it). Check out commit {self.pinned.get('commit')} and run the "
            "driver again, or pass --repin-engine to go on with the tree as it is now (the gate must then be played "
            "again before its pairs are mined)")

    def find(self, name: str) -> Path:
        """As `path`, for what a run may not need: where it is or would be, with no error when it is not there (the
        pinned build's place, when the tree is another engine: never the tree's)."""
        tree, kept = self.layout[name]
        if self.id and ((self.dir / kept).exists() or not self.same()):
            return self.dir / kept
        return tree

    def source(self) -> Path:
        """The source tar a Linux worker builds this engine from: the pinned build's, made from the working tree
        while the tree is still that source."""
        kept = self.dir / "src.tar.gz" if self.id else None
        if kept is not None and kept.exists():
            return kept
        if not self.same():
            raise SystemExit(f"this generation's engine is {describe(self.pinned)} and its build {self.dir} has no "
                             f"source for a Linux worker; the working tree is now {describe(self.current)}. Check out "
                             f"commit {self.pinned.get('commit')} and run the driver again")
        made = distributed.source_tar(self.repo, self.root / "dist" / "src")
        if kept is None:
            return made
        kept.parent.mkdir(parents=True, exist_ok=True)
        _put(made, kept)
        return kept

    def kept(self) -> bool:
        """Whether a build of this engine is pinned: only then do the stages run files nothing else can change, and
        only then is the engine's id written into their records."""
        return bool(self.id) and (self.dir / "engine.json").exists()

    def adopt(self, built, since: float | None = None, frozen: bool = False) -> list[str]:
        """After the build: what this run's build made (`built`, the names of `layout` its commands were run for)
        copied into the pinned build. Only those: a file the tree holds from some earlier build of another source (a
        BotLab published for a kind of worker this run did not publish for, a binary of a feature this run did not
        build) is no part of this engine, however it is named. With `since` (the build's start), a file older than
        it that the build's command did not vouch for is left out as well (`built` names what was vouched for;
        `since` is for a caller that has only the time). A kept file that differs from the tree's of the same
        source (its size or time: the same source built again) is replaced - until a stage has recorded this
        engine (`frozen`): from then on what is kept stays, and the difference is said, since the generation's
        ledgers name the binaries its games were begun with and refuse to resume with others. Nothing is taken from
        a tree that has moved on. Returns what was copied."""
        if not self.id or not self.same():
            return []
        taken = []
        for name, (tree, kept) in self.layout.items():
            dest = self.dir / kept
            if not tree.exists():
                continue
            if name not in built and not (since is not None and tree.stat().st_mtime >= since):
                continue
            if dest.exists() and filecmp.cmp(tree, dest, shallow=True):
                continue
            if dest.exists() and frozen:
                self.log(f"  engine {self.id}: WARNING: the tree's {tree.name} differs from the one this generation "
                         f"pinned ({dest}), the same source built again; the generation keeps the pinned one")
                continue
            if name in DIRECTORIES:
                tmp = dest.parent.with_name(dest.parent.name + ".tmp")
                if tmp.exists():
                    shutil.rmtree(tmp)
                shutil.copytree(tree.parent, tmp)
                # The old one renamed aside, the new one renamed in, the old one removed: never removed in place, where
                # a failure half way would leave no BotLab at all.
                aside = dest.parent.with_name(f"{dest.parent.name}.old-{os.getpid()}")
                try:
                    if dest.parent.exists():
                        os.replace(dest.parent, aside)
                    try:
                        os.replace(tmp, dest.parent)
                    except OSError:
                        if aside.exists():
                            os.replace(aside, dest.parent)
                        raise
                except OSError as e:  # in use (a run of this generation still plays with it): kept as it is
                    self.log(f"  engine {self.id}: {dest.parent} was not replaced ({e})")
                    shutil.rmtree(tmp, ignore_errors=True)
                    continue
                shutil.rmtree(aside, ignore_errors=True)
            else:
                dest.parent.mkdir(parents=True, exist_ok=True)
                try:
                    _put(tree, dest)
                    if kept.startswith("release/"):
                        # The libraries beside a release binary (onnxruntime's, the AI host's).
                        for lib in tree.parent.glob("*.dll"):
                            if not (dest.parent / lib.name).exists() or not filecmp.cmp(lib, dest.parent / lib.name, shallow=True):
                                _put(lib, dest.parent / lib.name)
                except OSError as e:
                    self.log(f"  engine {self.id}: {dest} was not replaced ({e})")
                    continue
            taken.append(kept)
        record = self.dir / "engine.json"
        if taken:  # nothing kept, nothing said: an `engine.json` is what says a build is pinned (`kept`)
            record.write_text(json.dumps(dict(self.pinned, pinned=time.strftime("%Y-%m-%d %H:%M:%S"),
                                              files=sorted(str(p.relative_to(self.dir).as_posix())
                                                           for p in self.dir.rglob("*") if p.is_file()
                                                           and p.name != "engine.json" and "botlab/" not in p.as_posix())),
                                         indent=1), encoding="utf-8")
        return taken


def _put(source: Path, dest: Path):
    """A file copied whole, then renamed into place (`LINUX_BUILD`'s `put`)."""
    tmp = dest.with_name(dest.name + ".tmp")
    shutil.copy2(source, tmp)
    os.replace(tmp, dest)


def prune(root: Path, keep: set[str], log=print) -> list[Path]:
    """The pinned builds no kept generation names, deleted (retention's part for `ROOT/builds`)."""
    gone = []
    builds = root / BUILDS
    for d in sorted(builds.iterdir()) if builds.is_dir() else []:
        if d.is_dir() and d.name not in keep:
            log(f"deleting {d} (no kept generation runs this engine)")
            shutil.rmtree(d, ignore_errors=True)
            gone.append(d)
    return gone


def named(root: Path, last: int) -> set[str]:
    """The engines whose builds are kept: those the newest `last` generations name, and those of every generation
    retention never retires - a pinned one (`retention.pinned`) and one whose model is a champion (`champions/genN`):
    their games are the ones somebody may want to replay."""
    from . import retention  # noqa: PLC0415

    out = set()
    gens = retention.generations(root)
    recent = {g for _, g in gens[-max(last, 1):]}
    for _, g in gens:
        if g not in recent and not retention.pinned(g, set()) and not (root / "champions" / g.name).exists():
            continue
        try:
            ident = json.loads((g / "generation.json").read_text(encoding="utf-8")).get("engine") or {}
        except (OSError, ValueError):
            continue
        if ident.get("id"):
            out.add(ident["id"])
    return out
