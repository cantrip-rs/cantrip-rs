"""One generation of expert iteration (docs/ai-player-plan.md, "Phase 4"): self-play, train, export, parity, gate,
promote, retire.

    python -m cantrip_ai.generation --root ../../ai-data [--generation N] [--games 5000] [--simulations 64]
        [--decks-dir decks/standard] [--device auto] [--train-hours 0.65] [--gate-games 6000] [--gate-h1 0.52] [--dry-run] [--tiny]

Self-play draws its pairings from `--decks-dir` by metagame share (the player count in each decklist's header), and
so does the gate (`--gate-pairings metagame`; `mirrors` plays the decks as a pool of mirrors, as before 2026-11-13).
`--decks-dir ''` keeps the fixed rw-aggro/ub-reanimator slice.

The gate's games come in pairs, each pair one deal with the players exchanged (a true duplicate), and BotLab scores
them by pair as well as by game: the pentanomial (the candidate's 0 / 0.5 / 1 / 1.5 / 2 points in a pair), the paired
share and its interval, and with `--accept sprt-paired` the pentanomial SPRT. `--accept above-half`, the default (the
project's rule, "anything over 50% is a win"), reads the paired share (exactly, from the five pair counts), and
`--gate-early-stop` ends it once the paired interval excludes a half (at `--gate-stop-z` 2.58, after
`--gate-stop-min-pairs` 100 pairs of which `--gate-stop-min-decisive` 30 did not split); `--accept always` promotes
with no gate (`--gate-measure` plays it anyway, as a measurement).

Measurements beside the gate, each optional and each after the promotion, so none can hold up or stop a generation (a
measurement that fails is recorded as failed, in `generation.json` and the README, and the generation goes on):
`--net-duel-games N` (the networks alone, no search, by pair; stage `netduel`); the permanent held-out set
(`--heldout-games N` chooses it once, from a generation's self-play before its training, moved to `ROOT/heldout`, which
no window holds; stage `evaluate` then scores the candidate and the champion on it every generation: value MSE and
ECE, policy CE and top-1 against its full searches' targets); and the anchor ladder (`--ladder-every N`: the champion
after promotion against `--ladder-anchors`, gen0 and the tactical bot, at the budget and every
`--ladder-play-every`-th time at the play budget, in `ROOT/ladder.json` and `ladder.md`, a generation's rows replaced
when it runs again). The fixed
evaluation set train.py scores (`--eval-data`, the tactical bot's games in `pi/eval`) measures the distance from the
tactical bot: its policy CE rising is expected as the policy improves, and the README says so where it shows it.

The champion (`ROOT/champions/current`, a frozen model directory with its `train/checkpoint.pt`) plays itself in
`cantrip-selfplay`, both seats searching at the budget, and the games go to `ROOT/genN/selfplay` with the search's root
visits as the policy targets. The network is fine-tuned from the champion's checkpoint on the window of generations
(the newest weighted 1, older self-play `--older-weight`, the imitation data gen0 `--imitation-weight`), exported to
`ROOT/models/genN`, checked for parity against PyTorch, and gated: the candidate against the champion in BotLab, both
searching at the same budget, by `--accept` (default above-half, the paired share above 50%; `sprt` is the unpaired
SPRT, H0 50%, H1 `--gate-h1` 52%, alpha = beta = 0.05), each pair of games one deal with the seats swapped. Accepted,
the candidate is copied to `ROOT/champions/genN` and becomes `current`; rejected, it is
recorded and the champion stays. Either way `champions/history.json` gets a line, and the generation's data stays in
the window (a rejected generation's games are still the champion's search). Then retention (`retention.py`).

Each stage's time and numbers go to `ROOT/genN/generation.json` as it finishes, and `ROOT/genN/README.md` is rewritten
from it; a stage already done there is not run again, so an interrupted generation resumes where it stopped (the
build runs every time, so a resumed run never uses a stale binary). A gate that finished but was not recorded (the
driver stopped between BotLab exiting and the stage being written) is read back from `genN/logs/gate.log` and
promoted, so the long gate is not rolled again; only a log of the same champion, seat, acceptance rule (its stop line)
and pairing mode is read back. Within a stage, self-play keeps its closed shards and plays only the
games they do not hold, and training resumes from its last checkpoint. Every measurement is in the information mode
`--mode` (Full).

Self-play governs itself (docs/ai-host.md, "The governor"): `--lanes` is a ceiling (96 by default), the lanes scale to
the first limiter of throughput, CPU or memory (`--selfplay-autoscale`, `--selfplay-scale-*`, `--selfplay-cpu-ceiling`),
and a game starts only while memory has room for it (`--selfplay-memory-budget`, `--selfplay-memory-reserve`,
`--selfplay-memory-commit-reserve`). Memory under `--selfplay-memory-floor`, or Ctrl-C, stops it gracefully with every
finished game in a closed shard (exit code 3): the driver then says so and exits, the stage not done, and running it
again resumes.

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

Training's length and mix (docs/ai-training.md, "Training options"): `--target-uses K` sets the steps so that each
search target is used about K times while it is in the window (instead of `--train-hours` at `--train-rate`, a rate
measured on the CPU): a target is trained on in its own generation and, at `--older-weight`'s share, in each of the
`--window` - 1 after it, so this generation's run uses each about K / (1 + (window - 1) * older-weight) times
(`train.py --target-uses`). `--search-target-repeat`, `--imitation-policy-weight`, `--value-mix-full` and
`--value-mix-fast` are train.py's. `--schedule constant --carry-optimizer` is continuous training: the rate held after
the warm-up and the champion's optimizer state (and its unaveraged weights, if it kept an average) carried on.
`--train-args` passes anything else to train.py; `{champion}` in it is the champion's checkpoint
(`--train-args '--anchor {champion} --anchor-kl 0.05'`).

Distributed self-play (docs/ai-training.md, "Distributed self-play"): `--workers FILE` plays the self-play stage on this
machine and on the workers the file names, over SSH (`distributed.py`): each game index played exactly once across
them, handed out on demand, the shards pulled back into `genN/selfplay` as they close, checked, and recorded with the
host that played them (`genN/selfplay/HOSTS.json`). Off by default, and with it off nothing changes.
`generation.py --root ROOT --stop` (`--stop-now`: the games in flight too) writes `selfplay.stop` in the current
generation's self-play directory: self-play stops gracefully as at Ctrl-C (exit 3), and a distributed driver stops its
workers the same way. The driver removes the file when it runs the stage again.

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

from . import distributed, mining, retention

REPO = Path(__file__).resolve().parents[2]
# heldout, netduel, evaluate and ladder run only when asked for (or, evaluate, when a held-out set exists). The
# measurements come after the promotion, and a failure of one is recorded and passed over (`MEASUREMENTS`).
STAGES = ("build", "selfplay", "heldout", "train", "export", "parity", "gate", "promote", "mine", "netduel", "evaluate",
          "ladder", "mined", "retire")
# The stages a retrain runs again (all but the build, the self-play and the held-out set's choice).
LATER = ("train", "export", "parity", "gate", "promote", "mine", "netduel", "evaluate", "ladder", "mined", "retire")
# The stages that decide nothing about this generation: a failure is recorded and the generation goes on. The pair
# miner's two (`mining.py`) make the next generation's targets; without them it trains without this one's.
MEASUREMENTS = ("mine", "netduel", "evaluate", "ladder", "mined")
# BotLab's `Decks`: a file of a decks folder with fewer cards than this is no decklist (a pool's run.txt and the like).
MIN_DECK = 30
EXE = ".exe" if os.name == "nt" else ""
BOTLAB = REPO / "dotnet/tools/Cantrip.BotLab/bin/Release/net8.0/cantrip-botlab.dll"
NATIVE = REPO / f"target/cards/cantrip-native-host{EXE}"
AI_HOST = REPO / f"target/release/cantrip-ai-host{EXE}"
SELFPLAY = REPO / f"target/release/cantrip-selfplay{EXE}"
INFER = REPO / f"target/release/cantrip-infer{EXE}"
EXPLORE = REPO / f"target/release/cantrip-explore{EXE}"
# The native host that records every position of a table's games (`--features ai-record`), built apart: the pair
# miner's replays (`mining.py`). The recorder changes nothing a game does.
RECORD_NATIVE = REPO / f"target/record/cards/cantrip-native-host{EXE}"
# With the GPU server, lanes and gate games a CPU thread: they wait on the server, so more than the cores keep the GPU
# fed (measured on 32 threads and an RTX 4080; docs/ai-training.md).
SERVER_LANES = 2
SERVER_GATE = 1
# Self-play's lane ceiling when it scales itself (`--selfplay-autoscale`, on by default): it grows to the first limiter
# of throughput, CPU or memory below this (docs/ai-host.md, "The governor").
AUTOSCALE_LANES = 96
# cantrip-selfplay's exit code when its governor stopped it (memory under its floor, or Ctrl-C): every finished game is
# in a closed shard, and running the stage again resumes (`cantrip_ai_host::governor::EXIT_STOPPED`).
SELFPLAY_STOPPED = 3


class Stopped(Exception):
    """A stage's process stopped gracefully, its work kept for a rerun to resume (`run`'s `stopped_code`)."""

    def __init__(self, code: int, text: str):
        super().__init__(f"stopped ({code})")
        self.code, self.text = code, text


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
SHARE = re.compile(r"(\d+(?:\.\d+)?)%\s+of\s+the\s+metagame", re.IGNORECASE)
COUNTED = re.compile(r"^(\d+)\s+(.+)$")


def deck_cards(text: str) -> int:
    """A decklist's main deck's cards, as BotLab's `DeckList.Parse` counts them (`DeckList.Count`): every line but the
    blank ones, `#` comments and the section headers, `N Name` N cards and a name alone one, up to a `Sideboard` line,
    leaving out a Commander section and the companion."""
    total, section, companion_next = 0, "main", False
    for raw in text.splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        low = line.lower()
        if low == "sideboard":
            section = "side"
            continue
        if low in ("commander", "commanders"):
            section = "commanders"
            continue
        if low in ("deck", "main"):
            section = "main"
            continue
        if low == "companion":
            companion_next = True
            continue
        if companion_next:
            companion_next = False
            continue
        if section == "main":
            m = COUNTED.match(line)
            total += int(m.group(1)) if m else 1
    return total


def deck_pool(directory: Path) -> list[tuple[str, float]]:
    """A folder of decklists, with the metagame weight in each header: its share (`9.7% of the metagame`), else its
    `(N players, ...)`; 1 when it has neither. A file
    of fewer than `MIN_DECK` cards is left out, as BotLab leaves it out of `--decks` (so a pairing never names a deck
    the duel does not have)."""
    pool = []
    for path in sorted(directory.glob("*.txt")):
        text = path.read_text(errors="replace")
        if deck_cards(text) < MIN_DECK:
            continue
        head = text.splitlines()[:1]
        share = SHARE.search(head[0]) if head else None
        found = PLAYERS.search(head[0]) if head else None
        pool.append((path.stem, float(share.group(1)) if share else int(found.group(1)) if found else 1))
    if not pool:
        raise SystemExit(f"no decklists in {directory}")
    return pool


def metagame_pairings(pool: list[tuple[str, float]], games: int, seed: int) -> list[str]:
    """`games` matchups drawn by metagame share (the players in each decklist's header), at most 4096 distinct slots.

    The self-play schedule cycles this list and swaps the two decks every other pass, so both seats meet each list.
    """
    names = [name for name, _ in pool]
    weights = [weight for _, weight in pool]
    rng = random.Random(seed)
    return [f"{rng.choices(names, weights)[0]}:{rng.choices(names, weights)[0]}"
            for _ in range(min(games, 4096))]


def _within(path: Path, others) -> bool:
    """Whether `path` is one of `others` or inside one."""
    p = Path(path).resolve()
    return any(p == (o := Path(other).resolve()) or o in p.parents for other in others)


def window(root: Path, gen: int, size: int, older: float, imitation: float, exclude=()) -> list[tuple[Path, float]]:
    """The training window: the self-play data of the last `size` generations up to `gen`, newest at weight 1, and gen0.
    A directory that is, or is inside, one of `exclude` (the permanent held-out set) is left out."""
    out: list[tuple[Path, float]] = []
    for n in range(gen, max(gen - size, 0), -1):
        d = root / f"gen{n}" / "selfplay"
        if n == gen or (d.exists() and any(d.glob("*.shard.zst"))):  # the generation's own: its self-play stage made it
            out.append((d, 1.0 if n == gen else older))
    if imitation > 0:
        out += [(d, imitation) for d in imitation_dirs(root)]
    return [(d, w) for d, w in out if not _within(d, exclude)]


# --- The permanent held-out set -----------------------------------------------------------------------------------

HELDOUT_RECORD = "HELDOUT.json"


def heldout_dir(root: Path, given: str | None) -> Path:
    """The permanent held-out set's directory: `--heldout-dir`, else `ROOT/heldout`."""
    return Path(given).resolve() if given else root / "heldout"


def heldout_ready(directory: Path) -> bool:
    return directory.is_dir() and any(directory.glob("*.shard.zst"))


def _shard_games(path: Path) -> int:
    sys.path.insert(0, str(REPO / "ai"))
    import shards  # noqa: PLC0415 (ai/shards.py, as dataset reads it)

    _, games = shards.scan(path)
    try:
        return sum(1 for _ in games)
    finally:
        games.close()


def designate_heldout(selfplay_dir: Path, dest: Path, games: int, gen: int, dry_run: bool, log=print, count=_shard_games) -> dict:
    """Chooses the permanent held-out set, once: whole shards of `selfplay_dir` (a generation's self-play, before any
    network has trained on it), moved into `dest` until they hold at least `games` games, with `HELDOUT.json` saying
    what they are. Moved, not copied, so no training window can hold them; the full shards first (each lane's first),
    spread over the lanes in turn rather than all of one lane's games."""
    if (dest / HELDOUT_RECORD).exists() or heldout_ready(dest):
        raise SystemExit(f"the held-out set {dest} is chosen already: it is chosen once and never changed")
    if dry_run:
        log(f"  would move whole shards of {selfplay_dir} holding at least {games} games to {dest}, the held-out set for good")
        return {}
    found = sorted(selfplay_dir.glob("*.shard.zst"))
    # Each lane's shards end -00000, -00001, ...: taking every lane's first shard before any lane's second spreads the
    # set over the lanes (and the pairings and seeds they were dealt).
    found.sort(key=lambda p: (p.stem.rsplit("-", 1)[-1].split(".")[0], p.name))
    chosen, total = [], 0
    for path in found:
        if total >= games:
            break
        n = count(path)
        chosen.append((path, n))
        total += n
    if total < games:
        raise SystemExit(f"--heldout-games {games}: {selfplay_dir} holds only {total} games")
    log(f"  held out for good: {len(chosen)} shards, {total} games of gen{gen}'s self-play, moved to {dest}")
    record = dict(source=str(selfplay_dir), generation=gen, games=total, shards=[p.name for p, _ in chosen], chosen=now(),
                  note="Chosen once and never trained on: the driver moved these shards out of the generation's self-play "
                       "before its training, and every window leaves this directory out.")
    dest.mkdir(parents=True, exist_ok=True)
    for path, _ in chosen:
        shutil.move(str(path), dest / path.name)
    (dest / HELDOUT_RECORD).write_text(json.dumps(record, indent=1), encoding="utf-8")
    return dict(directory=str(dest), games=total, shards=len(chosen), source=f"gen{gen}")


# --- The anchor ladder --------------------------------------------------------------------------------------------

LADDER = "ladder.json"


def ladder_rows(root: Path) -> list[dict]:
    path = root / LADDER
    return json.loads(path.read_text(encoding="utf-8")) if path.exists() else []


def ladder_table(rows: list[dict]) -> str:
    """`ROOT/ladder.md`: every ladder duel, the champion of its generation against a fixed anchor."""
    lines = ["# The anchor ladder", "", "Each row: the champion after generation N against a fixed anchor, by pair "
             "(written by `ai/cantrip_ai/generation.py` from `ladder.json`).", "",
             "| generation | champion | anchor | budget | games | paired share | pairs 0 / 0.5 / 1 / 1.5 / 2 | unpaired |",
             "|---|---|---|---|---|---|---|---|"]
    for r in rows:
        paired = "-" if r.get("paired_share") is None else f"{r['paired_share']:.1%} +/- {r['paired_margin']:.1%}"
        counts = " / ".join(str(c) for c in r["pentanomial"]) if r.get("pentanomial") else "-"
        lines.append(f"| {r['generation']} | {r['champion']} | {r['anchor']} | {r['budget']} | {r['games']} | {paired} | "
                     f"{counts} | {r['share']:.1%} +/- {r['margin']:.1%} |")
    return "\n".join(lines) + "\n"


def ladder_record(root: Path, rows: list[dict], gen: int | None = None):
    """The ladder's rows with `rows` added; generation `gen`'s earlier rows (a retrained generation's, its ladder run
    again) replaced by them."""
    kept = [r for r in ladder_rows(root) if gen is None or r.get("generation") != gen]
    every = sorted(kept + rows, key=lambda r: r.get("generation", 0))
    (root / LADDER).write_text(json.dumps(every, indent=1), encoding="utf-8")
    (root / "ladder.md").write_text(ladder_table(every), encoding="utf-8")


def ladder_due(gen: int, every: int, play_every: int) -> tuple[bool, bool]:
    """Whether generation `gen` runs the ladder (every `every` generations), and whether also at the play budget (every
    `play_every`-th of those)."""
    due = every > 0 and gen % every == 0
    return due, due and play_every > 0 and (gen // every) % play_every == 0


def seat_name(mode: str, budget) -> str:
    """BotLab's name for a neural seat in `mode` at `budget` (a simulation count, `250ms`, or nothing for the network
    alone): `neural@64`, `neural:decklist@250ms`, `neural:decklist`."""
    base = "neural" if mode == "full" else f"neural:{mode}"
    return f"{base}@{budget}" if budget not in (None, "", 0, "0") else base


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
# The duel by pair (BotLab's Tally.WritePaired): the pentanomial, the paired share, the paired SPRT, the interval stop.
PAIRS = re.compile(r"^Pairs by \S+ points in a pair \(0 / 0\.5 / 1 / 1\.5 / 2\): (\d+) / (\d+) / (\d+) / (\d+) / (\d+); "
                   r"(\d+) pairs, (\d+) incomplete", re.M)
PAIRED = re.compile(r"^Paired: \S+ scored ([\d.]+) % \+/- ([\d.]+) % by pair", re.M)
PAIRED_SPRT = re.compile(r"^Paired SPRT \(.*?\): LLR (-?[\d.]+) after (\d+) pairs; (.*)$", re.M)
INTERVAL = re.compile(r"^Interval stop \(.*?\): (.*)$", re.M)
RULES = ("sprt", "sprt-paired", "ahead", "above-half", "always")


def _verdict(tail: str) -> str:
    return "H1" if tail.startswith("H1") else "H0" if tail.startswith("H0") else "none"


def parse_gate(text: str) -> dict:
    """BotLab's duel summary: the candidate (A) against the champion (B), and the SPRT's verdict; and by pair, where the
    duel printed it (BotLab since 2026-11-13): the five counts of the pentanomial (`pentanomial`, A's 0 / 0.5 / 1 / 1.5 / 2
    points in a pair), the paired share and its 95% margin, the paired SPRT's LLR and verdict, and the interval stop.
    An older log has no paired lines: those fields are None."""
    m = RESULT.search(text)
    if not m:
        raise ValueError("no result line in the gate's output")
    wins, losses, draws = int(m.group(3)), int(m.group(4)), int(m.group(5) or 0)
    out = dict(wins=wins, losses=losses, draws=draws, games=wins + losses + draws, share=float(m.group(6)) / 100,
               margin=float(m.group(7)) / 100)
    s = SPRT.search(text)
    if s:
        llr, tail = float(s.group(1)), s.group(3)
        out.update(llr=llr, sprt=tail, verdict=_verdict(tail))
        at = re.search(r"at game (\d+)", tail)
        out["stopped_at"] = int(at.group(1)) if at else None
    else:
        out.update(llr=None, sprt=None, verdict="none", stopped_at=None)
    out.update(pentanomial=None, pairs=None, incomplete=None, split=None, paired_share=None, paired_margin=None,
               paired_llr=None, paired_sprt=None, paired_verdict="none", paired_stopped_at=None,
               interval_stop=None, interval_stopped_at=None, interval_way=None)
    if p := PAIRS.search(text):
        counts = [int(p.group(k)) for k in range(1, 6)]
        out.update(pentanomial=counts, pairs=int(p.group(6)), incomplete=int(p.group(7)),
                   split=counts[2] / sum(counts) if sum(counts) else None)
    if p := PAIRED.search(text):
        out.update(paired_share=float(p.group(1)) / 100, paired_margin=float(p.group(2)) / 100)
    if p := PAIRED_SPRT.search(text):
        tail = p.group(3)
        at = re.search(r"at pair (\d+)", tail)
        out.update(paired_llr=float(p.group(1)), paired_sprt=tail, paired_verdict=_verdict(tail),
                   paired_stopped_at=int(at.group(1)) if at else None)
    if p := INTERVAL.search(text):
        tail = p.group(1)
        at = re.search(r"stopped at pair (\d+), \S+ (ahead|behind)", tail)
        out.update(interval_stop=tail, interval_stopped_at=int(at.group(1)) if at else None,
                   interval_way=at.group(2) if at else None)
    return out


def accepted(gate: dict, rule: str) -> bool:
    """Whether the gate promotes the candidate, by `--accept`:

    `sprt`: the unpaired SPRT accepted H1. `sprt-paired`: the paired (pentanomial) SPRT accepted H1. `ahead`: the
    unpaired share above a half by more than its margin (no SPRT verdict needed). `above-half`: the candidate's share
    above a half after every gate game (a draw a half): any step forward is kept, so the champion's self-play moves on
    each generation a candidate is ahead ("anything over 50% is a win", 2026-11-13; at H1 52% gen2's 51.1%
    over 6,000 was rejected and three generations self-played gen1). It reads the paired share where the gate printed
    one (the same games' mean, by pair), the unpaired share in an older log. With `--gate-early-stop` the gate ends once
    the paired interval excludes a half, and the share then decides the same way. `always`: promoted, whatever the gate
    says or whether it was played (AlphaZero's continuous training)."""
    if rule == "always":
        return True
    if rule == "sprt":
        return gate.get("verdict") == "H1"
    if rule == "sprt-paired":
        return gate.get("paired_verdict") == "H1"
    if rule == "above-half":
        return above_half(gate)
    return gate["share"] - gate["margin"] > 0.5


def above_half(gate: dict) -> bool:
    """Whether the candidate scored more than a half, from the counts rather than a printed share (which is rounded to a
    tenth of a percent): by pair, its points over the pairs' (0 / 0.5 / 1 / 1.5 / 2 points, `pentanomial`) above one a
    pair, `sum(k c_k) > 2 sum(c_k)` in half points; in an older log with no pairs, its wins above its losses (a draw a
    half to each). A gate with neither is read by its share."""
    counts = gate.get("pentanomial")
    if counts and sum(counts):
        return sum(k * c for k, c in enumerate(counts)) > 2 * sum(counts)
    if gate.get("wins") is not None and gate.get("losses") is not None:
        return gate["wins"] > gate["losses"]
    share = gate.get("paired_share")
    return (share if share is not None else gate["share"]) > 0.5


def _csharp_number(x: float) -> str:
    """`x` as C#'s `ToString("0.###")` writes it: at most three decimals, no trailing zeros."""
    return f"{x:.3f}".rstrip("0").rstrip(".")


def gate_stop_lines(accept: str, early_stop: bool, z: float, min_pairs: int, min_decisive: int) -> dict:
    """The stop lines a gate's log holds under a rule, by the line's kind: the start the line must have, or None where
    the log has none of that kind. BotLab prints a line for each stop it ran (`Tally.SprtLine`, `Tally.WritePaired`)."""
    return {
        "SPRT (": "SPRT (" if accept == "sprt" else None,
        "Paired SPRT (": "Paired SPRT (" if accept == "sprt-paired" else None,
        "Interval stop (": (f"Interval stop (the paired {_csharp_number(z)}-sigma interval excludes 50 %, from {min_pairs} pairs, "
                            f"{min_decisive} of them not split): ") if accept == "above-half" and early_stop else None,
    }


def gate_reusable(text: str, champion: str, seat: str, lines: dict, dealt: str) -> bool:
    """Whether a gate log the driver never recorded is the gate this run would play: the same champion (the
    per-network summary names it), the same seats, the same acceptance rule (`gate_stop_lines`: each stop line there
    or not as the rule has it) and the same pairing mode (`dealt`, in the duel's first line, which says how the decks
    were dealt)."""
    if f"/{champion}:" not in text:
        return False
    head = next((line for line in text.splitlines() if line.startswith(f"{seat} against {seat}, ")), None)
    if head is None or dealt not in head:
        return False
    for kind, start in lines.items():
        found = re.search(rf"^{re.escape(kind)}.*$", text, re.M)
        if (found is None) != (start is None) or (found is not None and not found.group(0).startswith(start)):
            return False
    return True


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

    def fail(self, stage: str, seconds: float, why: str):
        """A measurement that failed: recorded, not done (a resumed generation tries it again)."""
        self.data["stages"][stage] = dict(done=False, failed=why, seconds=round(seconds, 1), finished=now())
        self.save()

    def save(self):
        if self.dry_run:
            return
        self.dir.mkdir(parents=True, exist_ok=True)
        self.path.write_text(json.dumps(self.data, indent=1), encoding="utf-8")
        (self.dir / "README.md").write_text(readme(self.data), encoding="utf-8")


def _f(v, fmt="{:.3f}"):
    return "-" if v is None else fmt.format(v)


def duel_text(r: dict) -> str:
    """A duel's result in a line: unpaired, then by pair where the duel printed it."""
    drawn = f", {r['draws']} drawn" if r.get("draws") else ""
    text = f"{r['wins']} to {r['losses']}{drawn} ({r['share']:.1%} +/- {r['margin']:.1%}) over {r['games']} games"
    if r.get("paired_share") is not None:
        text += (f"; by pair {r['paired_share']:.1%} +/- {r['paired_margin']:.1%} over {r['pairs']} pairs "
                 f"(0 / 0.5 / 1 / 1.5 / 2: {' / '.join(str(c) for c in r['pentanomial'])}"
                 + (f", {r['split']:.1%} split" if r.get("split") is not None else "") + ")")
    return text


FIXED_EVAL_NOTE = ("fixed eval (the tactical bot's games: it measures the distance from the tactical bot, so its policy CE "
                   "is expected to rise as the policy improves; not a regression by itself)")


def fixed_eval_text(fx: dict) -> str:
    """`train.py`'s `fixed_eval` (`--eval-data`, by default `pi/eval`: tactical-bot games) as the README shows it."""
    return (f"{FIXED_EVAL_NOTE}: policy CE {_f(fx.get('policy_ce'))}, top1 {_f(fx.get('top1'))}, "
            f"value MSE {_f(fx.get('value_mse'))}")


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
        g = r.get("governor") or {}
        lanes = (f"up to {r['lanes']} lanes (a mean {g['lanes_running']['Mean']:.1f} running"
                 + (f", held by {g['limiter']}" if g.get("limiter") else "") + ")") if g else f"{r['lanes']} lanes"
        row("selfplay", f"{r['games']} games at @{r['simulations']} on {lanes} ({r['games_per_hour']:.0f} an hour), "
                        f"{r['samples']} samples ({r['visit_samples']} with visit targets), {r['capped']} capped, "
                        f"{r['dropped']} dropped, {r['fallbacks']} network fallbacks"
                        + (f", {r['resumed']} resumed" if r.get('resumed') else "")
                        + (f", {r['decks']}" if r.get('decks') else "")
                        + f"; first player won {r['first_player']:.1%}; seeds {r['seeds']}"
                        + (("; hosts: " + ", ".join(
                            f"{name} {h['games']} games" + (f" at {h['games_per_hour']:.0f} an hour" if h.get("games_per_hour") else "")
                            + (f" on {h['lanes']} lanes" if h.get("lanes") else "") + (f" ({h['limiter']})" if h.get("limiter") else "")
                            for name, h in r["hosts"].items()) + " (`selfplay/HOSTS.json`)") if r.get("hosts") else ""))
    if "heldout" in s:
        r = s["heldout"]
        row("heldout", f"the permanent held-out set chosen: {r['games']} games in {r['shards']} shards of {r['source']}'s "
                       f"self-play, moved to {r['directory']} (never trained on)")
    if "train" in s:
        r = s["train"]
        h = r.get("held_out") or {}
        text = (f"{r['steps']} steps, {r['seen']} samples seen, from {r['init']}; window: "
                + ", ".join(f"{Path(w['directory']).parent.name}/{Path(w['directory']).name} x{w['weight']} ({w['samples']})" for w in r["window"])
                + f"; held out: top1 {_f(h.get('top1'))}, top1 of the visits {_f(h.get('top1_visits'))}, "
                  f"policy CE {_f(h.get('policy_ce'))}, value MSE {_f(h.get('value_mse'))}, value sign {_f(h.get('value_sign'))}")
        if fx := r.get("fixed_eval"):
            text += f"; {fixed_eval_text(fx)}"
        row("train", text)
    if "export" in s:
        row("export", f"{s['export']['model']}")
    if "parity" in s:
        row("parity", s["parity"]["summary"])
    if "gate" in s:
        r = s["gate"]
        if r.get("skipped"):
            row("gate", f"not played ({r['skipped']})")
        else:
            row("gate", f"candidate gen{gen} against champion {r['champion']} at @{r['simulations']}, {r['decks']}: {duel_text(r)}"
                        + (f"; SPRT LLR {_f(r['llr'])}: {r['sprt']}" if r.get("sprt") else "")
                        + (f"; paired SPRT LLR {_f(r['paired_llr'])}: {r['paired_sprt']}" if r.get("paired_sprt") else "")
                        + (f"; interval stop: {r['interval_stop']}" if r.get("interval_stop") else "")
                        + ("" if r.get("sprt") or r.get("paired_sprt") else "; no SPRT"))
    if "promote" in s:
        r = s["promote"]
        row("promote", ("accepted: gen{0} is the champion".format(gen) if r["accepted"] else f"rejected: {r['champion']} stays the champion")
            + f" ({r['rule']})")
    for name in MEASUREMENTS:
        if s.get(name, {}).get("failed"):
            why = " ".join(str(s[name]["failed"]).split())
            row(name, f"failed, and passed over (a measurement): {why[:300]}")
    if "netduel" in s and not s["netduel"].get("failed"):
        r = s["netduel"]
        row("netduel", f"the networks alone (no search), candidate gen{gen} against {r['champion']}, {r['decks']}: "
                       f"{duel_text(r)}; a measurement, not the gate")
    if "evaluate" in s and not s["evaluate"].get("failed"):
        r = s["evaluate"]
        parts = [f"{n}: value MSE {_f(e.get('value_mse'), '{:.4f}')}, ECE {_f(e.get('ece'), '{:.4f}')}, "
                 f"policy CE {_f(e.get('policy_ce'), '{:.4f}')}, top-1 {_f(e.get('top1'))} on {e.get('choices', '-')} targets"
                 for n, e in r["networks"].items()]
        row("evaluate", f"the permanent held-out set ({r['games']} games, {r['positions']} positions; full-search policy "
                        f"targets): " + "; ".join(parts))
    if "ladder" in s and not s["ladder"].get("failed"):
        r = s["ladder"]
        row("ladder", "; ".join(f"{x['champion']} against {x['anchor']} at {x['budget']}: {duel_text(x)}" for x in r["rows"])
            + " (all in `ladder.md`)")
    if "retire" in s:
        row("retire", f"{s['retire']['deleted']} path(s) deleted, the last {s['retire']['keep']} generations kept")
    for k, at in enumerate(d.get("attempts", []), 1):
        g, t = at["stages"].get("gate", {}), at["stages"].get("train", {})
        h = t.get("held_out") or {}
        lines.append(f"| attempt {k} | {sum(v['seconds'] for v in at['stages'].values()) / 60:.1f} min | rejected and retrained "
                     f"({at.get('reason', '')}): {t.get('steps')} steps, value mix {t.get('value_mix')}, held-out value MSE "
                     f"{_f(h.get('value_mse'))}; gate {g.get('wins')} to {g.get('losses')} ({_f(g.get('share'), '{:.1%}')} +/- "
                     f"{_f(g.get('margin'), '{:.1%}')}"
                     + (f"; by pair {_f(g.get('paired_share'), '{:.1%}')} +/- {_f(g.get('paired_margin'), '{:.1%}')}"
                        if g.get("paired_share") is not None else "")
                     + f"), SPRT LLR {_f(g.get('llr'))}: {g.get('sprt')} |")
    total = sum(v["seconds"] for v in s.values()) + sum(v["seconds"] for at in d.get("attempts", []) for v in at["stages"].values())
    lines += ["", f"In all {total / 3600:.2f} h.", ""]
    return "\n".join(lines)


# --- Running ------------------------------------------------------------------------------------------------------


def retrain(rec: Record, gen_dir: Path, model_dir: Path, args, log=print):
    """Set a rejected generation's attempt aside, from its training on, so the stages run again."""
    s = rec.data["stages"]
    if not s.get("promote", {}).get("done") or s["promote"].get("accepted"):
        raise SystemExit(f"--retrain: gen{rec.gen} has no rejected attempt to set aside")
    later = LATER
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


def selfplay_stopped(out: Path, log_path: Path) -> str:
    """What the driver says when self-play's governor stopped it (`SELFPLAY_STOPPED`): why, from its report, and that
    a rerun resumes."""
    why, kept = "see its log", "its finished games"
    try:
        r = json.loads((out / "selfplay-report.json").read_text(encoding="utf-8"))
        why = (r.get("Stopped") or {}).get("Reason") or why
        kept = f"{r.get('Games', 0)} finished games"
    except (OSError, ValueError):
        pass
    return (f"self-play stopped by its governor ({why}): {kept} are in closed shards in {out}. The stage is not done; "
            f"run the driver again with the same arguments to resume it (log: {log_path})")


def run(cmd: list[str], log_path: Path, dry_run: bool, env: dict | None = None, log=print, stream=False,
        stopped_code: int | None = None) -> str:
    """Runs `cmd` with its output in `log_path` (and here, with `stream`); a failure ends the driver. With
    `stopped_code`, the process stops gracefully on Ctrl-C (it hears it too, from the console): the driver keeps waiting
    for it, and that exit code raises `Stopped` instead of a failure."""
    shown = " ".join(str(c) for c in cmd)
    if len(shown) > 2000:
        shown = f"{shown[:2000]} ... ({len(cmd)} arguments)"
    log(f"$ {shown}")
    if dry_run:
        return ""
    log_path.parent.mkdir(parents=True, exist_ok=True)
    environment = {**os.environ, **(env or {})}
    interrupts = 0

    def waiting(p, step):
        """`step()` until it returns. A Ctrl-C meanwhile reaches the process too (it shares the console), and it stops:
        the driver waits for it. The second stops its games in flight; at the third the driver kills it and stops."""
        nonlocal interrupts
        while True:
            try:
                return step()
            except KeyboardInterrupt:
                if stopped_code is None:
                    raise
                interrupts += 1
                if interrupts >= 3:
                    log("interrupted a third time: killing it (its open work is recovered when the stage runs again)")
                    p.kill()
                    p.wait()
                    raise
                log("interrupted: waiting for the process to stop and close its work"
                    + (" (Ctrl-C again stops its games in flight)" if interrupts == 1 else
                       "; its games in flight are stopped (Ctrl-C once more kills it)"))

    if stream:
        # Tee the stage's output here as well as the log, so its progress bar is visible while the driver runs.
        with log_path.open("wb") as f:
            p = subprocess.Popen([str(c) for c in cmd], cwd=REPO, env=environment, stdout=subprocess.PIPE,
                                 stderr=subprocess.STDOUT, bufsize=0)
            assert p.stdout is not None
            while chunk := waiting(p, lambda: p.stdout.read(4096)):
                f.write(chunk)
                f.flush()
                try:
                    sys.stderr.buffer.write(chunk)
                    sys.stderr.buffer.flush()
                except AttributeError:
                    sys.stderr.write(chunk.decode(errors="replace"))
                    sys.stderr.flush()
            code = waiting(p, p.wait)
            p.stdout.close()
    elif stopped_code is not None:
        with log_path.open("w") as f:
            p = subprocess.Popen([str(c) for c in cmd], cwd=REPO, env=environment, stdout=f, stderr=subprocess.STDOUT)
            code = waiting(p, p.wait)
    else:
        with log_path.open("w") as f:
            p = subprocess.run([str(c) for c in cmd], cwd=REPO, env=environment, stdout=f, stderr=subprocess.STDOUT)
        code = p.returncode
    text = log_path.read_text(encoding="utf-8", errors="replace")
    if stopped_code is not None and code == stopped_code:
        raise Stopped(code, text)
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
        # A process group of its own, so a Ctrl-C at the console does not take it down under a stage that is stopping
        # gracefully (self-play finishes its games in flight on it); the driver stops it when the stage ends.
        group = (dict(creationflags=subprocess.CREATE_NEW_PROCESS_GROUP) if os.name == "nt" else dict(start_new_session=True))
        self.process = subprocess.Popen([str(c) for c in cmd], cwd=REPO / "ai", stdout=self.file, stderr=subprocess.STDOUT,
                                        env={**os.environ, **env}, **group)
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


def request_stop(root: Path, gen: int | None, now: bool, log=print) -> Path:
    """`--stop`: `selfplay.stop` in the generation's self-play directory (the current one: started, not finished), which
    its self-play reads each tick (and a distributed driver passes on to its workers)."""
    gen = gen or next_generation(root)
    out = root / f"gen{gen}" / "selfplay"
    out.mkdir(parents=True, exist_ok=True)
    path = out / distributed.STOP_FILE
    path.write_text("now\n" if now else "", encoding="utf-8")
    log(f"asked gen{gen}'s self-play to stop{' now (its games in flight too)' if now else ''}: {path}")
    return path


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
    a.add_argument("--gate-simulations", type=int,
                   help="the gate's search budget, when it differs from self-play's (--simulations): self-play's fast "
                        "searches may search deeper than play does")
    a.add_argument("--full-simulations", type=int, default=0,
                   help="playout-cap randomisation in self-play: this budget for a share --full-share of searched decisions, "
                        "the only policy targets; the rest search with --simulations (0: off, every search --simulations)")
    a.add_argument("--full-share", type=float, default=0.25, help="the share of searched decisions given --full-simulations")
    a.add_argument("--full-search-batch", type=int, default=0,
                   help="leaves a full search sends to the network at once (0: --search-batch); fewer round trips for its larger budget")
    a.add_argument("--search-batch", type=int, default=8, help="leaves a self-play search sends to the network at once")
    a.add_argument("--lanes", type=int, help="self-play games at once, at most (default: with --selfplay-autoscale, "
                                              f"{AUTOSCALE_LANES}; without, one a CPU thread, or with the GPU server "
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
    a.add_argument("--selfplay-memory-budget", type=float, metavar="GB",
                   help="self-play starts a game only while its private memory with one more game stays under this "
                        "(cantrip-selfplay --memory-budget; default: no cap)")
    a.add_argument("--selfplay-memory-reserve", type=float, default=6.0, metavar="GB",
                   help="self-play starts a game only while the system's available memory less one more game stays above "
                        "this (cantrip-selfplay --memory-reserve)")
    a.add_argument("--selfplay-memory-commit-reserve", type=float, default=6.0, metavar="GB",
                   help="and its commit headroom less one more game above this (cantrip-selfplay --commit-reserve)")
    a.add_argument("--selfplay-memory-floor", type=float, default=2.0, metavar="GB",
                   help="available memory or commit headroom under this stops self-play gracefully, its finished games "
                        "kept; rerun to resume (cantrip-selfplay --memory-floor)")
    a.add_argument("--selfplay-autoscale", action=argparse.BooleanOptionalAction, default=True,
                   help="self-play scales its lanes to the first limiter of throughput, CPU or memory, --lanes the ceiling "
                        "(cantrip-selfplay --autoscale)")
    a.add_argument("--selfplay-scale-window", type=float, metavar="S",
                   help="seconds of throughput each scaling decision measures (cantrip-selfplay --scale-window; default 45)")
    a.add_argument("--selfplay-scale-min-gain", type=float, metavar="F",
                   help="the least marginal gain a lane added must bring, as a share of an average lane's throughput "
                        "(cantrip-selfplay --scale-min-gain; default 0.2)")
    a.add_argument("--selfplay-cpu-ceiling", type=float, metavar="F",
                   help="the system's busy CPU share past which self-play adds no lane (cantrip-selfplay --cpu-ceiling; "
                        "default 0.95)")
    a.add_argument("--selfplay-traces", action="store_true",
                   help="self-play writes each game's trace too (cantrip-selfplay --traces, into selfplay/traces), which "
                        "cantrip-reanalyse and cantrip-explore audit stand the games in again from; off by default")
    a.add_argument("--selfplay-search", action="append", default=[], metavar="KEY=VALUE",
                   help="a search setting for every self-play search (cantrip-selfplay --search; repeatable), e.g. "
                        "Worlds=1, Gumbel=true, GumbelNoise=true")
    a.add_argument("--selfplay-full-search", action="append", default=[], metavar="KEY=VALUE",
                   help="a search setting for the full searches alone (cantrip-selfplay --full-search; repeatable), e.g. "
                        "Worlds=4, RootShortcut=false")
    a.add_argument("--seat-settings", default="",
                   help="search settings the gate's and the ladder's searching neural seats play with, both sides, as "
                        "BotLab's `+Key=Value` suffix (e.g. '+Worlds=1+Gumbel=true'): the networks compared at the search "
                        "they will play with")
    a.add_argument("--sampled", type=int, default=20, help="decisions sampled at the temperature before the most visited is played")
    a.add_argument("--temperature", type=float, default=1.0)
    a.add_argument("--window", type=int, default=8, help="generations of self-play data trained on")
    a.add_argument("--older-weight", type=float, default=0.5)
    a.add_argument("--imitation-weight", type=float, default=0.2, help="gen0's data (the tactical bot's games)")
    a.add_argument("--train-hours", type=float, default=0.65, help="4,067 steps at --train-rate: generation 1 overfit its value at 1.5")
    a.add_argument("--train-rate", type=float, default=445.0, help="samples a second training runs at (sets the steps)")
    a.add_argument("--train-steps", type=int, help="instead of --train-hours")
    a.add_argument("--no-mine", action="store_true",
                   help="no pair mining (`mining.py`): the gate's pairs are not judged, and no targets are made from them")
    a.add_argument("--mine-window", type=int, default=2,
                   help="the generations whose mined targets a generation trains on (the last ones that have them)")
    a.add_argument("--mine-weight", type=float, default=8.0,
                   help="the newest mined targets' weight in the training window, halved per generation of age")
    a.add_argument("--mine-lanes", type=int, default=24, help="lanes of each deep judgement run (on a worker: all at once)")
    a.add_argument("--mine-local-lanes", type=int, default=8,
                   help="lanes of each judgement run here, with no worker (one run at a time, each search up to 2 GB)")
    a.add_argument("--mine-control", type=int, default=200, help="split pairs mined as a control")
    a.add_argument("--mine-heldout", type=float, default=0.15, help="the share of mined pairs held out for measuring")
    a.add_argument("--mine-timeout", type=float, default=8.0, help="hours to wait for a worker's judgements")
    a.add_argument("--extra-data", action="append", default=[], metavar="DIR[:WEIGHT]",
                   help="another directory of shards in the training window (repeatable), e.g. the pair miner's targets "
                        "(policy-only, `reanalysed`) at a weight above 1 to repeat them")
    a.add_argument("--lr", type=float, default=2e-4)
    a.add_argument("--warmup", type=int, default=200)
    a.add_argument("--value-mix", type=float, default=0.5, help="the root value's share of the value target on searched positions")
    a.add_argument("--train-args", default="",
                   help="more arguments for train.py, one string; {champion} is the champion's checkpoint")
    a.add_argument("--target-uses", type=float,
                   help="the steps: each search target used about this many times in its life in the window "
                        "(instead of --train-hours; --train-steps still wins)")
    a.add_argument("--search-target-repeat", type=float,
                   help="train.py's: each search target dealt this many times an epoch, the copies policy-only")
    a.add_argument("--imitation-policy-weight", type=float,
                   help="train.py's: the policy loss's weight on the imitation data (games with no search)")
    a.add_argument("--value-mix-full", type=float, help="train.py's: the root value's share on full searches")
    a.add_argument("--value-mix-fast", type=float, help="train.py's: the root value's share on fast searches")
    a.add_argument("--schedule", choices=["cosine", "constant"], default="cosine",
                   help="train.py's learning-rate schedule after the warm-up (constant: continuous training)")
    a.add_argument("--carry-optimizer", action="store_true",
                   help="start training with the champion's optimizer state (train.py --init-optimizer)")
    a.add_argument("--stop-after", choices=STAGES, help="stop once this stage is done (the generation resumes from the next)")
    a.add_argument("--train-init", help="the checkpoint this generation's training starts from instead of the champion's: "
                                        "a network of another size (a step up the size ladder), trained on its own beforehand")
    a.add_argument("--gate-games", type=int, default=6000, help="the most games the gate may take (an even number: pairs)")
    a.add_argument("--gate-h0", type=float, default=0.5, help="the SPRTs' H0, the candidate's share (sprt, sprt-paired)")
    a.add_argument("--gate-h1", type=float, default=0.52,
                   help="the SPRTs' H1, the candidate's share: Magic's variance hides a 55%% step, so a smaller one "
                        "(unpaired, about 3,700 games to a verdict at 50%% or 52%%, up to ~5,400 between; paired, fewer "
                        "by as much as the pairs split)")
    a.add_argument("--gate-alpha", type=float, default=0.05, help="the SPRTs' false-acceptance rate")
    a.add_argument("--gate-beta", type=float, default=0.05, help="the SPRTs' false-rejection rate")
    a.add_argument("--gate-parallel", type=int, help="gate games at once (default: one a CPU thread; with the GPU server, "
                                                      f"{SERVER_GATE} a CPU thread)")
    a.add_argument("--gate-pairings", choices=["metagame", "mirrors"], default="metagame",
                   help="the gate's decks: metagame-weighted pairings as self-play draws them, each pair of games one "
                        "deal with the players exchanged (a true duplicate), or the decks as a pool of mirrors (before "
                        "2026-11-13). Without --decks-dir, rw-aggro and ub-reanimator with mirrors either way")
    a.add_argument("--accept", choices=RULES, default="above-half",
                   help="the gate's rule (`accepted`): sprt (unpaired SPRT, H1 accepted), sprt-paired (the pentanomial "
                        "SPRT, H1 accepted), ahead (the unpaired share above a half by its margin), above-half (the paired "
                        "share above a half: anything over 50%% is a win), always (promoted with no gate; "
                        "--gate-measure plays it anyway, as a measurement). The SPRTs stop the gate early, above-half "
                        "with --gate-early-stop. Default above-half (the project's rule)")
    a.add_argument("--gate-early-stop", action="store_true",
                   help="with above-half: stop the gate once the paired interval excludes 50%% either way "
                        "(--gate-stop-z, --gate-stop-min-pairs, --gate-stop-min-decisive), else at --gate-games")
    a.add_argument("--gate-stop-z", type=float, default=2.58,
                   help="the early stop's interval, in standard errors (2.58, 99%%: it looks after every pair)")
    a.add_argument("--gate-stop-min-pairs", type=int, default=100, help="pairs counted before the early stop may stop")
    a.add_argument("--gate-stop-min-decisive", type=int, default=30,
                   help="pairs not split 1-1 counted before the early stop may stop (the interval rests on them)")
    a.add_argument("--gate-measure", action="store_true", help="with --accept always: play the gate anyway, as a measurement")
    a.add_argument("--net-duel-games", type=int, default=0,
                   help="a network-only duel (no search) of the candidate against the champion after the promotion, this "
                        "many games, by pair: cheap, and it resolves about 1%% in tens of thousands of games (0: off)")
    a.add_argument("--heldout-dir", help="the permanent held-out set of self-play games, never trained on (default "
                                         "ROOT/heldout); every window leaves it out")
    a.add_argument("--heldout-games", type=int, default=0,
                   help="choose the permanent held-out set now, once: this many games' whole shards moved from this "
                        "generation's self-play into --heldout-dir before its training (0: not; a set already chosen "
                        "is kept)")
    a.add_argument("--heldout-eval-games", type=int, default=1000,
                   help="games of the held-out set the evaluate stage scores (value MSE and ECE, policy CE and top-1 "
                        "against the full searches' targets), the candidate and the champion")
    a.add_argument("--no-heldout-eval", action="store_true", help="skip the evaluate stage even when a held-out set exists")
    a.add_argument("--ladder-every", type=int, default=0,
                   help="every N generations, after promotion, the champion against the fixed --ladder-anchors, "
                        "recorded in ROOT/ladder.json and ladder.md (0: off)")
    a.add_argument("--ladder-anchors", default="gen0,tactical",
                   help="the ladder's anchors, by commas: a champion or model by name (gen0), a model directory, or a "
                        "BotLab seat (tactical)")
    a.add_argument("--ladder-games", type=int, default=1000, help="games a ladder duel plays")
    a.add_argument("--ladder-budget", default=None, help="the ladder's search budget (default: --simulations)")
    a.add_argument("--ladder-play-every", type=int, default=0,
                   help="every Nth ladder also at --ladder-play-budget, the budget the bot plays people at (0: never)")
    a.add_argument("--ladder-play-budget", default="128", help="the play budget: N simulations (deterministic; 128 stands for 250 ms) or Nms")
    a.add_argument("--keep", type=int, default=8)
    a.add_argument("--no-build", action="store_true")
    a.add_argument("--retrain", metavar="REASON", help="run a rejected generation again from its training, saying why")
    a.add_argument("--workers", metavar="FILE",
                   help="distributed self-play: the workers file (docs/ai-training.md, 'Distributed self-play'); this "
                        "machine plays its share too unless --no-selfplay-local")
    a.add_argument("--selfplay-local", action=argparse.BooleanOptionalAction, default=True,
                   help="with --workers: this machine plays self-play games too")
    a.add_argument("--worker-lanes", type=int, default=256, help="a worker's lane ceiling (its own `lanes` wins)")
    a.add_argument("--worker-cuda-dlls", default=",".join(distributed.CUDA_DLLS),
                   help="with --workers: the CUDA libraries the bundle carries, by commas, from CANTRIP_CUDA_LIBS (or this "
                        "PyTorch's torch/lib); the default is what cantrip-infer loads")
    a.add_argument("--dist-chunk", type=int, help="with --workers: games granted at a time (default --selfplay-shard-games)")
    a.add_argument("--dist-lookahead", type=float, default=0.05, metavar="H",
                   help="with --workers: each host's backlog of games granted but not started, in hours of its rate "
                        "(beyond its lanes)")
    a.add_argument("--dist-poll", type=float, default=30.0, metavar="S", help="with --workers: seconds between polls")
    a.add_argument("--worker-timeout", type=float, default=600.0, metavar="S",
                   help="with --workers: a worker unreachable this long is lost and its unplayed games go to the others")
    a.add_argument("--worker-server-args",
                   help="with --workers: a worker's cantrip-infer arguments (its own `server_args` wins); default "
                        f"--server-args with --server-impl native, else '{distributed.WORKER_SERVER_ARGS}'. --shapes is refused")
    a.add_argument("--dist-lease", type=float, default=900.0, metavar="S",
                   help="with --workers: a self-play whose coordinator has not polled it for this long stops gracefully")
    a.add_argument("--abandon-workers", action="store_true",
                   help="finish a distributed generation's self-play on this machine alone, without its ledger")
    a.add_argument("--stop", action="store_true",
                   help="ask the generation's running self-play to stop gracefully (its stop file), and exit")
    a.add_argument("--stop-now", action="store_true", help="as --stop, and stop the games in flight too")
    a.add_argument("--dry-run", action="store_true", help="print the plan and run nothing")
    a.add_argument("--tiny", action="store_true", help="a toy scale, for testing the driver")
    args = a.parse_args(argv)
    if args.stop or args.stop_now:
        return request_stop(Path(args.root).resolve(), args.generation, args.stop_now)
    show_progress = args.progress == "on" or (args.progress == "auto" and sys.stderr.isatty())
    server = wants_server(args.gpu_server, args.device)
    cpus = os.cpu_count() or 4
    if args.lanes is None:
        args.lanes = AUTOSCALE_LANES if args.selfplay_autoscale else max(1, round(cpus * (SERVER_LANES if server else 1)))
    if args.gate_parallel is None:
        args.gate_parallel = max(1, round(cpus * (SERVER_GATE if server else 1)))
    if args.tiny:
        args.games, args.simulations, args.train_steps, args.gate_games, args.warmup = 4, 8, 4, 4, 1
        args.accept = "ahead" if args.accept in ("sprt", "sprt-paired") else args.accept
        args.net_duel_games = min(args.net_duel_games, 4)
        args.ladder_games = min(args.ladder_games, 4)
        args.heldout_eval_games = min(args.heldout_eval_games, 4)

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
    held = heldout_dir(root, args.heldout_dir)
    if args.retrain:
        retrain(rec, gen_dir, model_dir, args, log)

    def stage(name, fn):
        if rec.done(name) and name != "build":
            log(f"[{name}] done already ({rec.data['stages'][name]['seconds']:.0f} s)")
            return
        log(f"[{name}]")
        t = time.time()
        try:
            numbers = fn()
        except (Exception, SystemExit) as e:
            # A measurement decides nothing: its failure is recorded and the generation goes on.
            if name not in MEASUREMENTS or args.dry_run:
                raise
            why = str(e) or type(e).__name__
            log(f"[{name}] failed, recorded and passed over: {why.splitlines()[0] if why else ''}")
            rec.fail(name, time.time() - t, why[-2000:])
            return
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
        if (server and args.server_impl == "native") or args.workers:
            # On its own: the gpu feature loads onnxruntime at run time, which the other binaries are not built to.
            run(["cargo", "build", "--release", "-p", "cantrip-ai-host", "--features", "gpu", "--bin", "cantrip-infer"],
                gen_dir / "logs/build-infer.log", args.dry_run, stream=show_progress)
        run(["cargo", "build", "--profile", "cards", "-p", "cantrip-host", "--bin", "cantrip-native-host"],
            gen_dir / "logs/build-native.log", args.dry_run, stream=show_progress)
        if not args.no_mine:
            # The recording native host for the pair miner's replays, in a target directory of its own (the same binary
            # with a feature on would otherwise take the place of the table's).
            run(["cargo", "build", "--profile", "cards", "-p", "cantrip-host", "--bin", "cantrip-native-host",
                 "--features", "ai-record"], gen_dir / "logs/build-native-record.log", args.dry_run,
                env=dict(CARGO_TARGET_DIR=str(REPO / "target" / "record")), stream=show_progress)
        run([*dotnet(), "build", "dotnet/tools/Cantrip.BotLab/Cantrip.BotLab.csproj", "-c", "Release", "-v", "q", "--nologo"],
            gen_dir / "logs/build-botlab.log", args.dry_run, env=dict(DOTNET_ROLL_FORWARD="Major"), stream=show_progress)

    def selfplay_command(exe, model, out, lanes) -> list:
        """cantrip-selfplay's command line but for its decks, its resume and its backend: the same on every host."""
        cmd = [exe, "--model", model, "--out", out, "--games", args.games, "--simulations", args.simulations,
               "--lanes", lanes, "--seed", seed, "--sampled", args.sampled, "--temperature", args.temperature,
               "--mode", args.mode, "--shard-games", args.selfplay_shard_games, "--batch", args.search_batch]
        if args.full_simulations:
            cmd += ["--full-simulations", args.full_simulations, "--full-share", args.full_share]
            if args.full_search_batch:
                cmd += ["--full-batch", args.full_search_batch]
        if args.selfplay_traces:
            cmd.append("--traces")
        for kv in args.selfplay_search:
            cmd += ["--search", kv]
        for kv in args.selfplay_full_search:
            cmd += ["--full-search", kv]
        # The governor: memory admission and its graceful stop, and the lanes scaled to the first limiter.
        if args.selfplay_memory_budget is not None:
            cmd += ["--memory-budget", args.selfplay_memory_budget]
        cmd += ["--memory-reserve", args.selfplay_memory_reserve, "--commit-reserve", args.selfplay_memory_commit_reserve,
                "--memory-floor", args.selfplay_memory_floor]
        if not args.selfplay_autoscale:
            cmd.append("--no-autoscale")
        for flag, value in (("--scale-window", args.selfplay_scale_window), ("--scale-min-gain", args.selfplay_scale_min_gain),
                            ("--cpu-ceiling", args.selfplay_cpu_ceiling)):
            if value is not None:
                cmd += [flag, value]
        return cmd

    def selfplay():
        out = gen_dir / "selfplay"
        resume = out.exists() and any(p.suffix == ".zst" or p.suffix == ".part" for p in out.iterdir())
        if out.exists() and not args.dry_run and not resume:
            shutil.rmtree(out)
        if not args.dry_run:
            # A stop asked of an earlier run (`--stop`) is not this one's.
            (out / distributed.STOP_FILE).unlink(missing_ok=True)
        cmd = selfplay_command(SELFPLAY, champ, out, args.lanes)
        if resume or args.workers:
            # Distributed, the directory is resumed whatever it holds: the workers' shards come into it.
            cmd.append("--resume")
        listed = None
        if decks_dir is not None:
            # A pairing a game, in a file: as arguments they pass Windows' 32K command line.
            listed = gen_dir / "selfplay-pairings.txt"
            if not args.dry_run:
                gen_dir.mkdir(parents=True, exist_ok=True)
                listed.write_text("".join(p + "\n" for p in metagame_pairings(pool, args.games, seed)), encoding="utf-8")
            cmd += ["--decks", decks_dir, "--prior", decks_dir, "--pairings", listed]
        if args.workers:
            return distributed_selfplay(cmd, out, listed)
        if (gen_dir / "dist" / "ledger.json").exists() and not args.abandon_workers and not args.dry_run:
            # Its workers may still be playing (or hold games it granted them), and its ledger is what keeps each game
            # once: finishing the stage here alone would play them twice, or leave them out.
            raise SystemExit(f"gen{gen}'s self-play is distributed ({gen_dir / 'dist' / 'ledger.json'}): run it with "
                             "--workers as it began, or --abandon-workers to finish it on this machine alone (stop its "
                             "workers first: their shards are then not collected)")
        with server_for([champ], "selfplay") as gpu:
            if gpu is not None:
                cmd += ["--backend", "remote", "--server", gpu.address]
            try:
                run(cmd, gen_dir / "logs/selfplay.log", args.dry_run, env=dict(CANTRIP_PROGRESS="1") if show_progress else None,
                    stream=show_progress, stopped_code=SELFPLAY_STOPPED)
            except Stopped:
                raise SystemExit(selfplay_stopped(out, gen_dir / "logs/selfplay.log"))
        if args.dry_run:
            return
        r = json.loads((out / "selfplay-report.json").read_text(encoding="utf-8"))
        numbers = dict(games=r["Games"], resumed=r.get("Resumed", 0), games_this_run=r.get("GamesThisRun", r["Games"]),
                       samples=r["Samples"], visit_samples=r["VisitSamples"], capped=r["Capped"], dropped=r["Dropped"],
                       fallbacks=r["NetworkFallbacks"], simulations=r["Simulations"], lanes=r["Lanes"], games_per_hour=r["GamesPerHour"],
                       searched_per_second=r["SearchedPerSecond"], evaluations_per_second=r["EvaluationsPerSecond"],
                       first_player=r["FirstPlayerWinRate"], mean_turns=r["MeanTurns"], seeds=r["Seeds"], producer=r["Producer"],
                       wins=r["Wins"], errors=r["Errors"], failures=r["Failures"])
        if g := r.get("Governor"):
            numbers["governor"] = dict(lanes_running=g["LanesRunning"], memory=g["Memory"], deferred=g["Deferred"],
                                       limiter=(g.get("Autoscale") or {}).get("Limiter"))
        if decks_dir is not None:
            numbers["decks"] = f"{len(pool)} decks from {decks_dir.name}, metagame-weighted pairings"
        return numbers

    def distributed_selfplay(cmd: list, out: Path, listed: Path | None) -> dict | None:
        """The self-play stage on this machine and the workers (`distributed.py`): run to the end, its numbers from the
        shards accepted and every host's part."""
        workers = distributed.load_workers(Path(args.workers))
        dist_dir = gen_dir / "dist"
        tag = distributed.root_tag(root)
        log(f"  distributed self-play: {len(workers)} worker(s) ({', '.join(w['name'] for w in workers) or 'none'})"
            + (" and this machine" if args.selfplay_local else "") + f"; ledger {dist_dir / 'ledger.json'}")
        default_server = args.worker_server_args
        if default_server is None:
            # The native server's settings, not serve.py's (whose arguments `--server-args` holds with the python one).
            default_server = args.server_args if args.server_impl == "native" and args.server_args.strip() else distributed.WORKER_SERVER_ARGS
        server_words = {w["name"]: distributed.worker_server_args(w, default_server) for w in workers}
        if args.dry_run:
            for w in workers:
                log(f"  would push the bundle to {w['user']}@{w['host']}:{w['root']}/bundle/<hash> and play in "
                    f"{w['root']}/gens/{tag}/gen{gen}/{w['name']}/<attempt>-<nonce>, cantrip-infer {' '.join(server_words[w['name']])}")
            log(f"$ {' '.join(str(c) for c in cmd)} --grant {dist_dir / '<attempt>.grant'} --host {distributed.local_name()} "
                f"--lease {dist_dir / '<attempt>.heartbeat'} --lease-seconds {args.dist_lease:g}")
            return None
        cache_path = root / "dist" / "hashes.json"
        cache = distributed.read_json(cache_path) or {}
        files = manifest = bundle_hash = None
        if workers:
            cuda = os.environ.get("CANTRIP_CUDA_LIBS")
            if not cuda:
                import torch  # noqa: PLC0415

                cuda = str(Path(torch.__file__).parent / "lib")
            if not args.ort_dylib:
                raise SystemExit("--workers needs onnxruntime-gpu for the workers' cantrip-infer: --ort-dylib or ORT_DYLIB_PATH")
            files = distributed.bundle_files(SELFPLAY, INFER, Path(args.ort_dylib).parent, Path(cuda), champ, decks_dir,
                                             decks_dir, listed, [n.strip() for n in args.worker_cuda_dlls.split(",") if n.strip()])
            bundle_hash, manifest = distributed.bundle_manifest(files, cache)
            log(f"  worker bundle {bundle_hash}: {len(files)} files, "
                f"{sum(m['size'] for m in manifest.values()) / 2**30:.2f} GB")
        # What this self-play is (`Ledger.identity`): a ledger made with other settings is refused, and so is a host whose
        # own settings changed.
        model_files = [p for p in champ.iterdir() if p.is_file() and (p.name == "manifest.json" or p.suffix == ".onnx")]
        global_identity = dict(
            model=distributed.files_hash(model_files, cache), games=args.games, seed=seed, mode=args.mode,
            simulations=args.simulations, full_simulations=args.full_simulations, full_share=args.full_share,
            full_batch=args.full_search_batch, batch=args.search_batch, sampled=args.sampled, temperature=args.temperature,
            search=list(args.selfplay_search), full_search=list(args.selfplay_full_search),
            pairings=distributed.files_hash([listed], cache) if listed is not None else None,
            decks=distributed.files_hash(sorted(decks_dir.glob("*.txt")), cache) if decks_dir is not None else None)
        local_name = distributed.local_name()
        hosts_identity = {w["name"]: dict(bundle=bundle_hash, backend=w.get("backend", "gpu"),
                                          server_args=" ".join(server_words[w["name"]]),
                                          precision=distributed.precision_of(server_words[w["name"]]))
                          for w in workers}
        local_identity = dict(binary=distributed.sha256(SELFPLAY, cache)[:16],
                              backend="server" if server else "cpu",
                              server_args=args.server_args if server else None,
                              precision=distributed.precision_of(args.server_args.split()) if server else None)
        if args.selfplay_local:
            hosts_identity[local_name] = local_identity
        cache_path.parent.mkdir(parents=True, exist_ok=True)
        distributed.write_atomic(cache_path, json.dumps(cache))
        # The header every shard must carry, asked of this binary and model before any game: never learnt from a shard.
        printed = subprocess.run([str(c) for c in cmd] + ["--print-header"], cwd=REPO, capture_output=True, text=True)
        if printed.returncode != 0:
            raise SystemExit(f"cantrip-selfplay --print-header failed:\n{printed.stdout}{printed.stderr}")
        header = json.loads(printed.stdout.strip().splitlines()[-1])

        def job(host, attempt, d) -> dict:
            """A worker's attempt: cantrip-infer and cantrip-selfplay from the bundle, the search settings this
            machine's, the governor on, the lanes its ceiling."""
            w, b = host.w, host.bundle_dir()
            model = f"{b}/model/{champ.name}"
            # Its arguments: the executable is the job's own.
            remote = [str(c) for c in selfplay_command(f"{b}/bin/cantrip-selfplay.exe", model, f"{d}/selfplay",
                                                       w.get("lanes") or args.worker_lanes)[1:]]
            if decks_dir is not None:
                remote += ["--decks", f"{b}/decks", "--prior", f"{b}/decks", "--pairings", f"{b}/pairings.txt"]
            remote.append("--resume")
            # The recovery opens no evaluator: a backend that needs no server.
            recover = remote + ["--backend", "tract", "--recover"]
            remote += ["--grant", f"{d}/grant.txt", "--host", host.name]
            env = {"ORT_DYLIB_PATH": f"{b}/bin/onnxruntime.dll", "CANTRIP_CUDA_LIBS": f"{b}/bin", "CANTRIP_PROGRESS": "0"}
            mine = hosts_identity[host.name]
            record = dict(server_args=mine["server_args"], precision=mine["precision"], bundle=bundle_hash)
            if w.get("backend", "gpu") == "gpu":
                return dict(selfplay=f"{b}/bin/cantrip-selfplay.exe", selfplay_args=remote + ["--backend", "remote"],
                            recover_args=recover, env=env, infer=f"{b}/bin/cantrip-infer.exe", record=record,
                            infer_args=["serve", "--model", model, "--port", "0", "--ready-file", f"{d}/infer.ready",
                                        "--device", str(w.get("device", 0)), *server_words[host.name]])
            return dict(selfplay=f"{b}/bin/cantrip-selfplay.exe", selfplay_args=remote + ["--backend", w["backend"]],
                        recover_args=recover, env=env, infer=None, infer_args=[], record=dict(record, server_args=None, precision=None))

        started = time.time()
        with distributed.LedgerLock(dist_dir / "ledger.lock"):
            ledger = distributed.Ledger.load(dist_dir / "ledger.json", args.games, seed,
                                             identity=dict({"global": global_identity, "hosts": hosts_identity}),
                                             header=header)
            ledger.save()
            with server_for([champ], "selfplay") as gpu:
                local = list(cmd)
                if gpu is not None:
                    local += ["--backend", "remote", "--server", gpu.address]
                hosts = []
                if args.selfplay_local:
                    hosts.append(distributed.LocalHost(local_name, local, out, dist_dir, gen_dir / "logs",
                                                       env=dict(CANTRIP_PROGRESS="0"), cwd=REPO, lease=args.dist_lease,
                                                       identity={k: v for k, v in local_identity.items() if k in ("server_args", "precision") and v}))
                hosts += [distributed.SshHost(w, tag, gen, files, manifest, bundle_hash, job, log, lease=args.dist_lease)
                          for w in workers]
                if not hosts:
                    raise SystemExit("--workers with --no-selfplay-local and no worker: no host to play on")
                pairings = (sum(1 for line in listed.read_text(encoding="utf-8").splitlines() if line.strip())
                            if listed is not None else 3)
                coordinator = distributed.Coordinator(
                    ledger, hosts, out, dist_dir, read=lambda p: distributed.read_shard(p, seed, pairings), log=log,
                    chunk=args.dist_chunk or args.selfplay_shard_games, lookahead=args.dist_lookahead,
                    initial=args.dist_chunk or args.selfplay_shard_games, poll_seconds=args.dist_poll,
                    timeout=args.worker_timeout, heldout=held)
                result = coordinator.run()
            (out / distributed.HOSTS).write_text(json.dumps(distributed.hosts_manifest(ledger), indent=1), encoding="utf-8")
        if result == "stopped":
            raise SystemExit(f"self-play stopped on every host: {len(ledger.done)} of {args.games} games are in closed "
                             f"shards in {out}. The stage is not done; run the driver again with the same arguments to "
                             f"resume it (the ledger: {dist_dir / 'ledger.json'})")
        note = f"{len(pool)} decks from {decks_dir.name}, metagame-weighted pairings" if decks_dir is not None else None
        n = distributed.numbers(ledger, args.simulations, time.time() - started, pairings, note)
        for name, h in n["hosts"].items():
            log(f"  {name}: {h['games']} games" + (f", {h['games_per_hour']:.0f} an hour" if h.get("games_per_hour") else "")
                + (f", {h['lanes']} lanes, held by {h.get('limiter')}" if h.get("lanes") else ""))
        return n

    def heldout():
        return designate_heldout(gen_dir / "selfplay", held, args.heldout_games, gen, args.dry_run, log)

    def train():
        w = window(root, gen, args.window, args.older_weight, args.imitation_weight, exclude=[held])
        for d, weight in w:
            log(f"  window: {d} x{weight}")
        out = model_dir / "train"
        resuming = (out / "checkpoint.pt").exists()
        if out.exists() and not args.dry_run and not resuming:
            shutil.rmtree(out)
        steps = args.train_steps or max(1, int(args.train_hours * 3600 * args.train_rate / 256))
        uses = None
        if args.target_uses and not args.train_steps:
            # This generation's share of a search target's uses: its own run, then each later one it stays in the
            # window for, at the older generations' weight.
            uses = args.target_uses / (1 + (args.window - 1) * args.older_weight)
        init = Path(args.train_init) if args.train_init else champ / "train" / "checkpoint.pt"
        if args.train_init and not init.exists() and not args.dry_run:
            raise SystemExit(f"--train-init {init}: no such checkpoint")
        length = ["--steps", steps] if uses is None else ["--target-uses", f"{uses:.4g}"]
        extra = [str(e) for e in args.extra_data]
        if not args.no_mine:
            # The pair miner's targets of the generations before (`mining.window_dirs`).
            extra += mining.window_dirs(root, gen, args.mine_window, args.mine_weight)
        for e in extra:
            log(f"  extra: {e}")
        cmd = [sys.executable, "-m", "cantrip_ai.train", "--data", *[f"{d}:{weight:g}" for d, weight in w], *extra, "--out", out,
               *length, "--lr", args.lr, "--warmup", args.warmup, "--value-mix", args.value_mix, "--seed", gen,
               "--device", args.device, "--progress", "on" if show_progress else "off"]
        if args.train_hours and not args.train_steps and uses is None:
            cmd += ["--hours", round(args.train_hours * 1.25, 3)]
        for flag, value in (("--search-target-repeat", args.search_target_repeat),
                            ("--imitation-policy-weight", args.imitation_policy_weight),
                            ("--value-mix-full", args.value_mix_full), ("--value-mix-fast", args.value_mix_fast)):
            if value is not None:
                cmd += [flag, value]
        if args.schedule != "cosine":
            cmd += ["--schedule", args.schedule]
        if resuming:
            cmd.append("--resume")
        elif init.exists() or args.dry_run:
            cmd += ["--init", init]
            if args.carry_optimizer:
                cmd.append("--init-optimizer")
        champion_checkpoint = str(champ / "train" / "checkpoint.pt")
        cmd += [word.replace("{champion}", champion_checkpoint) for word in args.train_args.split()]
        run(cmd, gen_dir / "logs/train.log", args.dry_run, env=dict(PYTHONPATH=str(REPO / "ai")), stream=show_progress)
        if args.dry_run:
            return
        m = json.loads((out / "metrics.json").read_text(encoding="utf-8"))["history"][-1]
        import torch  # the checkpoint's data record: what each window directory gave

        ck = torch.load(out / "checkpoint.pt", map_location="cpu", weights_only=False)
        c = ck["config"]
        params = sum(v.numel() for k, v in ck["state"].items() if k != "table" and not k.startswith("feat_"))
        if m.get("fixed_eval"):
            log(f"  {fixed_eval_text(m['fixed_eval'])}")
        return dict(steps=m["step"], seen=m["seen"], held_out=m.get("held_out"), fixed_eval=m.get("fixed_eval"),
                    size=f"{c['width']}x{c['layers']}", parameters=params, seconds_training=round(m.get("seconds", 0.0), 1),
                    precision=ck["args"].get("precision", "fp32"),
                    init="resumed" if resuming else (str(init) if args.train_init else (init.parent.parent.name if init.exists() else "fresh")),
                    lr=args.lr, value_mix=args.value_mix, window=ck["data"]["directories"], samples=ck["data"]["samples"],
                    **{k: v for k, v in dict(target_uses=args.target_uses, search_target_repeat=args.search_target_repeat,
                                             imitation_policy_weight=args.imitation_policy_weight,
                                             value_mix_full=args.value_mix_full, value_mix_fast=args.value_mix_fast,
                                             schedule=args.schedule if args.schedule != "cosine" else None,
                                             carry_optimizer=args.carry_optimizer or None,
                                             train_args=args.train_args or None).items() if v is not None})

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
                # A server below fp32 is held to the reduced bar, the one it checks itself against as it starts.
                said = args.server_args.split()
                if "--precision" in said and said[said.index("--precision") + 1:][:1] != ["fp32"]:
                    env["CANTRIP_AI_SERVER_REDUCED"] = "1"
            text = run(["cargo", "test", "--profile", "cards", "-p", "cantrip-ai", "--features", "ort", "--test", "eval",
                        "model_parity", "--", "--ignored", "--nocapture"], gen_dir / "logs/parity.log", args.dry_run,
                       env=env, stream=show_progress)
        said = [l for l in text.splitlines() if "parity" in l.lower() or "max" in l.lower()]
        return dict(summary=(said[-1].strip() if said else "passed").replace("|", "/"))

    def dealt_marker() -> str:
        """What BotLab's first line of a duel of this generation says of its decks (`Program.Duel`'s `dealt`)."""
        if decks_dir is None:
            return "rw-aggro vs ub-reanimator"
        if args.gate_pairings == "mirrors":
            return " decks, each a mirror"
        return "in the pairings of gate-pairings.txt, each pair one deal"

    def dealt_text() -> str:
        """The decks a duel of this generation plays, in words."""
        if decks_dir is None:
            return "rw-aggro and ub-reanimator with mirrors"
        if args.gate_pairings == "mirrors":
            return f"{len(pool)} standard decks ({decks_dir.name}), each a mirror"
        return f"{len(pool)} standard decks ({decks_dir.name}), metagame-weighted pairings, each pair of games one deal"

    def dealt(name: str, games: int, at: int) -> list:
        """BotLab's decks for a duel of `games` games. With --decks-dir and metagame pairings, the matchups drawn by
        metagame share as self-play's are (`metagame_pairings`, at the duel's own seed), a line a pair of games,
        written to genN/<name>-pairings.txt: BotLab deals both games of a pair from one shuffle with the players
        exchanged. Else the decks as a pool of mirrors, or the fixed slice."""
        if decks_dir is None:
            return ["--deck", "rw-aggro", "--deck", "ub-reanimator", "--mirrors"]
        if args.gate_pairings == "mirrors":
            # BotLab's pool of mirrors: both seats the same standard list, each pair of games the next list.
            return ["--decks", decks_dir, "--ai-prior", decks_dir]
        listed = gen_dir / f"{name}-pairings.txt"
        if not args.dry_run:
            gen_dir.mkdir(parents=True, exist_ok=True)
            listed.write_text("".join(p + "\n" for p in metagame_pairings(pool, max(1, (games + 1) // 2), at)), encoding="utf-8")
        return ["--decks", decks_dir, "--ai-prior", decks_dir, "--pairings", listed]

    def duel(name: str, a: str, b: str, models: list[Path], games: int, at: int, extra: list, b_model: Path | None) -> dict | None:
        """A BotLab duel, A playing `models[0]`, B `b_model` (or a BotLab seat by name), logged to genN/logs/NAME.log;
        its summary read by `parse_gate`, or None in a dry run."""
        cmd = [*dotnet(), BOTLAB, "duel", "--a", a, "--b", b, "--model", models[0]]
        if b_model is not None:
            cmd += ["--model-b", b_model]
        cmd += [*dealt(name, games, at), "--games", games, "--parallel", args.gate_parallel, "--seed", at, *extra]
        # Every game's result beside the log (BotLab's --results): at a fixed budget a game is played again exactly from
        # its number and the duel's seed, seats and decks (--only-games), so any pair's positions can be mined later.
        cmd += ["--results", gen_dir / "logs" / f"{name}-results.tsv"]
        env = {"DOTNET_ROLL_FORWARD": "Major", "CANTRIP_NATIVE_HOST": str(NATIVE), "CANTRIP_AI_HOST": str(AI_HOST)}
        with server_for(models, name) as gpu:
            if gpu is not None:
                cmd += ["--ai-server", gpu.address]
            text = run(cmd, gen_dir / "logs" / f"{name}.log", args.dry_run, env=env, stream=show_progress)
        return None if args.dry_run else parse_gate(text)

    def mine_settings() -> mining.Settings:
        return mining.Settings(lanes=args.mine_lanes, control=args.mine_control, heldout=args.mine_heldout,
                               seed=seed % 1_000_000 + 7)

    def mine():
        """The pair miner's first half (`mining.py`): the gate's decisive pairs (and a control) replayed and recorded on
        this machine, their forks and losers' decisions found and planned, the deep judgements started (on the first
        worker, else here). The judge is the network the gate leaves champion."""
        p, s = mining.Paths(gen_dir), mine_settings()
        results = gen_dir / "logs" / "gate-results.tsv"
        accepted = (rec.data["stages"].get("promote") or {}).get("accepted")
        judge_model = model_dir if accepted else champ
        py = [sys.executable, "-m", "cantrip_ai.mine"]
        pyenv = dict(PYTHONPATH=str(REPO / "ai"))
        logs = gen_dir / "logs"
        if args.dry_run:
            log(f"  would mine the gate's pairs in {p.root}, judged by {judge_model.name}"
                + (f" on {distributed.load_workers(Path(args.workers))[0]['name']}" if args.workers else " here"))
            return
        if not results.exists():
            raise SystemExit(f"no gate results to mine ({results})")
        # A stage run again keeps its replays when they were made and verified against these gate results: the replay is
        # this stage's longest step, and the same games again would be the same recordings.
        verified_file = p.replay / "verify.json"
        resumed = (verified_file.exists() and (p.selected / "selection.json").exists()
                   and verified_file.stat().st_mtime > results.stat().st_mtime)
        for d in ((p.judge, p.keep, p.targets, p.heldout) if resumed
                  else (p.selected, p.replay, p.recorded, p.judge, p.keep, p.targets, p.heldout)):
            mining.clear(d)
        p.root.mkdir(parents=True, exist_ok=True)
        if resumed:
            log(f"  the replays verified before are kept ({verified_file})")
        else:
            run([*py, "select", "--results", results, "--out", p.selected, "--control", s.control, "--seed", s.seed],
                logs / "mine-select.log", False, env=pyenv)
        # The gate's games again, exactly: the gate's seats, networks, decks, pairings, seed and server.
        name = seat_name(args.mode, args.gate_simulations or args.simulations) + args.seat_settings
        if decks_dir is not None and args.gate_pairings != "mirrors":
            decks = ["--decks", decks_dir, "--ai-prior", decks_dir, "--pairings", gen_dir / "gate-pairings.txt"]
        else:
            decks = dealt("gate", args.gate_games, seed + 5_000_000)
        env = {"DOTNET_ROLL_FORWARD": "Major", "CANTRIP_NATIVE_HOST": str(RECORD_NATIVE), "CANTRIP_AI_HOST": str(AI_HOST),
               "CANTRIP_AI_TRACES": "1"}
        p.replay.mkdir(parents=True, exist_ok=True)
        replayed = []
        with server_for([model_dir, champ], "mine-replay") if not resumed else contextlib.nullcontext(None) as gpu:
            for half in ("first", "second"):
                games = p.selected / f"{half}.txt"
                if not games.read_text(encoding="utf-8").strip():
                    continue
                out = p.replay / f"{half}-results.tsv"
                replayed.append(out)
                if resumed:
                    continue
                cmd = [*dotnet(), BOTLAB, "duel", "--a", name, "--b", name, "--model", model_dir, "--model-b", champ, *decks,
                       "--games", args.gate_games, "--parallel", args.gate_parallel, "--seed", seed + 5_000_000,
                       "--only-games", games, "--results", out, "--ai-record", p.recorded / half,
                       "--ai-record-mode", "decklist", "--ai-record-pairs"]
                if gpu is not None:
                    cmd += ["--ai-server", gpu.address]
                run(cmd, logs / f"mine-replay-{half}.log", False, env=env, stream=show_progress)
        if not resumed:
            run([*py, "verify", "--gate", results, "--replayed", *replayed, "--out", verified_file],
                logs / "mine-verify.log", False, env=pyenv)
        verified = json.loads((p.replay / "verify.json").read_text(encoding="utf-8"))
        dropped = mining.prune_selection(p, verified)
        log(f"  replayed {verified['games']} games, {verified['same']} the gate's"
            + (f"; {dropped} pair(s) whose games differ left out" if dropped else ""))
        run([EXPLORE, "mine-forks", "--first", p.recorded / "first", "--second", p.recorded / "second", "--out", p.forks,
             "--lanes", 32], logs / "mine-forks.log", False)
        # The models as directories: mine-values loads each loser's network from its decision's `model`.
        run([*py, "losers", "--selected", p.selected, "--a-model", model_dir, "--b-model", champ],
            logs / "mine-losers.log", False, env=pyenv)
        values = []
        with server_for([model_dir, champ], "mine-values") as gpu:
            for half in ("first", "second"):
                out = p.values(half)
                run([EXPLORE, "mine-values", "--dir", p.recorded / half, "--games", p.selected / f"losers-{half}.jsonl",
                     "--out", out, "--lanes", 16, *(["--server", gpu.address] if gpu is not None else [])],
                    logs / f"mine-values-{half}.log", False)
                values.append(out)
        run([*py, "plan", "--selected", p.selected, "--forks", p.forks, "--values", *values, "--out", p.plan,
             "--heldout", s.heldout, "--before", s.before, "--seed", s.seed], logs / "mine-plan.log", False, env=pyenv)
        planned = mining.split_plan(p)
        log(f"  planned {planned['forks']} forks and {planned['losers']} losers' decisions")
        where = "here"
        if args.workers:
            worker = distributed.load_workers(Path(args.workers))[0]
            cuda = os.environ.get("CANTRIP_CUDA_LIBS")
            if not cuda:
                import torch  # noqa: PLC0415

                cuda = str(Path(torch.__file__).parent / "lib")
            cache_path = root / "dist" / "hashes.json"
            cache = distributed.read_json(cache_path) or {}
            files = mining.bundle(EXPLORE, INFER, Path(args.ort_dylib).parent, Path(cuda), judge_model, decks_dir,
                                  [n.strip() for n in args.worker_cuda_dlls.split(",") if n.strip()])
            judge = mining.RemoteJudge(worker, distributed.root_tag(root), gen, files, cache, log)
            cache_path.parent.mkdir(parents=True, exist_ok=True)
            distributed.write_atomic(cache_path, json.dumps(cache))
            default = args.worker_server_args
            if default is None:
                default = (args.server_args if args.server_impl == "native" and args.server_args.strip()
                           else distributed.WORKER_SERVER_ARGS)
            nonce = distributed.secrets.token_hex(4)
            judge.start(mining.judge_data(p), judge_model.name, s, distributed.worker_server_args(worker, default), nonce)
            p.job.write_text(json.dumps(dict(worker=worker["name"], nonce=nonce, model=judge_model.name)), encoding="utf-8")
            where = worker["name"]
        else:
            local_judge(p, s, judge_model)
            p.job.write_text(json.dumps(dict(worker=None, model=judge_model.name)), encoding="utf-8")
        chosen = mining.selection(p)
        return dict(decisive=len(chosen["decisive"]), control=len(chosen["control"]), replayed=verified["games"],
                    same=verified["same"], dropped_pairs=dropped, planned=planned, judge=judge_model.name, judged_on=where)

    def local_judge(p: mining.Paths, s: mining.Settings, judge_model: Path):
        """The judgements here, a run at a time (each search up to 2 GB: `--mine-local-lanes` bounds the memory)."""
        local = mining.Settings(**{**s.__dict__, "lanes": args.mine_local_lanes})
        with server_for([judge_model], "mine-judge") as gpu:
            for r in mining.judge_runs(p, local, str(judge_model), str(decks_dir), p.root, str(p.judge)):
                run([EXPLORE, *r["args"], *(["--server", gpu.address] if gpu is not None else [])],
                    gen_dir / "logs" / f"mine-judge-{r['name']}.log", False, stream=show_progress)

    def mined():
        """The pair miner's second half: the judgements waited for and fetched, the decided forks and the loser
        decisions far from the network's prior kept, written as policy-only shards (`genN/mine/targets`, the next
        generations' training; `genN/mine/heldout`, measurement), and the report."""
        p = mining.Paths(gen_dir)
        if args.dry_run:
            log(f"  would keep and filter the judgements into {p.targets} and {p.heldout}, and write {p.report}")
            return
        if not p.job.exists():
            raise SystemExit("the mine stage started no judgements")
        job = json.loads(p.job.read_text(encoding="utf-8"))
        if job.get("worker"):
            worker = next(w for w in distributed.load_workers(Path(args.workers)) if w["name"] == job["worker"])
            judge = mining.RemoteJudge(worker, distributed.root_tag(root), gen, {}, {}, log)
            exit_ = mining.wait(judge, job["nonce"], 60.0, args.mine_timeout, log)
            if exit_.get("code") != 0:
                raise SystemExit(f"the judgements on {job['worker']} failed: {exit_}")
            mining.clear(p.judge)
            judge.fetch(job["nonce"], p.judge)
        py = [sys.executable, "-m", "cantrip_ai.mine"]
        pyenv = dict(PYTHONPATH=str(REPO / "ai"))
        logs = gen_dir / "logs"
        runs = [p.judge / name for name, _, _ in mining.FORK_RUNS]
        text = run([*py, "keep", "--selected", p.selected, "--forks", p.forks, "--plan", p.plan, "--runs", *runs,
                    "--losers", p.judge / mining.LOSERS_RUN, "--kl-min", mine_settings().kl_min, "--out", p.keep],
                   logs / "mine-keep.log", False, env=pyenv)
        kept = json.loads(text.strip().splitlines()[-1])
        producer = f"mine:{job['model']}@gen{gen}"
        for split, dest in (("train", p.targets), ("heldout", p.heldout)):
            for run_dir in (runs[0], p.judge / mining.LOSERS_RUN):
                if (run_dir / split).is_dir():
                    run([EXPLORE, "mine-filter", "--from", run_dir / split, "--keep", p.keep / f"keep-{split}.jsonl",
                         "--out", dest / run_dir.name, "--producer", producer],
                        logs / f"mine-filter-{split}-{run_dir.name}.log", False)
        settings = mine_settings()
        (p.root / "settings.json").write_text(json.dumps(dict(
            # The keys `mine report` reads.
            fork=f"{settings.fork_simulations} simulations x {settings.fork_worlds} worlds, two seeds pooled",
            target=f"{settings.fork_simulations} x {settings.fork_worlds}, the first seed's",
            loser=f"{settings.loser_simulations} simulations x {settings.loser_worlds} worlds", kl_min=settings.kl_min)),
            encoding="utf-8")
        run([*py, "report", "--selected", p.selected, "--forks", p.forks, "--runs", *runs, "--losers",
             p.judge / mining.LOSERS_RUN, "--settings", p.root / "settings.json", "--a-name", f"gen{gen}", "--b-name",
             champ.name, "--out", p.report], logs / "mine-report.log", False, env=pyenv)
        log(f"  mined targets: {json.dumps(kept)}; report {p.report}")
        return dict(kept=kept, targets=str(p.targets), heldout=str(p.heldout), report=str(p.report))

    def netduel():
        # The networks alone: no search, so tens of thousands of games an hour, by pair. A measurement beside the gate.
        name = seat_name(args.mode, None)
        result = duel("netduel", name, name, [model_dir, champ], args.net_duel_games, seed + 6_000_000, [], champ)
        if result is None:
            return
        log(f"  the networks alone: {duel_text(result)}")
        return dict(result, champion=champ.name, decks=dealt_text())

    def evaluate():
        # The permanent held-out set: value MSE and ECE over its games, policy CE and top-1 against its full searches'
        # targets, for the candidate and the champion on the same positions.
        networks = [f"gen{gen}={model_dir / 'train' / 'checkpoint.pt'}"]
        if (champ / "train" / "checkpoint.pt").exists() or args.dry_run:
            networks.append(f"{champ.name}={champ / 'train' / 'checkpoint.pt'}")
        out = gen_dir / "heldout-eval.json"
        run([sys.executable, "-m", "cantrip_ai.value_eval", "--data", held, "--every-game", "--policy", "--games",
             args.heldout_eval_games, "--device", args.device, "--checkpoint", *networks, "--out", out],
            gen_dir / "logs/evaluate.log", args.dry_run, env=dict(PYTHONPATH=str(REPO / "ai")), stream=show_progress)
        if args.dry_run:
            return
        results = json.loads(out.read_text(encoding="utf-8"))
        keys = ("value_mse", "ece", "sign", "policy_ce", "top1", "top1_played", "choices", "positions")
        return dict(directory=str(held), games=results[0]["games"], positions=results[0]["positions"],
                    networks={r["name"]: {k: r.get(k) for k in keys} for r in results})

    def gate():
        gate_sims = args.gate_simulations or args.simulations
        name = seat_name(args.mode, gate_sims) + args.seat_settings
        decks = dealt_text()
        if args.accept == "always" and not args.gate_measure:
            log("  not played: --accept always promotes with no gate (--gate-measure plays it, as a measurement)")
            return dict(skipped="--accept always", champion=champ.name, simulations=gate_sims, decks=decks)
        gate_log = gen_dir / "logs/gate.log"
        if not args.dry_run and gate_log.exists():
            # A gate that finished but that the driver never recorded (it stopped between BotLab exiting and the
            # stage being written) is final: read it back instead of rolling a fresh one. The per-network summary
            # names the champion it played, so a log from a different champion (or budget) is not reused.
            # The rule's stop line and the pairing mode must be this run's as well: a log of another rule or pairing is
            # another gate.
            text = gate_log.read_text(encoding="utf-8", errors="replace")
            lines = gate_stop_lines(args.accept, args.gate_early_stop, args.gate_stop_z, args.gate_stop_min_pairs,
                                    args.gate_stop_min_decisive)
            if gate_reusable(text, champ.name, name, lines, dealt_marker()):
                try:
                    recovered = parse_gate(text)
                except ValueError:
                    pass
                else:
                    log(f"[gate] reusing the finished gate in {gate_log} "
                        f"({recovered['wins']} to {recovered['losses']}, {recovered['verdict']})")
                    return dict(recovered, champion=champ.name, simulations=gate_sims, decks=decks)
        extra = []
        tested = ["--sprt-h0", args.gate_h0, "--sprt-h1", args.gate_h1, "--sprt-alpha", args.gate_alpha, "--sprt-beta", args.gate_beta]
        if args.accept == "sprt":
            extra = ["--sprt", *tested]
        elif args.accept == "sprt-paired":
            extra = ["--sprt-paired", *tested]
        elif args.accept == "above-half" and args.gate_early_stop:
            extra = ["--interval-stop", "--interval-z", args.gate_stop_z, "--interval-min-pairs", args.gate_stop_min_pairs,
                     "--interval-min-decisive", args.gate_stop_min_decisive]
        result = duel("gate", name, name, [model_dir, champ], args.gate_games, seed + 5_000_000, extra, champ)
        if result is None:
            return
        return dict(result, champion=champ.name, simulations=gate_sims, decks=decks, h0=args.gate_h0, h1=args.gate_h1,
                    measured_only=args.accept == "always")

    def promote():
        if args.dry_run:
            log(f"  accepted: copy {model_dir} to {root / 'champions' / f'gen{gen}'} and make it current; rejected: record it")
            return
        g = rec.data["stages"]["gate"]
        ok = accepted(g, args.accept)
        entry = dict(generation=gen, accepted=ok, rule=args.accept, verdict=g.get("verdict"), share=g.get("share"),
                     margin=g.get("margin"), llr=g.get("llr"), games=g.get("games"), paired_share=g.get("paired_share"),
                     paired_margin=g.get("paired_margin"), pentanomial=g.get("pentanomial"), paired_llr=g.get("paired_llr"),
                     paired_verdict=g.get("paired_verdict"), against=champ.name, simulations=args.gate_simulations or args.simulations,
                     mode=args.mode, at=now())
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

    def anchor_model(anchor: str) -> Path | None:
        """A ladder anchor's model directory: a champion's, a generation's model, or a path; None for a BotLab seat."""
        for d in (root / "champions" / anchor, root / "models" / anchor, Path(anchor)):
            if (d / "manifest.json").exists():
                return d
        return None

    def ladder():
        # The champion now (the candidate, if it was just promoted) against fixed anchors: a strength scale that does
        # not move with the champion, which candidate against champion alone cannot give.
        now_champ = (champion(root) if not args.dry_run else None) or champ
        _, play = ladder_due(gen, args.ladder_every, args.ladder_play_every)
        budgets = [args.ladder_budget or args.simulations] + ([args.ladder_play_budget] if play else [])
        rows = []
        for k, anchor in enumerate(x.strip() for x in args.ladder_anchors.split(",") if x.strip()):
            model = anchor_model(anchor)
            if model is not None and (model.name == now_champ.name or model.resolve() == now_champ.resolve()):
                log(f"  {anchor} is the champion: no ladder duel against itself")
                continue
            for j, budget in enumerate(budgets):
                a_name = seat_name(args.mode, budget) + args.seat_settings
                b_name = a_name if model is not None else anchor
                label = re.sub(r"[^\w.-]", "_", f"ladder-{anchor}-{budget}")
                result = duel(label, a_name, b_name, [now_champ] + ([model] if model is not None else []), args.ladder_games,
                              seed + 7_000_000 + 1000 * k + j, [], model)
                if result is None:
                    continue
                keys = ("wins", "losses", "draws", "games", "share", "margin", "pentanomial", "pairs", "split",
                        "paired_share", "paired_margin")
                rows.append(dict(generation=gen, champion=now_champ.name, anchor=anchor, budget=f"@{budget}",
                                 mode=args.mode, decks=dealt_text(), at=now(), **{x: result.get(x) for x in keys}))
                log(f"  {now_champ.name} against {anchor} at @{budget}: {duel_text(result)}")
        if args.dry_run:
            return
        ladder_record(root, rows, gen)
        return dict(rows=rows)

    def retire():
        doomed = retention.retire(root, args.keep, dry_run=args.dry_run, log=log)
        return dict(deleted=len(doomed), keep=args.keep)

    # The optional stages: the held-out set is chosen once, the network duel and the ladder run when asked for, and the
    # held-out set is scored whenever there is one.
    choose = args.heldout_games > 0 and not rec.done("heldout") and not heldout_ready(held) and not (held / HELDOUT_RECORD).exists()
    if args.heldout_games > 0 and not choose and not rec.done("heldout"):
        log(f"held-out set: {held} is chosen already, and kept")
    enabled = dict(
        build=not args.no_build,
        heldout=choose or rec.done("heldout"),
        netduel=args.net_duel_games > 0 or rec.done("netduel"),
        evaluate=not args.no_heldout_eval and (heldout_ready(held) or choose or rec.done("evaluate")),
        ladder=ladder_due(gen, args.ladder_every, args.ladder_play_every)[0] or rec.done("ladder"),
        # Pair mining needs a gate played in pairs whose games were kept (`--results`).
        mine=(not args.no_mine and args.accept != "always") or rec.done("mine"),
        mined=(not args.no_mine and args.accept != "always") or rec.done("mined"),
    )
    fns = (build, selfplay, heldout, train, export, parity, gate, promote, mine, netduel, evaluate, ladder, mined, retire)
    for name, fn in zip(STAGES, fns):
        if enabled.get(name, True):
            stage(name, fn)
        if name == args.stop_after:
            log(f"stopped after {name}, as asked")
            return
    if not args.dry_run:
        rec.data["finished"] = now()
        rec.save()
        log(readme(rec.data))


if __name__ == "__main__":
    main()
