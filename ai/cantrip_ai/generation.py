"""One generation of expert iteration (docs/ai-player-plan.md, "Phase 4"): self-play, train, export, parity, gate,
promote, retire.

    python -m cantrip_ai.generation --root ../../ai-data [--generation N] [--games 5000] [--simulations 64]
        [--decks-dir decks/standard] [--device auto] [--train-hours 0.65] [--gate-games 6000] [--gate-h1 0.52] [--dry-run] [--tiny]

Self-play draws its pairings from `--decks-dir` by metagame share (the player count in each decklist's header), and
the gate plays a pool of those decks as mirrors. `--decks-dir ''` keeps the fixed rw-aggro/ub-reanimator slice.

The champion (`ROOT/champions/current`, a frozen model directory with its `train/checkpoint.pt`) plays itself in
`cantrip-selfplay`, both seats searching at the budget, and the games go to `ROOT/genN/selfplay` with the search's root
visits as the policy targets. The network is fine-tuned from the champion's checkpoint on the window of generations
(the newest weighted 1, older self-play `--older-weight`, the imitation data gen0 `--imitation-weight`), exported to
`ROOT/models/genN`, checked for parity against PyTorch, and gated: the candidate against the champion in BotLab, both
searching at the same budget, the SPRT (H0 50%, H1 `--gate-h1` 52%, alpha = beta = 0.05) over the decks as mirrors,
seats swapped. Accepted, the candidate is copied to `ROOT/champions/genN` and becomes `current`; rejected, it is
recorded and the champion stays. Either way `champions/history.json` gets a line, and the generation's data stays in
the window (a rejected generation's games are still the champion's search). Then retention (`retention.py`).

Each stage's time and numbers go to `ROOT/genN/generation.json` as it finishes, and `ROOT/genN/README.md` is rewritten
from it; a stage already done there is not run again, so an interrupted generation resumes where it stopped (the
build runs every time, so a resumed run never uses a stale binary). A gate that finished but was not recorded (the
driver stopped between BotLab exiting and the stage being written) is read back from `genN/logs/gate.log` and
promoted, so the long gate is not rolled again. Within a stage, self-play keeps its closed shards and plays only the
games they do not hold, and training resumes from its last checkpoint. Every measurement is in the information mode
`--mode` (Full).

`--retrain` runs a rejected generation again from its training on (its self-play kept): the earlier attempt's stages
go to `attempts` in `generation.json` (and a line of the README), its logs to `logs/attemptK/`, its training metrics
beside them, and its model is deleted; the new attempt's settings (`--value-mix`, `--train-steps`, `--lr`) are the
command line's.

With `--gpu-server` (on by default when `--device` resolves to an NVIDIA GPU), self-play, parity and the gate evaluate
on the inference server (`serve.py`, docs/ai-training.md "The GPU inference server") instead of ONNX Runtime on the
CPU: the driver starts it for the stage on a free port, serving the stage's models, passes `--backend remote --server`
(BotLab: `--ai-server`), and stops it when the stage ends. Its log is `genN/logs/server-<stage>.log`. A model exported
before export wrote `model.pt` gets one from its checkpoint first. With the server, `--lanes` and `--gate-parallel`
default to more than the cores, since the games wait on the GPU. `--server-impl native` serves with `cantrip-infer`
instead (docs/ai-host.md, "The native inference server": the same protocol, no Python on the request path; built with
`--features gpu`, onnxruntime-gpu named by `--ort-dylib` or `ORT_DYLIB_PATH`, CUDA's libraries from PyTorch's).

`--dry-run` prints the stages' commands and the window and runs nothing. `--tiny` sets a toy scale (a handful of
games at 8 simulations, a few training steps, a gate of 4 games, no SPRT stop) for testing the driver end to end.
"""

from __future__ import annotations

import argparse
import contextlib
import json
import os
import random
import re
import shlex
import shutil
import subprocess
import sys
import time
from pathlib import Path

from . import retention

REPO = Path(__file__).resolve().parents[2]
STAGES = ("build", "selfplay", "train", "export", "parity", "gate", "promote", "retire")
EXE = ".exe" if os.name == "nt" else ""
BOTLAB = REPO / "dotnet/tools/Cantrip.BotLab/bin/Release/net8.0/cantrip-botlab.dll"
NATIVE = REPO / f"target/cards/cantrip-native-host{EXE}"
AI_HOST = REPO / f"target/release/cantrip-ai-host{EXE}"
SELFPLAY = REPO / f"target/release/cantrip-selfplay{EXE}"
INFER = REPO / f"target/release/cantrip-infer{EXE}"
# With the GPU server, lanes and gate games a CPU thread: they wait on the server, so more than the cores keep the GPU
# fed (measured on 32 threads and an RTX 4080; docs/ai-training.md).
SERVER_LANES = 2
SERVER_GATE = 1


def dotnet() -> list[str]:
    """The command the build and the gate run `dotnet` with: `CANTRIP_DOTNET` if it is set (a command line, e.g.
    `nix shell nixpkgs#dotnet-sdk_10 -c dotnet`), else the `dotnet` on PATH, else the .NET SDK through Nix."""
    if command := os.environ.get("CANTRIP_DOTNET"):
        return shlex.split(command)
    if shutil.which("dotnet"):
        return ["dotnet"]
    if shutil.which("nix"):
        return ["nix", "shell", "nixpkgs#dotnet-sdk_10", "-c", "dotnet"]
    sys.exit("no dotnet: install the .NET SDK (8 or later), or set CANTRIP_DOTNET to the command that runs it")


# --- Where things are ---------------------------------------------------------------------------------------------


def champion(root: Path) -> Path | None:
    current = root / "champions" / "current"
    return current.resolve() if current.exists() else None


def completed(root: Path, gen: int) -> bool:
    """Whether the driver wrote the generation's record all the way to the end."""
    try:
        return bool(json.loads((root / f"gen{gen}" / "generation.json").read_text(encoding="utf-8")).get("finished"))
    except (FileNotFoundError, json.JSONDecodeError):
        return False


def next_generation(root: Path) -> int:
    """The generation to make: the latest one started but not finished, else the one after the latest.

    gen0 is the imitation model, not a generation, so the first is 1. An interrupted generation is resumed rather
    than skipped: picking `max + 1` would strand its self-play and model under a number every later run passes over.
    """
    gens = [n for n, _ in retention.generations(root) if n >= 1]
    if gens and not completed(root, gens[-1]):
        return gens[-1]
    return (gens[-1] if gens else 0) + 1


def imitation_dirs(root: Path) -> list[Path]:
    """gen0's data: the tactical bot's games, one directory a pairing."""
    g = root / "gen0"
    return sorted(p for p in g.iterdir() if p.is_dir() and any(p.glob("*.shard.zst"))) if g.exists() else []


PLAYERS = re.compile(r"\((\d+)\s+players?\b", re.IGNORECASE)


def deck_pool(directory: Path) -> list[tuple[str, int]]:
    """A folder of decklists, with the metagame weight in each header's `(N players, ...)`; 1 when it has none."""
    pool = []
    for path in sorted(directory.glob("*.txt")):
        head = path.read_text(errors="replace").splitlines()[:1]
        found = PLAYERS.search(head[0]) if head else None
        pool.append((path.stem, int(found.group(1)) if found else 1))
    if not pool:
        raise SystemExit(f"no decklists in {directory}")
    return pool


def metagame_pairings(pool: list[tuple[str, int]], games: int, seed: int) -> list[str]:
    """`games` matchups drawn by metagame share (the players in each decklist's header), at most 4096 distinct slots.

    The self-play schedule cycles this list and swaps the two decks every other pass, so both seats meet each list.
    """
    names = [name for name, _ in pool]
    weights = [weight for _, weight in pool]
    rng = random.Random(seed)
    return [f"{rng.choices(names, weights)[0]}:{rng.choices(names, weights)[0]}"
            for _ in range(min(games, 4096))]


def window(root: Path, gen: int, size: int, older: float, imitation: float) -> list[tuple[Path, float]]:
    """The training window: the self-play data of the last `size` generations up to `gen`, newest at weight 1, and gen0."""
    out: list[tuple[Path, float]] = []
    for n in range(gen, max(gen - size, 0), -1):
        d = root / f"gen{n}" / "selfplay"
        if n == gen or (d.exists() and any(d.glob("*.shard.zst"))):  # the generation's own: its self-play stage made it
            out.append((d, 1.0 if n == gen else older))
    if imitation > 0:
        out += [(d, imitation) for d in imitation_dirs(root)]
    return out


def bootstrap(root: Path, log=print) -> Path:
    """The first champion: gen0's model, frozen under champions/ (pinned: champions are never retired)."""
    if (c := champion(root)) is not None:
        return c
    model = root / "models" / "gen0"
    if not (model / "manifest.json").exists():
        raise SystemExit(f"no champion and no {model}: train gen0 first (docs/ai-player-plan.md, 'Phase 2')")
    dest = root / "champions" / "gen0"
    if not dest.exists():
        shutil.copytree(model, dest)
    set_current(root, "gen0")
    history(root, dict(generation=0, accepted=True, verdict="bootstrap", note="gen0, the imitation model, is the first champion", at=now()))
    log(f"champion: gen0 (bootstrapped from {model})")
    return dest.resolve()


def _unlink(path: Path):
    """Removes a symlink or a directory junction (never what it points to)."""
    if path.is_symlink() or path.is_file():
        path.unlink()
    elif path.exists() or os.path.lexists(path):
        os.rmdir(path)  # a junction: rmdir removes the link, never the directory it points at


def set_current(root: Path, name: str):
    """`champions/current` to `champions/NAME`: a relative symlink, replaced atomically. Windows makes a symlink only in
    Developer Mode or as an administrator; without one, a directory junction (absolute, made by any user) stands in,
    and it is swapped by removing the old one first."""
    link = root / "champions" / "current"
    tmp = link.with_name("current.tmp")
    _unlink(tmp)
    try:
        tmp.symlink_to(name, target_is_directory=True)
    except OSError:
        if os.name != "nt":
            raise
        import _winapi  # noqa: PLC0415 (Windows only)

        _winapi.CreateJunction(str((root / "champions" / name).resolve()), str(tmp))
        _unlink(link)
        os.rename(tmp, link)
        return
    try:
        os.replace(tmp, link)
    except OSError:
        if os.name != "nt":
            raise
        _unlink(link)  # Windows cannot replace a directory link in one step
        os.rename(tmp, link)


def history(root: Path, entry: dict):
    path = root / "champions" / "history.json"
    rows = json.loads(path.read_text(encoding="utf-8")) if path.exists() else []
    rows.append(entry)
    path.write_text(json.dumps(rows, indent=1), encoding="utf-8")


def now() -> str:
    return time.strftime("%Y-%m-%d %H:%M:%S")


# --- Reading results ----------------------------------------------------------------------------------------------

RESULT = re.compile(r"^(\S+) beat (\S+) (\d+) to (\d+)(?:, (\d+) drawn)? \(([\d.]+) % \+/- ([\d.]+) %\)\.", re.M)
SPRT = re.compile(r"^SPRT \(.*?\): LLR (-?[\d.]+) after (\d+) games; (.*)$", re.M)


def parse_gate(text: str) -> dict:
    """BotLab's duel summary: the candidate (A) against the champion (B), and the SPRT's verdict."""
    m = RESULT.search(text)
    if not m:
        raise ValueError("no result line in the gate's output")
    wins, losses, draws = int(m.group(3)), int(m.group(4)), int(m.group(5) or 0)
    out = dict(wins=wins, losses=losses, draws=draws, games=wins + losses + draws, share=float(m.group(6)) / 100,
               margin=float(m.group(7)) / 100)
    s = SPRT.search(text)
    if s:
        llr, tail = float(s.group(1)), s.group(3)
        out.update(llr=llr, sprt=tail)
        out["verdict"] = "H1" if tail.startswith("H1") else "H0" if tail.startswith("H0") else "none"
        at = re.search(r"at game (\d+)", tail)
        out["stopped_at"] = int(at.group(1)) if at else None
    else:
        out.update(llr=None, sprt=None, verdict="none", stopped_at=None)
    return out


def accepted(gate: dict, rule: str) -> bool:
    """`sprt`: H1 accepted. `ahead`: the share above a half by more than the margin (no SPRT verdict needed)."""
    if rule == "sprt":
        return gate["verdict"] == "H1"
    return gate["share"] - gate["margin"] > 0.5


# --- The record ---------------------------------------------------------------------------------------------------


class Record:
    def __init__(self, gen_dir: Path, gen: int, dry_run: bool):
        self.dir, self.gen, self.dry_run = gen_dir, gen, dry_run
        self.path = gen_dir / "generation.json"
        self.data = json.loads(self.path.read_text(encoding="utf-8")) if self.path.exists() else dict(generation=gen, started=now(), stages={})

    def done(self, stage: str) -> bool:
        return self.data["stages"].get(stage, {}).get("done", False)

    def put(self, stage: str, seconds: float, **numbers):
        self.data["stages"][stage] = dict(done=True, seconds=round(seconds, 1), finished=now(), **numbers)
        self.save()

    def save(self):
        if self.dry_run:
            return
        self.dir.mkdir(parents=True, exist_ok=True)
        self.path.write_text(json.dumps(self.data, indent=1), encoding="utf-8")
        (self.dir / "README.md").write_text(readme(self.data), encoding="utf-8")


def _f(v, fmt="{:.3f}"):
    return "-" if v is None else fmt.format(v)


def readme(d: dict) -> str:
    s = d["stages"]
    gen = d["generation"]
    lines = [f"# Generation {gen}", "", f"Expert iteration (docs/ai-player-plan.md, \"Phase 4\"), {d.get('mode', 'full')} mode throughout. "
             f"Started {d['started']}; champion at the start: {d.get('champion', '?')}. Written by `ai/cantrip_ai/generation.py` "
             "from `generation.json`.", "", "| stage | time | numbers |", "|---|---|---|"]

    def row(stage, text):
        if stage in s:
            lines.append(f"| {stage} | {s[stage]['seconds'] / 60:.1f} min | {text} |")

    if "selfplay" in s:
        r = s["selfplay"]
        row("selfplay", f"{r['games']} games at @{r['simulations']} on {r['lanes']} lanes ({r['games_per_hour']:.0f} an hour), "
                        f"{r['samples']} samples ({r['visit_samples']} with visit targets), {r['capped']} capped, "
                        f"{r['dropped']} dropped, {r['fallbacks']} network fallbacks"
                        + (f", {r['resumed']} resumed" if r.get('resumed') else "")
                        + (f", {r['decks']}" if r.get('decks') else "")
                        + f"; first player won {r['first_player']:.1%}; seeds {r['seeds']}")
    if "train" in s:
        r = s["train"]
        h = r.get("held_out") or {}
        row("train", f"{r['steps']} steps, {r['seen']} samples seen, from {r['init']}; window: "
                     + ", ".join(f"{Path(w['directory']).parent.name}/{Path(w['directory']).name} x{w['weight']} ({w['samples']})" for w in r["window"])
                     + f"; held out: top1 {_f(h.get('top1'))}, top1 of the visits {_f(h.get('top1_visits'))}, "
                       f"policy CE {_f(h.get('policy_ce'))}, value MSE {_f(h.get('value_mse'))}, value sign {_f(h.get('value_sign'))}")
    if "export" in s:
        row("export", f"{s['export']['model']}")
    if "parity" in s:
        row("parity", s["parity"]["summary"])
    if "gate" in s:
        r = s["gate"]
        drawn = f", {r['draws']} drawn" if r["draws"] else ""
        row("gate", f"candidate gen{gen} against champion {r['champion']} at @{r['simulations']}, {r['decks']}: "
                    f"{r['wins']} to {r['losses']}{drawn} "
                    f"({r['share']:.1%} +/- {r['margin']:.1%}) over {r['games']} games; SPRT LLR {_f(r['llr'])}: {r['sprt'] or 'not run'}")
    if "promote" in s:
        r = s["promote"]
        row("promote", ("accepted: gen{0} is the champion".format(gen) if r["accepted"] else f"rejected: {r['champion']} stays the champion")
            + f" ({r['rule']})")
    if "retire" in s:
        row("retire", f"{s['retire']['deleted']} path(s) deleted, the last {s['retire']['keep']} generations kept")
    for k, at in enumerate(d.get("attempts", []), 1):
        g, t = at["stages"].get("gate", {}), at["stages"].get("train", {})
        h = t.get("held_out") or {}
        lines.append(f"| attempt {k} | {sum(v['seconds'] for v in at['stages'].values()) / 60:.1f} min | rejected and retrained "
                     f"({at.get('reason', '')}): {t.get('steps')} steps, value mix {t.get('value_mix')}, held-out value MSE "
                     f"{_f(h.get('value_mse'))}; gate {g.get('wins')} to {g.get('losses')} ({_f(g.get('share'), '{:.1%}')} +/- "
                     f"{_f(g.get('margin'), '{:.1%}')}), SPRT LLR {_f(g.get('llr'))}: {g.get('sprt')} |")
    total = sum(v["seconds"] for v in s.values()) + sum(v["seconds"] for at in d.get("attempts", []) for v in at["stages"].values())
    lines += ["", f"In all {total / 3600:.2f} h.", ""]
    return "\n".join(lines)


# --- Running ------------------------------------------------------------------------------------------------------


def retrain(rec: Record, gen_dir: Path, model_dir: Path, args, log=print):
    """Set a rejected generation's attempt aside, from its training on, so the stages run again."""
    s = rec.data["stages"]
    if not s.get("promote", {}).get("done") or s["promote"].get("accepted"):
        raise SystemExit(f"--retrain: gen{rec.gen} has no rejected attempt to set aside")
    later = ("train", "export", "parity", "gate", "promote", "retire")
    k = len(rec.data.get("attempts", [])) + 1
    log(f"setting attempt {k} aside ({args.retrain}); self-play kept")
    if args.dry_run:
        return
    keep = gen_dir / "logs" / f"attempt{k}"
    keep.mkdir(parents=True, exist_ok=True)
    for stage in later:
        p = gen_dir / "logs" / f"{stage}.log"
        if p.exists():
            p.rename(keep / p.name)
    metrics = model_dir / "train" / "metrics.json"
    if metrics.exists():
        shutil.copy(metrics, keep / "metrics.json")
    if model_dir.exists():
        shutil.rmtree(model_dir)
    rec.data.setdefault("attempts", []).append(dict(reason=args.retrain, stages={n: s.pop(n) for n in later if n in s}))
    rec.data.pop("finished", None)
    rec.save()


def run(cmd: list[str], log_path: Path, dry_run: bool, env: dict | None = None, log=print, stream=False) -> str:
    shown = " ".join(str(c) for c in cmd)
    if len(shown) > 2000:
        shown = f"{shown[:2000]} ... ({len(cmd)} arguments)"
    log(f"$ {shown}")
    if dry_run:
        return ""
    log_path.parent.mkdir(parents=True, exist_ok=True)
    environment = {**os.environ, **(env or {})}
    if stream:
        # Tee the stage's output here as well as the log, so its progress bar is visible while the driver runs.
        with log_path.open("wb") as f:
            p = subprocess.Popen([str(c) for c in cmd], cwd=REPO, env=environment, stdout=subprocess.PIPE,
                                 stderr=subprocess.STDOUT, bufsize=0)
            assert p.stdout is not None
            while chunk := p.stdout.read(4096):
                f.write(chunk)
                f.flush()
                try:
                    sys.stderr.buffer.write(chunk)
                    sys.stderr.buffer.flush()
                except AttributeError:
                    sys.stderr.write(chunk.decode(errors="replace"))
                    sys.stderr.flush()
            code = p.wait()
    else:
        with log_path.open("w") as f:
            p = subprocess.run([str(c) for c in cmd], cwd=REPO, env=environment, stdout=f, stderr=subprocess.STDOUT)
        code = p.returncode
    text = log_path.read_text(encoding="utf-8", errors="replace")
    if code != 0:
        raise SystemExit(f"failed ({code}): {shown}\n--- {log_path} ---\n{text[-3000:]}")
    return text


class InferenceServer:
    """`python -m cantrip_ai.serve` for a stage (docs/ai-training.md, "The GPU inference server"): started serving
    `models` on a free port, its address read from its ready file, stopped when the stage is done. Its output is the
    stage's `server-<stage>.log`."""

    def __init__(self, models: list[Path], log_path: Path, device: str, extra: list[str], dry_run: bool, log=print,
                 native: bool = False, ort_dylib: str | None = None):
        self.models, self.log_path, self.device, self.extra, self.dry_run, self.log = models, log_path, device, extra, dry_run, log
        self.native, self.ort_dylib = native, ort_dylib
        self.process = None
        self.address = "127.0.0.1:PORT"

    def command(self, ready: Path) -> tuple[list, dict]:
        """serve.py, or with `native` cantrip-infer (`--device` as a CUDA index; onnxruntime-gpu and CUDA's libraries
        named in its environment, CUDA's from this PyTorch's when not set)."""
        models = [x for m in self.models for x in ("--model", m)]
        env = {"PYTHONPATH": str(REPO / "ai")}
        if not self.native:
            return [sys.executable, "-m", "cantrip_ai.serve", *models, "--port", 0, "--ready-file", ready, "--device",
                    self.device, *self.extra], env
        device = self.device.split(":", 1)[1] if ":" in self.device else "0"
        if self.ort_dylib:
            env["ORT_DYLIB_PATH"] = self.ort_dylib
        if "CANTRIP_CUDA_LIBS" not in os.environ:
            with contextlib.suppress(ImportError):
                import torch  # noqa: PLC0415

                env["CANTRIP_CUDA_LIBS"] = str(Path(torch.__file__).parent / "lib")
        return [INFER, "serve", *models, "--port", 0, "--ready-file", ready, "--device", device, *self.extra], env

    def __enter__(self):
        ready = self.log_path.with_suffix(".ready")
        cmd, env = self.command(ready)
        self.log(f"$ {' '.join(str(c) for c in cmd)} &")
        if self.dry_run:
            return self
        self.log_path.parent.mkdir(parents=True, exist_ok=True)
        if ready.exists():
            ready.unlink()
        self.file = self.log_path.open("wb")
        self.process = subprocess.Popen([str(c) for c in cmd], cwd=REPO / "ai", stdout=self.file, stderr=subprocess.STDOUT,
                                        env={**os.environ, **env})
        deadline = time.time() + 600
        while not ready.exists():
            if self.process.poll() is not None or time.time() > deadline:
                self.stop()
                text = self.log_path.read_text(encoding="utf-8", errors="replace")
                raise SystemExit(f"the inference server did not start\n--- {self.log_path} ---\n{text[-3000:]}")
            time.sleep(0.2)
        self.address = ready.read_text().strip()
        ready.unlink()
        self.log(f"  inference server at {self.address} (log {self.log_path})")
        return self

    def stop(self):
        if self.process is not None and self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(30)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        if self.process is not None:
            self.file.close()
        self.process = None

    def __exit__(self, *_):
        self.stop()


def wants_server(choice: str, device: str) -> bool:
    """`--gpu-server`: on, off, or auto: on when the training device is an NVIDIA GPU. A ROCm build of PyTorch also
    calls its GPU `cuda`, but the server has not been run on AMD hardware yet, so there auto keeps ONNX Runtime on the
    CPU, as before, and `--gpu-server on` asks for the server."""
    if choice != "auto":
        return choice == "on"
    if device != "auto" and not device.startswith("cuda"):
        return False
    try:
        import torch  # noqa: PLC0415
    except ImportError:
        return False
    return torch.cuda.is_available() and torch.version.hip is None


def main(argv=None):
    a = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    a.add_argument("--root", required=True, help="the ai-data directory")
    a.add_argument("--generation", type=int, help="the generation to make (default: the one after the latest)")
    a.add_argument("--mode", default="full", choices=["full", "decklist", "hidden"])
    a.add_argument("--decks-dir", default="decks/standard",
                   help="a folder of decklists for self-play and the gate; empty for rw-aggro/ub-reanimator")
    a.add_argument("--device", default="auto", help="training device: auto, cpu, or cuda (HIP on ROCm)")
    a.add_argument("--progress", choices=["auto", "on", "off"], default="auto",
                   help="show the running stage's output (and its bar) in this terminal; auto is on only on a tty")
    a.add_argument("--games", type=int, default=5000, help="self-play games")
    a.add_argument("--simulations", type=int, default=64, help="the search budget of self-play and the gate")
    a.add_argument("--full-simulations", type=int, default=0,
                   help="playout-cap randomisation in self-play: this budget for a share --full-share of searched decisions, "
                        "the only policy targets; the rest search with --simulations (0: off, every search --simulations)")
    a.add_argument("--full-share", type=float, default=0.25, help="the share of searched decisions given --full-simulations")
    a.add_argument("--search-batch", type=int, default=8, help="leaves a self-play search sends to the network at once")
    a.add_argument("--lanes", type=int, help="self-play games at once (default: one a CPU thread; with the GPU server, "
                                              f"{SERVER_LANES} a CPU thread)")
    a.add_argument("--gpu-server", choices=["auto", "on", "off"], default="auto",
                   help="self-play, parity and the gate evaluate on one batched inference server (serve.py) instead of "
                        "ONNX Runtime on the CPU; auto is on when --device is an NVIDIA GPU (on ROCm, ask with on)")
    a.add_argument("--server-device", help="the inference server's device (default: --device)")
    a.add_argument("--server-args", default="", help="more arguments for serve.py, one string")
    a.add_argument("--server-impl", choices=["python", "native"], default="python",
                   help="the inference server: serve.py, or cantrip-infer (native, built with --features gpu; NVIDIA only)")
    a.add_argument("--ort-dylib", default=os.environ.get("ORT_DYLIB_PATH"),
                   help="onnxruntime-gpu's onnxruntime library for --server-impl native (default: ORT_DYLIB_PATH)")
    a.add_argument("--selfplay-shard-games", type=int, default=100,
                   help="games a self-play shard holds (a smaller shard loses less to an interruption)")
    a.add_argument("--sampled", type=int, default=20, help="decisions sampled at the temperature before the most visited is played")
    a.add_argument("--temperature", type=float, default=1.0)
    a.add_argument("--window", type=int, default=8, help="generations of self-play data trained on")
    a.add_argument("--older-weight", type=float, default=0.5)
    a.add_argument("--imitation-weight", type=float, default=0.2, help="gen0's data (the tactical bot's games)")
    a.add_argument("--train-hours", type=float, default=0.65, help="4,067 steps at --train-rate: generation 1 overfit its value at 1.5")
    a.add_argument("--train-rate", type=float, default=445.0, help="samples a second training runs at (sets the steps)")
    a.add_argument("--train-steps", type=int, help="instead of --train-hours")
    a.add_argument("--lr", type=float, default=2e-4)
    a.add_argument("--warmup", type=int, default=200)
    a.add_argument("--value-mix", type=float, default=0.5, help="the root value's share of the value target on searched positions")
    a.add_argument("--train-args", default="", help="more arguments for train.py, one string")
    a.add_argument("--gate-games", type=int, default=6000, help="the most games the gate's SPRT may take")
    a.add_argument("--gate-h1", type=float, default=0.52,
                   help="the SPRT's H1, the candidate's share: Magic's variance hides a 55%% step, so a smaller one "
                        "(about 3,700 games to a verdict at 50%% or 52%%, up to ~5,400 between)")
    a.add_argument("--gate-parallel", type=int, help="gate games at once (default: one a CPU thread; with the GPU server, "
                                                      f"{SERVER_GATE} a CPU thread)")
    a.add_argument("--accept", choices=["sprt", "ahead"], default="sprt")
    a.add_argument("--keep", type=int, default=8)
    a.add_argument("--no-build", action="store_true")
    a.add_argument("--retrain", metavar="REASON", help="run a rejected generation again from its training, saying why")
    a.add_argument("--dry-run", action="store_true", help="print the plan and run nothing")
    a.add_argument("--tiny", action="store_true", help="a toy scale, for testing the driver")
    args = a.parse_args(argv)
    show_progress = args.progress == "on" or (args.progress == "auto" and sys.stderr.isatty())
    server = wants_server(args.gpu_server, args.device)
    cpus = os.cpu_count() or 4
    if args.lanes is None:
        args.lanes = max(1, round(cpus * (SERVER_LANES if server else 1)))
    if args.gate_parallel is None:
        args.gate_parallel = max(1, round(cpus * (SERVER_GATE if server else 1)))
    if args.tiny:
        args.games, args.simulations, args.train_steps, args.gate_games, args.warmup = 4, 8, 4, 4, 1
        args.accept = "ahead" if args.accept == "sprt" else args.accept

    root = Path(args.root).resolve()
    decks_dir, pool = None, []
    if args.decks_dir.strip():
        decks_dir = Path(args.decks_dir)
        if not decks_dir.is_absolute():
            decks_dir = REPO / decks_dir
        pool = deck_pool(decks_dir)
    gen = args.generation or next_generation(root)
    gen_dir = root / f"gen{gen}"
    rec = Record(gen_dir, gen, args.dry_run)
    log = print
    champ = champion(root) if args.dry_run else bootstrap(root)
    if champ is None:
        champ = root / "champions" / "gen0"
    rec.data.setdefault("champion", champ.name)
    rec.data["mode"] = args.mode
    champ = root / "champions" / rec.data["champion"]
    log(f"generation {gen} in {gen_dir}; champion {champ.name}; {args.mode} mode")
    model_dir = root / "models" / f"gen{gen}"
    seed = gen * 10_000_000
    if args.retrain:
        retrain(rec, gen_dir, model_dir, args, log)

    def stage(name, fn):
        if rec.done(name) and name != "build":
            log(f"[{name}] done already ({rec.data['stages'][name]['seconds']:.0f} s)")
            return
        log(f"[{name}]")
        t = time.time()
        numbers = fn()
        if not args.dry_run:
            rec.put(name, time.time() - t, **(numbers or {}))

    def server_for(models: list[Path], stage_name: str):
        """The inference server for a stage, or nothing (a stand-in whose address is unused) when it is off."""
        if not server:
            return contextlib.nullcontext(None)
        for m in models:
            # A model exported before export wrote the server's weights: write them from its checkpoint.
            if (m / "manifest.json").exists() and not (m / "model.pt").exists() and (m / "train" / "checkpoint.pt").exists():
                run([sys.executable, "-m", "cantrip_ai.export", "--checkpoint", m / "train" / "checkpoint.pt", "--out", m,
                     "--weights-only"], gen_dir / "logs" / f"weights-{m.name}.log", args.dry_run,
                    env=dict(PYTHONPATH=str(REPO / "ai")))
        return InferenceServer(models, gen_dir / "logs" / f"server-{stage_name}.log", args.server_device or args.device,
                               args.server_args.split(), args.dry_run, log, native=args.server_impl == "native",
                               ort_dylib=args.ort_dylib)

    def build():
        run(["cargo", "build", "--release", "-p", "cantrip-ai-host", "--features", "ort"], gen_dir / "logs/build-release.log",
            args.dry_run, stream=show_progress)
        if server and args.server_impl == "native":
            # On its own: the gpu feature loads onnxruntime at run time, which the other binaries are not built to.
            run(["cargo", "build", "--release", "-p", "cantrip-ai-host", "--features", "gpu", "--bin", "cantrip-infer"],
                gen_dir / "logs/build-infer.log", args.dry_run, stream=show_progress)
        run(["cargo", "build", "--profile", "cards", "-p", "cantrip-host", "--bin", "cantrip-native-host"],
            gen_dir / "logs/build-native.log", args.dry_run, stream=show_progress)
        run([*dotnet(), "build", "dotnet/tools/Cantrip.BotLab/Cantrip.BotLab.csproj", "-c", "Release", "-v", "q", "--nologo"],
            gen_dir / "logs/build-botlab.log", args.dry_run, env=dict(DOTNET_ROLL_FORWARD="Major"), stream=show_progress)

    def selfplay():
        out = gen_dir / "selfplay"
        resume = out.exists() and any(p.suffix == ".zst" or p.suffix == ".part" for p in out.iterdir())
        if out.exists() and not args.dry_run and not resume:
            shutil.rmtree(out)
        cmd = [SELFPLAY, "--model", champ, "--out", out, "--games", args.games, "--simulations", args.simulations,
               "--lanes", args.lanes, "--seed", seed, "--sampled", args.sampled, "--temperature", args.temperature,
               "--mode", args.mode, "--shard-games", args.selfplay_shard_games, "--batch", args.search_batch]
        if args.full_simulations:
            cmd += ["--full-simulations", args.full_simulations, "--full-share", args.full_share]
        if resume:
            cmd.append("--resume")
        if decks_dir is not None:
            # A pairing a game, in a file: as arguments they pass Windows' 32K command line.
            listed = gen_dir / "selfplay-pairings.txt"
            if not args.dry_run:
                gen_dir.mkdir(parents=True, exist_ok=True)
                listed.write_text("".join(p + "\n" for p in metagame_pairings(pool, args.games, seed)), encoding="utf-8")
            cmd += ["--decks", decks_dir, "--prior", decks_dir, "--pairings", listed]
        with server_for([champ], "selfplay") as gpu:
            if gpu is not None:
                cmd += ["--backend", "remote", "--server", gpu.address]
            run(cmd, gen_dir / "logs/selfplay.log", args.dry_run, env=dict(CANTRIP_PROGRESS="1") if show_progress else None,
                stream=show_progress)
        if args.dry_run:
            return
        r = json.loads((out / "selfplay-report.json").read_text(encoding="utf-8"))
        numbers = dict(games=r["Games"], resumed=r.get("Resumed", 0), games_this_run=r.get("GamesThisRun", r["Games"]),
                       samples=r["Samples"], visit_samples=r["VisitSamples"], capped=r["Capped"], dropped=r["Dropped"],
                       fallbacks=r["NetworkFallbacks"], simulations=r["Simulations"], lanes=r["Lanes"], games_per_hour=r["GamesPerHour"],
                       searched_per_second=r["SearchedPerSecond"], evaluations_per_second=r["EvaluationsPerSecond"],
                       first_player=r["FirstPlayerWinRate"], mean_turns=r["MeanTurns"], seeds=r["Seeds"], producer=r["Producer"],
                       wins=r["Wins"], errors=r["Errors"], failures=r["Failures"])
        if decks_dir is not None:
            numbers["decks"] = f"{len(pool)} decks from {decks_dir.name}, metagame-weighted pairings"
        return numbers

    def train():
        w = window(root, gen, args.window, args.older_weight, args.imitation_weight)
        for d, weight in w:
            log(f"  window: {d} x{weight}")
        out = model_dir / "train"
        resuming = (out / "checkpoint.pt").exists()
        if out.exists() and not args.dry_run and not resuming:
            shutil.rmtree(out)
        steps = args.train_steps or max(1, int(args.train_hours * 3600 * args.train_rate / 256))
        init = champ / "train" / "checkpoint.pt"
        cmd = [sys.executable, "-m", "cantrip_ai.train", "--data", *[f"{d}:{weight:g}" for d, weight in w], "--out", out,
               "--steps", steps, "--lr", args.lr, "--warmup", args.warmup, "--value-mix", args.value_mix, "--seed", gen,
               "--device", args.device, "--progress", "on" if show_progress else "off"]
        if args.train_hours and not args.train_steps:
            cmd += ["--hours", round(args.train_hours * 1.25, 3)]
        if resuming:
            cmd.append("--resume")
        elif init.exists() or args.dry_run:
            cmd += ["--init", init]
        cmd += args.train_args.split()
        run(cmd, gen_dir / "logs/train.log", args.dry_run, env=dict(PYTHONPATH=str(REPO / "ai")), stream=show_progress)
        if args.dry_run:
            return
        m = json.loads((out / "metrics.json").read_text(encoding="utf-8"))["history"][-1]
        import torch  # the checkpoint's data record: what each window directory gave

        ck = torch.load(out / "checkpoint.pt", map_location="cpu", weights_only=False)
        return dict(steps=m["step"], seen=m["seen"], held_out=m.get("held_out"),
                    init="resumed" if resuming else (init.parent.parent.name if init.exists() else "fresh"),
                    lr=args.lr, value_mix=args.value_mix, window=ck["data"]["directories"], samples=ck["data"]["samples"])

    def export():
        run([sys.executable, "-m", "cantrip_ai.export", "--checkpoint", model_dir / "train" / "checkpoint.pt", "--out", model_dir],
            gen_dir / "logs/export.log", args.dry_run, env=dict(PYTHONPATH=str(REPO / "ai")), stream=show_progress)
        return dict(model=str(model_dir))

    def parity():
        # With the GPU server, the server checks the references itself as it starts, and the Rust side runs them
        # through it too (`RemoteEvaluator`), beside tract and ort.
        with server_for([model_dir], "parity") as gpu:
            env = dict(CANTRIP_MODEL=str(model_dir))
            if gpu is not None:
                env["CANTRIP_AI_SERVER"] = gpu.address
            text = run(["cargo", "test", "--profile", "cards", "-p", "cantrip-ai", "--features", "ort", "--test", "eval",
                        "model_parity", "--", "--ignored", "--nocapture"], gen_dir / "logs/parity.log", args.dry_run,
                       env=env, stream=show_progress)
        said = [l for l in text.splitlines() if "parity" in l.lower() or "max" in l.lower()]
        return dict(summary=(said[-1].strip() if said else "passed").replace("|", "/"))

    def gate():
        name = f"neural:{args.mode}@{args.simulations}" if args.mode != "full" else f"neural@{args.simulations}"
        if decks_dir is not None:
            decks = f"{len(pool)} standard decks ({decks_dir.name}), each a mirror"
        else:
            decks = "rw-aggro and ub-reanimator with mirrors"
        gate_log = gen_dir / "logs/gate.log"
        if not args.dry_run and gate_log.exists():
            # A gate that finished but that the driver never recorded (it stopped between BotLab exiting and the
            # stage being written) is final: read it back instead of rolling a fresh one. The per-network summary
            # names the champion it played, so a log from a different champion (or budget) is not reused.
            text = gate_log.read_text(encoding="utf-8", errors="replace")
            if f"/{champ.name}:" in text and f"neural@{args.simulations} against" in text:
                try:
                    recovered = parse_gate(text)
                except ValueError:
                    pass
                else:
                    log(f"[gate] reusing the finished gate in {gate_log} "
                        f"({recovered['wins']} to {recovered['losses']}, {recovered['verdict']})")
                    return dict(recovered, champion=champ.name, simulations=args.simulations, decks=decks)
        cmd = [*dotnet(), BOTLAB, "duel", "--a", name, "--b", name, "--model", model_dir, "--model-b", champ]
        if decks_dir is not None:
            # BotLab's pool of mirrors: both seats the same standard list, each pair of games the next list.
            cmd += ["--decks", decks_dir, "--ai-prior", decks_dir]
        else:
            cmd += ["--deck", "rw-aggro", "--deck", "ub-reanimator", "--mirrors"]
        cmd += ["--games", args.gate_games, "--parallel", args.gate_parallel, "--seed", seed + 5_000_000]
        if args.accept == "sprt":
            cmd += ["--sprt", "--sprt-h1", args.gate_h1]
        env = {"DOTNET_ROLL_FORWARD": "Major", "CANTRIP_NATIVE_HOST": str(NATIVE), "CANTRIP_AI_HOST": str(AI_HOST)}
        with server_for([model_dir, champ], "gate") as gpu:
            if gpu is not None:
                cmd += ["--ai-server", gpu.address]
            text = run(cmd, gen_dir / "logs/gate.log", args.dry_run, env=env, stream=show_progress)
        if args.dry_run:
            return
        return dict(parse_gate(text), champion=champ.name, simulations=args.simulations, decks=decks, h1=args.gate_h1)

    def promote():
        if args.dry_run:
            log(f"  accepted: copy {model_dir} to {root / 'champions' / f'gen{gen}'} and make it current; rejected: record it")
            return
        g = rec.data["stages"]["gate"]
        ok = accepted(g, args.accept)
        entry = dict(generation=gen, accepted=ok, verdict=g["verdict"], share=g["share"], margin=g["margin"], llr=g["llr"],
                     games=g["games"], against=champ.name, simulations=args.simulations, mode=args.mode, at=now())
        if ok:
            dest = root / "champions" / f"gen{gen}"
            if dest.exists():
                shutil.rmtree(dest)
            shutil.copytree(model_dir, dest)
            (dest / "CHAMPION.json").write_text(json.dumps(entry, indent=1))
            set_current(root, dest.name)
        history(root, entry)
        log(f"  {'accepted: gen' + str(gen) + ' is the champion' if ok else 'rejected: ' + champ.name + ' stays'}")
        return dict(accepted=ok, champion=f"gen{gen}" if ok else champ.name, rule=args.accept)

    def retire():
        doomed = retention.retire(root, args.keep, dry_run=args.dry_run, log=log)
        return dict(deleted=len(doomed), keep=args.keep)

    for name, fn in zip(STAGES, (build, selfplay, train, export, parity, gate, promote, retire)):
        if name == "build" and args.no_build:
            continue
        stage(name, fn)
    if not args.dry_run:
        rec.data["finished"] = now()
        rec.save()
        log(readme(rec.data))


if __name__ == "__main__":
    main()
