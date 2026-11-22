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
measurement that fails is recorded as failed, in `generation.json` and the README's WARNING block, told, and the
generation goes on; the pair miner's stages are not measurements - they make training data, and one that fails stops
the generation unless `--allow-data-failure`: `DATA_STAGES`):
the permanent held-out set
(`--heldout-games N` chooses it once, from a generation's self-play before its training, moved to `ROOT/heldout`, which
no window holds; stage `evaluate` then scores the candidate and the champion on it every generation: value MSE and
ECE, policy CE and top-1 against its full searches' targets, the two paired by game, and each network's mean value by
the seat's deck against the seat's mean outcome); and the anchor ladder (`--ladder-every N`: a new champion against
`--ladder-anchors`, gen0 and the tactical bot, at the budget, after every N-th promotion, and every
`--ladder-play-every`-th of those also at the play budget, in `ROOT/ladder.json` and `ladder.md`, a generation's rows
replaced when it runs again; a generation that promoted nothing plays no ladder: its champion was measured when it was
promoted). The gate and every ladder duel are also read by matchup (`reports.matchups`, from the duel's results file):
the paired share in each pairing of decks, the mirrors and the rest, and a matchup far below a half is flagged at the
top of the README, as a seat a network misvalues is (`reports.py`). The fixed
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

Distributed self-play (docs/ai-training.md, "Distributed self-play"): `--workers REQUEST` plays the self-play stage on
this machine and on the machines the request leases from the worker manager (docs/workers.md; `distributed.py`), held
for each distributed stage and let go after it: each game index played exactly once across
them, handed out on demand, the shards pulled back into `genN/selfplay` as they close, checked, and recorded with the
host that played them (`genN/selfplay/HOSTS.json`). Off by default, and with it off nothing changes.
With `--workers` the duels are distributed too (docs/ai-training.md, "Distributed duels"; `duels.py`): the gate, the
ladder and the pair miner's replays, each game played on one host, the results merged and summarised
by BotLab as one machine's duel (`--no-dist-duels` keeps them here, `--no-duel-local` leaves this machine out).
`generation.py --root ROOT --stop` (`--stop-now`: the games in flight too) writes `selfplay.stop` in the current
generation's self-play directory and `genN/dist/stop`: self-play stops gracefully as at Ctrl-C (exit 3), and a
distributed driver (self-play's, a duel's, the judgements') stops its hosts the same way. The driver removes the files
when it runs the stage again.

The deep corpus (docs/ai-training.md, "The deep corpus"; `reanalyse.py`): with `--reanalyse-positions` or
`--reanalyse-hours`, stage `reanalyse` (after `mine`; a data stage, and no part of a retrain) draws positions at
random from the generation's recorded games - its self-play, played with `--selfplay-traces`, else the gate's games
the miner recorded - and has the champion the gate leaves search each once at `--reanalyse-simulations` over
`--reanalyse-worlds` (131,072 over 16), across the workers and (unless `--no-judge-local`) this machine, each host at
the lanes a probe of its memory and throughput found, until the positions or the hours are spent. The searches are
kept as policy targets in `ROOT/corpus/deep/genN`, which retention never prunes; `--corpus-train` adds the whole
corpus to every generation's training, weighted by the searches' effort (`--corpus-weight`, `--corpus-decay`), and
the evaluate stage scores the networks' policy against the corpus's held-out slices.

While it runs, the driver keeps `genN/status.json` (`status.py`: the stage, its pid, a progress counter read from the
ledgers, the shards and the logs), tells what happens (`notify.py`: a failure, a flagged warning, the verdict, the
end), runs every stage with the engine the generation pinned when it began (`engine.py`, `ROOT/builds/<id>`), checks
before it plays anything that it can (`preflight`: disk, memory, the workers, the champion's parity; `--preflight`
does only that and plans), and serves with the first serving option that passes parity for every model a server
holds (`serving_check`, `--parity-fallback`). `loop.py` runs generations one after another over all of this.
docs/ai-training.md: "Watching a run", "The loop", "Preflight", "The engine a generation runs", "Parity, and what a
generation serves with".

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
import threading
import time
from pathlib import Path

from . import distributed, duels, engine, mining, notify, reanalyse, reports, retention, status

REPO = Path(__file__).resolve().parents[2]
# heldout, evaluate and ladder run only when asked for (or, evaluate, when a held-out set exists; the ladder, when the
# generation promoted). The measurements come after the promotion, and a failure of one is recorded and passed over
# (`MEASUREMENTS`); the pair miner's two stop the generation when they fail (`DATA_STAGES`).
STAGES = ("build", "selfplay", "heldout", "train", "export", "parity", "gate", "promote", "mine", "reanalyse", "evaluate",
          "ladder", "mined", "retire")
# The stages a retrain runs again (all but the build, the self-play and the held-out set's choice).
LATER = ("train", "export", "parity", "gate", "promote", "mine", "evaluate", "ladder", "mined", "retire")
# Stages the driver ran once and no longer does, which an older generation's record may hold: the network-only duel
# (until 2026-11-21; by hand now, docs/ai-training.md "A network-only duel"). The README still shows one recorded, and
# a retrain sets it aside with the rest.
RETIRED = ("netduel",)
# The stages that decide nothing and make nothing a later stage reads: a failure is recorded, flagged at the top of
# the README, told (`notify`), and the generation goes on.
MEASUREMENTS = ("evaluate", "ladder")
# The stages whose output is training data: the pair miner's two (`mining.py`) make the next generations' targets,
# and the deep reanalysis (`reanalyse.py`) this generation's part of the corpus every later one trains on. A
# failure stops the generation, loudly, so that no generation trains without them unnoticed; `--allow-data-failure`
# records it as a measurement's is and goes on (the README, and the next generation's, then say what is missing).
# The reanalysis is no part of a retrain (`LATER`): a generation's deep targets are made once and kept.
MINE_STAGES = ("mine", "mined")
DATA_STAGES = MINE_STAGES + ("reanalyse",)


def data_lost(name: str, gen) -> str:
    """What the generations after `gen` go without when its data stage `name` fails."""
    return (f"gen{gen}'s mined targets" if name in MINE_STAGES else
            f"gen{gen}'s deep targets (its part of the corpus, `{reanalyse.CORPUS.as_posix()}`)")
# The serving options the parity check falls back through, in order, after the configured ones (docs/ai-training.md,
# "Parity, and what a generation serves with"): each replaces the configured precision flags. Measured on gen14 (an
# RTX 4080, T64): the default fp16 graph 32.3k rows/s, residual with TF32 inputs 34.7k, attention with TF32 inputs
# 26.2k, products 21.3k, fp32 15.1k. serve.py has only its precisions.
NATIVE_FALLBACKS = ("--precision fp16 --fp16-graph residual --inputs-tf32", "--precision fp16 --fp16-graph attention --inputs-tf32",
                    "--precision fp16 --fp16-graph products", "--precision fp32")
PYTHON_FALLBACKS = ("--precision fp32",)
# The flags of a server's arguments that a serving option replaces (each with the values it takes).
PRECISION_FLAGS = {"--precision": 1, "--fp16-graph": 1, "--inputs-tf32": 0, "--unfused-attention": 0}
# What training's data loader says when its workers do not fit in memory (each attaches the window): the one failure
# of training's that is tried again, with fewer workers (`--train-oom-retries`). A GPU out of memory is not this.
LOADER_OOM = re.compile(r"\bMemoryError|DataLoader worker \(pid|WinError 1455|WinError 1450|paging file is too small|"
                        r"Cannot allocate memory|not enough memory|Unable to allocate|Couldn't open shared file mapping")
# train.py's own `--workers` default, when `--train-args` does not name one.
TRAIN_WORKERS = 2
# What a server or the parity test says when a model served is outside the tolerance (`status.PARITY_FAILED`, which
# the supervisor's reading of a failure shares): only this makes the parity check fall back to the next serving
# option; a server that did not start for any other reason (the GPU out of memory, a library missing, a timeout) is
# that failure, and no slower option's.
PARITY_FAILED = status.PARITY_FAILED


def loader_workers(words: list[str]) -> int:
    """train.py's `--workers` as `--train-args` names it (`--workers N` or `--workers=N`, the last one), else its
    default."""
    found = TRAIN_WORKERS
    for k, word in enumerate(words):
        if word == "--workers" and k + 1 < len(words) and words[k + 1].isdigit():
            found = int(words[k + 1])
        elif word.startswith("--workers=") and word.split("=", 1)[1].isdigit():
            found = int(word.split("=", 1)[1])
    return found
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
# BotLab published self-contained, one file, for a worker's duels (`duels.py`): by the build stage, for each kind of
# worker the request leases.
BOTLAB_PUBLISH = REPO / "dotnet/tools/Cantrip.BotLab/bin/publish"
BOTLAB_RIDS = {"windows": ("win-x64", "cantrip-botlab.exe"), "linux": ("linux-x64", "cantrip-botlab")}
# BotLab's sequential stops: a distributed duel's are the driver's (`duels.sequential_stop`), not each host's.
STOP_FLAGS = ("--sprt", "--sprt-paired", "--interval-stop")
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


def ladder_due(promotions: int, every: int, play_every: int) -> tuple[bool, bool]:
    """Whether the generation that made the `promotions`-th champion runs the ladder (every `every`-th promotion), and
    whether also at the play budget (every `play_every`-th of those). A generation that promoted nothing has no ladder
    (`promotions` 0): its champion is the one an earlier ladder measured."""
    due = every > 0 and promotions > 0 and promotions % every == 0
    return due, due and play_every > 0 and (promotions // every) % play_every == 0


def promotions(root: Path, gen: int) -> int:
    """How many generations up to `gen` promoted their candidate (`champions/history.json`; a generation retrained is
    one generation)."""
    path = root / "champions" / "history.json"
    rows = json.loads(path.read_text(encoding="utf-8")) if path.exists() else []

    def number(g) -> int | None:
        # A history row's generation is a number, or a labelled variant of one ("5-mined", "6-v4"): that generation.
        m = re.match(r"\d+", str(g)) if g is not None else None
        return int(m.group()) if m else None

    return len({n for r in rows if r.get("accepted") and (n := number(r.get("generation"))) is not None and n <= gen})


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
        self.engine: str | None = None  # the pinned engine's id, written into every stage's record
        self.path = gen_dir / "generation.json"
        self.data = json.loads(self.path.read_text(encoding="utf-8")) if self.path.exists() else dict(generation=gen, started=now(), stages={})

    def done(self, stage: str) -> bool:
        return self.data["stages"].get(stage, {}).get("done", False)

    def begin(self, stage: str):
        """A stage is begun (again, when an earlier run of the driver did not finish it): counted, so its record can
        say whether its seconds are a whole run's (`runs`; `status.expected` goes by whole runs only)."""
        begun = self.data.setdefault("begun", {})
        begun[stage] = begun.get(stage, 0) + 1
        self.save()

    def put(self, stage: str, seconds: float, **numbers):
        self.data["stages"][stage] = dict(done=True, seconds=round(seconds, 1), finished=now(),
                                          runs=self.data.get("begun", {}).get(stage, 1),
                                          **({"engine": self.engine} if self.engine else {}), **numbers)
        self.save()

    def fail(self, stage: str, seconds: float, why: str):
        """A stage that failed and was passed over (a measurement, or a data stage allowed to): recorded, not done (a
        resumed generation tries it again)."""
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


def duels_by_matchup(d: dict) -> list[tuple[str, dict]]:
    """The duels of a generation's record that were read by matchup (`reports.matchups`), each with its name."""
    s = d["stages"]
    found = [(name, s[name].get("matchups")) for name in ("gate", *RETIRED) if name in s]
    found += [(f"ladder against {x['anchor']} at {x['budget']}", x.get("matchups"))
              for x in s.get("ladder", {}).get("rows", [])]
    return [(name, m) for name, m in found if m]


def warnings(d: dict) -> list[str]:
    """Everything a generation's record flags, a line each: the README's WARNING block, and what is told (`notify`) as
    it appears. The reports' flags (a matchup far below a half, a seat misvalued: `reports.py`); a stage that failed
    and was passed over; mined targets an earlier generation should have made and training went without; a serving
    option the parity check fell back to; what the preflight doubted."""
    s = d["stages"]
    flags = [f"{name}, {f}" for name, m in duels_by_matchup(d) for f in reports.matchup_flags(m)]
    flags += reports.seat_flags(s.get("evaluate", {}).get("seats") or [])
    for name in DATA_STAGES + MEASUREMENTS:
        if s.get(name, {}).get("failed"):
            why = " ".join(str(s[name]["failed"]).split())
            lost = "mined targets" if name in MINE_STAGES else "deep targets"
            flags.append(f"stage {name} failed and was passed over"
                         + (f", so the next generations train WITHOUT this generation's {lost}" if name in DATA_STAGES else "")
                         + f": {why[:200]}")
    flags += list(d.get("missing_mined") or [])
    flags += list(d.get("corpus_left_out") or [])
    # What a stage flagged while it ran (self-play: a worker's traces that did not come home).
    flags += list(d.get("flagged") or [])
    if (c := s.get("evaluate", {}).get("corpus_failed")):
        flags.append(f"evaluate: the deep corpus's held-out slice was not scored: {' '.join(str(c).split())[:200]}")
    r = s.get("reanalyse") or {}
    if r.get("done") and r.get("searched") and (r.get("memory_bound") or 0) > 0.05 * r["searched"]:
        flags.append(f"reanalyse: {r['memory_bound']} of {r['searched']} deep searches stopped at their memory bound, "
                     "their budget not spent (raise --reanalyse-memory-mb)")
    if r.get("done") and r.get("mismatched"):
        flags.append(f"reanalyse: {r['mismatched']} positions were stood in as another position than the recording's "
                     "and gave no target (the engine that judged was not the recordings')")
    for when, pick in sorted((d.get("serving") or {}).items()):
        if pick.get("index"):
            failed = "; ".join(f"`{t['args'] or '(the default)'}`: {t['failed']}" for t in pick.get("tried", []))
            flags.append(f"parity ({when}): serving {', '.join(pick.get('models', []))} with `{pick['args']}` instead of the "
                         f"configured flags ({failed[:300]})")
    flags += [f"preflight: {w}" for w in (d.get("preflight") or {}).get("warnings", []) if w not in flags]
    return flags


def readme(d: dict) -> str:
    s = d["stages"]
    gen = d["generation"]
    lines = [f"# Generation {gen}", "", f"Expert iteration (docs/ai-player-plan.md, \"Phase 4\"), {d.get('mode', 'full')} mode throughout. "
             f"Started {d['started']}; champion at the start: {d.get('champion', '?')}. Written by `ai/cantrip_ai/generation.py` "
             "from `generation.json`.", ""]
    # What a total hides, first: a matchup of a duel far below a half, a seat a network misvalues (`reports.py`), a
    # stage that failed and was passed over, training data that is missing, a serving option fallen back to.
    duels_read = duels_by_matchup(d)
    seats = s.get("evaluate", {}).get("seats") or []
    flags = warnings(d)
    if flags:
        lines += [f"> **WARNING: {len(flags)} flagged** (a matchup more than {reports.MATCHUP_Z:g} standard errors below "
                  f"a half; a seat whose mean value is more than {reports.SEAT_GAP:g} from its mean outcome; a stage "
                  "that failed; training data missing). The tables are below.", *[f"> - **{f}**" for f in flags], ""]
    lines += ["| stage | time | numbers |", "|---|---|---|"]

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
    for name in DATA_STAGES + MEASUREMENTS + RETIRED:
        if s.get(name, {}).get("failed"):
            why = " ".join(str(s[name]["failed"]).split())
            kind = "its output is training data: the next generations train without it" if name in DATA_STAGES else "a measurement"
            row(name, f"failed, and passed over ({kind}): {why[:300]}")
    if s.get("mine", {}).get("done"):
        r = s["mine"]
        row("mine", f"the gate's pairs mined: {r.get('decisive')} decisive and {r.get('control')} control pairs, "
                    f"{r.get('replayed')} games replayed ({r.get('same')} the gate's"
                    + (f", {r['dropped_pairs']} pair(s) left out" if r.get("dropped_pairs") else "") + "), "
                    f"{(r.get('planned') or {}).get('forks')} forks and {(r.get('planned') or {}).get('losers')} losers' "
                    f"decisions judged by {r.get('judge')} on {r.get('judged_on')}")
    if s.get("mined", {}).get("done"):
        r = s["mined"]
        row("mined", f"mined targets kept: {json.dumps(r.get('kept'))} (`mine/targets`, the next generations' training; "
                     "`mine/REPORT.md`)")
    if s.get("reanalyse", {}).get("done"):
        r = s["reanalyse"]
        each = r.get("seconds_a_position")
        row("reanalyse", f"deep reanalysis: {r.get('searched')} of {r.get('planned')} positions of {r.get('source')} games "
                         f"searched by {r.get('judge')} at {r.get('simulations')} simulations over {r.get('worlds')} worlds"
                         + (f" (seeds {r.get('seeds')})" if len(r.get("seeds") or []) > 1 else "")
                         + (f", {each:.1f} s a position" if each else "")
                         + (f"; stopped: {r['closed']}" if r.get("closed") else "")
                         + f"; the deep choice is not the network's in {r.get('disagree')}"
                         + (f", KL median {_f(r.get('kl_median'))}" if r.get("kl_median") is not None else "")
                         + (f"; {r['memory_bound']} stopped at their memory bound" if r.get("memory_bound") else "")
                         + f"; {r.get('positions')} targets kept in `{r.get('corpus')}` ({json.dumps(r.get('directories'))})")
    if "netduel" in s and not s["netduel"].get("failed"):
        r = s["netduel"]
        row("netduel", f"the networks alone (no search), candidate gen{gen} against {r['champion']}, {r['decks']}: "
                       f"{duel_text(r)}; a measurement, not the gate")
    if "evaluate" in s and not s["evaluate"].get("failed"):
        r = s["evaluate"]
        parts = [f"{n}: value MSE {_f(e.get('value_mse'), '{:.4f}')}, ECE {_f(e.get('ece'), '{:.4f}')}, "
                 f"policy CE {_f(e.get('policy_ce'), '{:.4f}')}, top-1 {_f(e.get('top1'))} on {e.get('choices', '-')} targets"
                 for n, e in (r.get("networks") or {}).items()]
        if text := reports.pairs_text(r.get("paired")):
            parts.append(text)
        text = ""
        if r.get("networks"):
            text = (f"the permanent held-out set ({r['games']} games, {r['positions']} positions; full-search policy "
                    f"targets): " + "; ".join(parts) + ("; by seat below" if seats else ""))
        if c := r.get("corpus"):
            deep = [f"{n}: policy CE {_f(e.get('policy_ce'), '{:.4f}')}, top-1 {_f(e.get('top1'))}" for n, e in c["networks"].items()]
            if pc := (c.get("paired") or {}).get("policy_ce"):
                deep.append(f"{c['paired']['of']} less {c['paired']['against']}, paired by game: policy CE "
                            f"{pc['diff']:+.4f} +/- {pc['half']:.4f}")
            text += ("; " if text else "") + (f"the deep corpus's held-out slice ({c.get('choices')} deep targets of "
                                             f"{c.get('games')} games, {len(c.get('directories') or [])} directories): "
                                             + "; ".join(deep))
        row("evaluate", text)
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
    for name, m in duels_read:
        lines += [f"## {name} by matchup", "", "The first seat's paired share (the gate's: the candidate's) in each "
                  "pairing of decks, whichever seat holds which; from the duel's results file.", "",
                  *reports.matchup_table(m), ""]
    if seats:
        lines += ["## The held-out set by seat", "", "Each network's mean value over the positions a deck's seat answered, "
                  "against how those games ended for it (both from -1, lost, to 1, won).", "", *reports.seat_table(seats), ""]
    return "\n".join(lines)


# --- Running ------------------------------------------------------------------------------------------------------


def retrain(rec: Record, gen_dir: Path, model_dir: Path, args, log=print):
    """Set a rejected generation's attempt aside, from its training on, so the stages run again."""
    s = rec.data["stages"]
    if not s.get("promote", {}).get("done") or s["promote"].get("accepted"):
        raise SystemExit(f"--retrain: gen{rec.gen} has no rejected attempt to set aside")
    later = LATER + RETIRED
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
    # The later stages' distributed ledgers (the gate's, the miner's replays and judgements, the ladder's: each its own
    # directory under dist/; self-play's ledger, grants and logs are dist/'s own files) go with the attempt, or the next
    # attempt's gate finds the last one's games under other settings and refuses to go on.
    dist = gen_dir / "dist"
    if dist.is_dir():
        for d in sorted(dist.iterdir()):
            if d.is_dir() and d.name not in ("incoming", "logs") and not d.name.startswith("attempt"):
                (dist / f"attempt{k}").mkdir(exist_ok=True)
                d.rename(dist / f"attempt{k}" / d.name)
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


def fatal_watch(p, log_path: Path, pattern, grace: float, log=print, every: float = 5.0):
    """Watches a running process's log for a failure it may not die of (`pattern`: training's data loader out of
    memory leaves the trainer waiting on workers that are gone - gen12 of the templates line waited 1.7 hours): once
    the log says it and the process is still there `grace` seconds later, the process and everything it started is
    killed, and the stage fails with what the log holds. Returns the event that ends the watch."""
    done = threading.Event()

    def look():
        seen = None
        while not done.wait(every):
            if p.poll() is not None:
                return
            if seen is None:
                if pattern.search("\n".join(status.tail(log_path, 200))):
                    seen = time.time()
            elif time.time() - seen >= grace:
                log(f"  {log_path.name} says a failure the process did not exit on; it has hung for {grace:.0f} s: killing it")
                distributed.kill_tree(p)
                return

    threading.Thread(target=look, name="fatal-watch", daemon=True).start()
    return done


def run(cmd: list[str], log_path: Path, dry_run: bool, env: dict | None = None, log=print, stream=False,
        stopped_code: int | None = None, fatal=None, fatal_grace: float = 60.0) -> str:
    """Runs `cmd` with its output in `log_path` (and here, with `stream`); a failure ends the driver. With
    `stopped_code`, the process stops gracefully on Ctrl-C (it hears it too, from the console): the driver keeps waiting
    for it, and that exit code raises `Stopped` instead of a failure. With `fatal` (a pattern), a process whose log
    says it and that has not exited `fatal_grace` seconds later is killed (`fatal_watch`)."""
    shown = " ".join(str(c) for c in cmd)
    if len(shown) > 2000:
        shown = f"{shown[:2000]} ... ({len(cmd)} arguments)"
    log(f"$ {shown}")
    if dry_run:
        return ""
    log_path.parent.mkdir(parents=True, exist_ok=True)
    environment = {**os.environ, **(env or {})}
    interrupts = 0
    watch = (lambda p: fatal_watch(p, log_path, fatal, fatal_grace, log)) if fatal is not None else (lambda p: threading.Event())

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
            watching = watch(p)
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
            watching.set()
            p.stdout.close()
    elif stopped_code is not None or fatal is not None:
        with log_path.open("w") as f:
            p = subprocess.Popen([str(c) for c in cmd], cwd=REPO, env=environment, stdout=f, stderr=subprocess.STDOUT)
            watching = watch(p)
            code = waiting(p, p.wait)
            watching.set()
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
                 native: bool = False, ort_dylib: str | None = None, infer: Path | None = None):
        self.models, self.log_path, self.device, self.extra, self.dry_run, self.log = models, log_path, device, extra, dry_run, log
        self.native, self.ort_dylib = native, ort_dylib
        self.infer = infer or INFER  # cantrip-infer: the generation's pinned build's (`engine.Engine`)
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
        return [self.infer, "serve", *models, "--port", 0, "--ready-file", ready, "--device", device, *self.extra], env

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
    its self-play reads each tick (and a distributed driver passes on to its workers), and `genN/dist/stop`, which a
    distributed driver - self-play's, a duel's, a judgement's - reads. Only the running stage's: where the status file
    says a driver is at another stage than self-play, no stop is left in the self-play directory (nothing there
    would read it); with no status to go by, both."""
    gen = gen or next_generation(root)
    running = status.read(root, gen) or {}
    stage = running.get("stage") if running.get("state") == "running" else None
    whole = root / f"gen{gen}" / "dist" / "stop"
    path = whole
    if stage in (None, "selfplay"):
        out = root / f"gen{gen}" / "selfplay"
        out.mkdir(parents=True, exist_ok=True)
        path = out / distributed.STOP_FILE
        path.write_text("now\n" if now else "", encoding="utf-8")
    # The whole distributed run's, whichever stage's it is (`duels.stop_file`).
    whole.parent.mkdir(parents=True, exist_ok=True)
    whole.write_text("now\n" if now else "", encoding="utf-8")
    log(f"asked gen{gen}'s {stage or 'self-play, duels and judgements'} to stop"
        f"{' now (its games in flight too)' if now else ''}: {path}")
    return path


def with_precision(configured: str, option: str) -> str:
    """A server's arguments with their precision flags (`PRECISION_FLAGS`) replaced by a serving option's: its batch
    sizes, pipelines and the rest stay."""
    words, kept, k = configured.split(), [], 0
    while k < len(words):
        if words[k] in PRECISION_FLAGS:
            k += 1 + PRECISION_FLAGS[words[k]]
        else:
            kept.append(words[k])
            k += 1
    return " ".join(kept + option.split())


def serving_options(configured: str, fallbacks) -> list[str]:
    """The serving options a parity check tries, in order: the configured arguments, then each fallback in their
    place (`with_precision`), none twice."""
    out = [" ".join(configured.split())]
    for option in fallbacks:
        made = with_precision(configured, option)
        if made not in out:
            out.append(made)
    return out


def available_memory() -> int | None:
    """Physical memory available now, in bytes; None where this cannot be asked."""
    if os.name == "nt":
        import ctypes  # noqa: PLC0415

        class Memory(ctypes.Structure):
            _fields_ = [("length", ctypes.c_ulong), ("load", ctypes.c_ulong), *[(n, ctypes.c_ulonglong) for n in (
                "total", "available", "total_page", "available_page", "total_virtual", "available_virtual", "extended")]]

        m = Memory()
        m.length = ctypes.sizeof(Memory)
        return int(m.available) if ctypes.windll.kernel32.GlobalMemoryStatusEx(ctypes.byref(m)) else None
    try:
        for line in Path("/proc/meminfo").read_text().splitlines():
            if line.startswith("MemAvailable:"):
                return int(line.split()[1]) * 1024
    except (OSError, ValueError):
        pass
    return None


def disk_free(path: Path) -> int:
    return shutil.disk_usage(path).free


def tree_bytes(path: Path) -> int:
    total = 0
    for base, _, files in os.walk(path):
        for name in files:
            try:
                total += os.path.getsize(os.path.join(base, name))
            except OSError:
                pass
    return total


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
    a.add_argument("--selfplay-scale-floor", type=int, metavar="N",
                   help="the lanes no fall in throughput backs self-play off below (cantrip-selfplay --scale-floor; "
                        "default half the most lanes it has held, at least 4). Memory still holds games back at any level")
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
    a.add_argument("--mine-floor", type=float, default=0.10,
                   help="the least share of the policy-target weight the mined targets make (they are scaled up to it); "
                        "each mined directory is weighted by its search's simulations over self-play's full search's")
    a.add_argument("--mine-lanes", type=int, default=24, help="lanes of each deep judgement run (with --workers, a worker's are --worker-judge-lanes)")
    a.add_argument("--mine-local-lanes", type=int, default=8,
                   help="lanes of each judgement run here, with no worker (one run at a time, each search up to 2 GB)")
    a.add_argument("--mine-control", type=int, default=200, help="split pairs mined as a control")
    a.add_argument("--mine-heldout", type=float, default=0.15, help="the share of mined pairs held out for measuring")
    a.add_argument("--mine-timeout", type=float, default=8.0,
                   help="unused: the judgements are made within the mine stage now (kept so older command lines run)")
    a.add_argument("--judge-local", action=argparse.BooleanOptionalAction, default=True,
                   help="with --workers: this machine judges its part of the pair miner's judgements and of the deep "
                        "reanalysis's searches")
    a.add_argument("--worker-judge-lanes", type=int,
                   help="a worker's judgement lanes, one run at a time (its own `judge_lanes` wins; default 4 x --mine-lanes, "
                        "the four runs at once on one worker before)")
    a.add_argument("--reanalyse-positions", type=int, default=0,
                   help="deep reanalysis (`reanalyse.py`; docs/ai-training.md, 'The deep corpus'): at most this many "
                        "positions of this generation's recorded games searched again, deeply, after the mine stage, "
                        "and kept for good in ROOT/corpus/deep/genN (0, with no --reanalyse-hours: no reanalysis)")
    a.add_argument("--reanalyse-hours", type=float, default=0.0, metavar="H",
                   help="the reanalysis's time budget: its judging stops when it is spent (the parts in flight finish), "
                        "and no more positions are planned than it buys; a deep search's cost varies a lot (0: none)")
    a.add_argument("--reanalyse-simulations", type=int, default=131072,
                   help="a deep search's simulations in all, shared by its worlds (self-play's full search: --full-simulations)")
    a.add_argument("--reanalyse-worlds", type=int, default=16, help="a deep search's worlds")
    a.add_argument("--reanalyse-memory-mb", type=int, default=0,
                   help="a deep search's memory bound (the search's MemoryMb; a search that reaches it stops short). "
                        f"0: {reanalyse.SIMULATION_BYTES >> 10} KB a simulation, the largest tree measured: 8192 MB at 131072")
    a.add_argument("--reanalyse-seeds", default="1",
                   help="the deep searches' seeds, by commas: each seed searches every position once, into a directory "
                        "of its own (one is enough: two seeds chose the same move on 91%% of positions)")
    a.add_argument("--reanalyse-model",
                   help="the network that searches: a champion or a model by name, or a model directory (default: the "
                        "champion the gate leaves)")
    a.add_argument("--reanalyse-source", choices=reanalyse.SOURCES, default="auto",
                   help="where the positions come from: this generation's self-play (played with --selfplay-traces), or "
                        "the gate's games the mine stage recorded; auto: self-play when it has traces, else the gate. "
                        "Both are this generation's engine's, the only one their games replay with")
    a.add_argument("--reanalyse-per-game", type=int, default=2, help="decisions drawn from a game")
    a.add_argument("--reanalyse-heldout", type=float, default=0.1,
                   help="the share of the reanalysed games held out of training: the evaluate stage scores the "
                        "networks' policy against their deep targets")
    a.add_argument("--reanalyse-search", action="append", default=[], metavar="KEY=VALUE",
                   help="another setting of the deep search (repeatable); RootShortcut=false and MemoryMb are set")
    a.add_argument("--reanalyse-batch", type=int, default=32, help="leaves a deep search sends to the network at once")
    a.add_argument("--reanalyse-lanes", type=int, default=0,
                   help="every host's lanes, with no probe (0: each host's own, from a probe of its memory and "
                        "throughput, made once a line and kept in ROOT/corpus/deep/probe.json)")
    a.add_argument("--reanalyse-max-lanes", type=int, default=0, help="the most lanes a worker's probe may find (0: what fits)")
    a.add_argument("--reanalyse-local-lanes", type=int, default=0, help="the most lanes this machine's probe may find")
    a.add_argument("--reanalyse-part-games", type=int, default=0,
                   help="the games a part judges (0: a host's lanes); a part is taken whole, so a run killed loses the "
                        "parts in flight, and a budget spent waits for them")
    a.add_argument("--reanalyse-reprobe", action="store_true", help="probe every host again this generation")
    a.add_argument("--corpus-train", action="store_true",
                   help="train on the whole deep corpus (every earlier generation's ROOT/corpus/deep/genN, whatever "
                        "its age), each directory weighted by its search's simulations over self-play's full search's")
    a.add_argument("--corpus-weight", type=float, default=1.0, help="with --corpus-train: the corpus's weight, times the effort")
    a.add_argument("--corpus-decay", type=float, default=0.85,
                   help="with --corpus-train: a corpus generation's weight is multiplied by this for each generation "
                        "of its age (1: never decays)")
    a.add_argument("--corpus-share", type=float, default=0.25,
                   help="with --corpus-train: the most of the policy-target weight the corpus makes beside the "
                        "window's self-play (policy targets counted by decision on both sides, each times its "
                        "weight): past it every corpus weight is scaled down alike (0: no cap)")
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
    a.add_argument("--heldout-dir", help="the permanent held-out set of self-play games, never trained on (default "
                                         "ROOT/heldout); every window leaves it out")
    a.add_argument("--heldout-games", type=int, default=0,
                   help="choose the permanent held-out set now, once: this many games' whole shards moved from this "
                        "generation's self-play into --heldout-dir before its training (0: not; a set already chosen "
                        "is kept)")
    a.add_argument("--heldout-eval-games", type=int, default=1000,
                   help="games of the held-out set the evaluate stage scores (value MSE and ECE, policy CE and top-1 "
                        "against the full searches' targets, the value by the seat's deck), the candidate and the champion")
    a.add_argument("--no-heldout-eval", action="store_true", help="skip the evaluate stage even when a held-out set exists")
    a.add_argument("--ladder-every", type=int, default=0,
                   help="after every N-th promotion (1: every new champion), the new champion against the fixed "
                        "--ladder-anchors, recorded in ROOT/ladder.json and ladder.md; a generation that promoted "
                        "nothing plays none (0: off)")
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
    a.add_argument("--workers", metavar="REQUEST",
                   help="distributed self-play: the machines to lease from the worker manager for each distributed stage "
                        "(docs/workers.md: `all`, names, or what a machine has to have; docs/ai-training.md, 'Distributed "
                        "self-play'); this machine plays its share too unless --no-selfplay-local")
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
    a.add_argument("--dist-duels", action=argparse.BooleanOptionalAction, default=True,
                   help="with --workers: the gate, the network duel, the ladder and the pair miner's replays played on "
                        "this machine and the workers too (docs/ai-training.md, \"Distributed duels\")")
    a.add_argument("--duel-local", action=argparse.BooleanOptionalAction, default=True,
                   help="with --workers: this machine plays its part of each distributed duel")
    a.add_argument("--duel-server-args", default=None,
                   help="every host's cantrip-infer in a distributed duel, one string (default: --server-args with "
                        "--server-impl native, else cantrip-infer's own defaults, fp32); --shapes is refused")
    a.add_argument("--worker-duel-parallel", type=int, default=32,
                   help="a worker's duel games at once (its own `duel_parallel` wins; on Linux, pinned, 1.7 a core)")
    a.add_argument("--abandon-workers", action="store_true",
                   help="finish a distributed generation's self-play on this machine alone, without its ledger")
    a.add_argument("--stop", action="store_true",
                   help="ask the generation's running self-play to stop gracefully (its stop file), and exit")
    a.add_argument("--stop-now", action="store_true", help="as --stop, and stop the games in flight too")
    a.add_argument("--dry-run", action="store_true", help="print the plan and run nothing")
    a.add_argument("--tiny", action="store_true", help="a toy scale, for testing the driver")
    a.add_argument("--allow-data-failure", action="store_true",
                   help="a data-producing stage that fails (mine, mined: the next generations' mined targets; "
                        "reanalyse: this generation's deep targets) is recorded "
                        "and passed over as a measurement is, and a generation trains although an earlier one's mined "
                        "targets are missing; without it either stops the generation")
    a.add_argument("--repin-engine", action="store_true",
                   help="a generation resumed on another engine source than it began with goes on with the working "
                        "tree's (docs/ai-training.md, 'The engine a generation runs'); its gate must then be played "
                        "again before its pairs are mined")
    a.add_argument("--preflight", action="store_true",
                   help="build, make every preflight check for real (disk and memory, the workers reached, the "
                        "champion's parity with the serving options), print the plan as --dry-run does, and run "
                        "nothing else: what the loop's supervisor runs before each generation")
    a.add_argument("--no-preflight", action="store_true", help="skip the checks at the generation's start")
    a.add_argument("--require-workers", action="store_true",
                   help="preflight: a worker --workers names that cannot be leased or reached fails it (default: a "
                        "warning - the stages are played without it)")
    a.add_argument("--min-free-disk", type=float, default=5.0, metavar="GB",
                   help="preflight: the least free space under --root (and at least 1.2 times the last generation's size)")
    a.add_argument("--min-free-memory", type=float, default=4.0, metavar="GB",
                   help="preflight, before training: the least available memory (and at least the window's size on disk)")
    a.add_argument("--parity-fallback", action="append", metavar="ARGS",
                   help="a serving option the parity check falls back to when the one before it fails for a model the "
                        "server holds (repeatable, in order; each replaces --server-args' precision flags). Default: "
                        + "; ".join(f"'{o}'" for o in NATIVE_FALLBACKS) + f" (serve.py: '{PYTHON_FALLBACKS[0]}')")
    a.add_argument("--no-parity-fallback", action="store_true", help="only the configured serving flags: parity fails with them or passes")
    a.add_argument("--train-oom-retries", type=int, default=2,
                   help="times training is run again with half its data-loader workers after they ran out of memory")
    args = a.parse_args(argv)
    # `--preflight` plans as a dry run does - nothing of the generation is run, recorded or bootstrapped - from here
    # on; only its build and its checks are real (`build_dry`, `preflight(real=True)`).
    if args.preflight:
        args.dry_run = True
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
        # A toy scale; the games and the gate's games are the command line's when it names them (a test that mines
        # needs a few decisive pairs).
        args.games = 4 if args.games == a.get_default("games") else args.games
        args.gate_games = 4 if args.gate_games == a.get_default("gate_games") else args.gate_games
        args.simulations, args.train_steps, args.warmup = 8, 4, 1
        args.accept = "ahead" if args.accept in ("sprt", "sprt-paired") else args.accept
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
    if not args.dry_run:
        # One driver a generation: beside a live one this one would write over its status and its record, and start
        # its stages a second time. Refused before anything is touched.
        running = status.read(root, gen)
        if (running and running.get("state") == "running" and running.get("pid") != os.getpid()
                and running.get("host") == status.socket.gethostname() and status.alive(running)):
            raise SystemExit(f"gen{gen} is being run by a driver already (pid {running.get('pid')}, in stage "
                             f"{running.get('stage')}: {gen_dir / status.STATUS}); this one does not start beside it. "
                             "`python -m cantrip_ai.status` shows it; `--stop` asks it to stop")
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
    if args.tiny:
        args.min_free_disk, args.min_free_memory = min(args.min_free_disk, 1.0), min(args.min_free_memory, 1.0)
    # `--preflight`'s build and checks are real, with their logs under ROOT/preflight.
    pre_dir = root / "preflight" if args.preflight else gen_dir
    build_dry = args.dry_run and not args.preflight

    # The engine this generation runs (`engine.py`): named once, in its record, and kept whatever the tree does after.
    current = engine.identify(REPO)
    pinned = rec.data.get("engine")
    if pinned and pinned.get("id") and pinned.get("id") != current.get("id") and args.repin_engine:
        log(f"engine: --repin-engine: this generation began on {engine.describe(pinned)} and goes on with the working "
            f"tree's {engine.describe(current)}; what it recorded before replays no longer")
        pinned = None
    if not pinned or not pinned.get("id"):
        pinned = current
        rec.data["engine"] = current
    eng = engine.Engine(root, REPO, pinned, current, log)
    # Written into a stage's record only while a build of the engine is pinned (`Engine.kept`): with nothing pinned
    # (`--no-build` on an engine never built here) the stages run whatever the tree holds, and the id would vouch for
    # binaries nobody kept.
    rec.engine = eng.id if eng.kept() else None
    log(f"engine: {engine.describe(pinned)}"
        + ("" if eng.same() else f"; the working tree is now {engine.describe(current)}: this generation keeps its own "
                                 f"build ({eng.dir}) and nothing is built from the tree"))
    built_now: set[str] = set()  # what this run's build stage built (`build`): the only files `eng.adopt` keeps
    heart: list = []  # the status file's keeper, once the generation runs (`status.Heartbeat`)

    def tell(event: str, message: str, stage_name: str | None = None, **details):
        """A notification (`notify.py`); a dry run tells nobody."""
        if not args.dry_run:
            notify.send(event, message, root=root, generation=gen, stage=stage_name, log=log, **details)

    def tell_warnings():
        """Whatever the record flags now that was not told before (`warnings`): told once, and kept in the status."""
        if args.dry_run:
            return
        told = rec.data.setdefault("told", [])
        new = [f for f in warnings(rec.data) if f not in told]
        for f in new:
            told.append(f)
            for h in heart:
                h.warn(f)
            tell("warning", f)
        if new:
            rec.save()

    def flag(text: str):
        """Something a running stage flags (`warnings`: the README's WARNING block, the status and a notification),
        at once - not when the stage ends, hours on."""
        if args.dry_run:
            return
        flagged = rec.data.setdefault("flagged", [])
        if text not in flagged:
            flagged.append(text)
            rec.save()
        tell_warnings()

    def stage(name, fn):
        if rec.done(name) and name != "build":
            log(f"[{name}] done already ({rec.data['stages'][name]['seconds']:.0f} s)")
            return
        log(f"[{name}]")
        for h in heart:
            h.begin(name)
        if not args.dry_run:
            rec.begin(name)
        t = time.time()
        try:
            numbers = fn()
        except (Exception, SystemExit) as e:
            # A measurement decides nothing: its failure is recorded, flagged and told, and the generation goes on. A
            # data-producing stage's is passed over only when that was asked for: the next generations would train
            # without what it makes.
            passed_over = name in MEASUREMENTS or (name in DATA_STAGES and args.allow_data_failure)
            if status.asked_to_stop(str(e)):
                # Asked to stop (`--stop`, Ctrl-C) while the stage ran (the judgements', a duel's): no failure of
                # the stage and nothing to pass over - the driver's last word is "stopped", and a run goes on.
                raise
            if args.dry_run or not passed_over:
                if name in DATA_STAGES and not args.dry_run:
                    raise SystemExit(
                        f"stage {name} failed, and its output is training data: the generations after this one would "
                        f"train WITHOUT {data_lost(name, gen)}. The generation stops here; fix the cause and run the "
                        "driver again (the stage is tried again), or pass --allow-data-failure to record the failure "
                        f"and go on without them.\n{e}") from e
                raise
            why = str(e) or type(e).__name__
            log(f"[{name}] failed, recorded and passed over: {why.splitlines()[0] if why else ''}")
            rec.fail(name, time.time() - t, why[-2000:])
            tell("data-failed" if name in DATA_STAGES else "measurement-failed",
                 f"gen{gen}'s {name} failed and was passed over: {why.splitlines()[0][:300] if why else ''}", name,
                 error=why[-1500:])
            tell_warnings()
            return
        if not args.dry_run:
            rec.put(name, time.time() - t, **(numbers or {}))
            tell_warnings()
            if name == "promote":
                tell("verdict", f"gen{gen} " + ("accepted: it is the champion" if (numbers or {}).get("accepted") else
                                                 f"rejected: {(numbers or {}).get('champion')} stays") + f" ({gate_line()})", name,
                     accepted=(numbers or {}).get("accepted"), gate={k: rec.data["stages"].get("gate", {}).get(k) for k in
                                                             ("wins", "losses", "draws", "games", "paired_share", "paired_margin", "pentanomial")})

    def gate_line() -> str:
        g = rec.data["stages"].get("gate") or {}
        if g.get("skipped") or g.get("wins") is None:
            return f"the gate was not played: {g.get('skipped') or 'no gate recorded'}"
        return duel_text(g)

    def check_mined() -> list[str]:
        """The mined targets this generation's training should have and has not: an earlier generation of the mining
        window whose mine or mined stage failed. Under this driver (its record names an engine) that failure was
        allowed to pass, and training without the targets has to be allowed too (`--allow-data-failure`), else this
        raises - at the preflight, before a game is played, and again at training. A failure recorded by a driver
        from before (no engine in the record) is past mending: flagged in the README and told once, never refused.
        Returns what is flagged, kept in the record (`missing_mined`)."""
        refusing, flagged = [], []
        if not args.no_mine:
            for g in range(gen - 1, max(gen - 1 - args.mine_window, 0), -1):
                earlier = status.read_json(root / f"gen{g}" / "generation.json") or {}
                failed = [n for n in MINE_STAGES if (earlier.get("stages") or {}).get(n, {}).get("failed")]
                if not failed:
                    continue
                if "engine" in earlier:
                    refusing.append(f"gen{g}'s mined targets (its {' and '.join(failed)} failed)")
                else:
                    flagged.append(f"gen{g}'s mined targets were lost before the hardened driver; training without them")
        if refusing and not args.allow_data_failure:
            raise SystemExit(f"training would go without {'; '.join(refusing)}. Run that generation's failed stage again "
                             "(the driver with --generation N: it tries it again), or pass --allow-data-failure to "
                             "train without them")
        flagged += [f"training without {r} (--allow-data-failure)" for r in refusing]
        if flagged or rec.data.get("missing_mined"):
            rec.data["missing_mined"] = flagged
        return flagged

    # --- What a generation serves with (docs/ai-training.md, "Parity, and what a generation serves with") ---------

    def serving_option() -> str | None:
        """The serving option the parity checks fell back to, if they did: the candidate's check's once it is made
        (it holds every model the later stages serve), before it the check of the champion at the start."""
        made = rec.data.get("serving") or {}
        pick = made.get("candidate") or made.get("start")
        return pick.get("option") if pick and pick.get("index") else None

    def served(configured: str) -> str:
        """A server's arguments as this generation serves: the configured ones, their precision flags the fallback's
        when the parity check fell back."""
        option = serving_option()
        return with_precision(configured, option) if option else configured

    def serving_check(models: list[Path], when: str, test=None, duels_too: bool = False) -> tuple[dict, str]:
        """The first serving option (the configured flags, then the fallbacks in order: `with_precision`) that passes
        parity for every one of `models` together, in every server that will hold them: the generation's own
        (`--server-args`) and, `duels_too`, the one every host of a distributed duel runs (`--duel-server-args`,
        cantrip-infer) - the gate never serves flags that were not checked. A server checks each model it holds
        against its references as it starts and refuses to start on one that fails, and `test` (the parity stage's
        Rust test) is then run through the generation's own. The tolerance is the server's and the test's, never
        loosened here.

        Only a parity failure (`PARITY_FAILED`) moves on to the next option. A server that does not start for any
        other reason is raised as itself: falling back on it would serve a whole generation with a slower option
        for a GPU that was busy for a minute.

        Returns what was chosen (and what failed before it) and the test's output."""
        native = args.server_impl == "native"
        fallbacks = list([] if args.no_parity_fallback else (args.parity_fallback if args.parity_fallback is not None
                                                             else NATIVE_FALLBACKS if native else PYTHON_FALLBACKS))
        # Each server the models will be held by: its configured arguments, and whether it is cantrip-infer.
        bases = [(" ".join(args.server_args.split()), native)]
        if duels_too and duels_distributed() and decks_dir is not None:
            duel = args.duel_server_args if args.duel_server_args is not None else (args.server_args if native else "")
            if (" ".join(duel.split()), True) not in bases:
                bases.append((" ".join(duel.split()), True))

        def under(fallback: str | None) -> list[tuple[str, bool]]:
            made = []
            for base, is_native in bases:
                one = (with_precision(base, fallback) if fallback else base, is_native)
                if one not in made:
                    made.append(one)
            return made

        options: list[tuple[str | None, list]] = [(None, under(None))]
        for f in fallbacks:
            if under(f) not in [servers for _, servers in options]:
                options.append((f, under(f)))
        names = ", ".join(m.name for m in models)
        tried: list[dict] = []
        logs = pre_dir / "logs"
        for k, (fallback, servers) in enumerate(options):
            text, failed = "", None
            for n, (words, is_native) in enumerate(servers):
                try:
                    with server_for(models, when, words.split(), logs, native=is_native) as gpu:
                        if n == 0 and test is not None:
                            text = test(gpu, words)
                except SystemExit as e:
                    if not PARITY_FAILED.search(str(e)):
                        raise  # not a parity failure: no other option would mend it
                    lines = [line.strip() for line in str(e).splitlines() if line.strip()]
                    why = next((line for line in reversed(lines) if PARITY_FAILED.search(line)), "parity failed")[:300]
                    failed = dict(args=words, failed=why)
                    for kept in (logs / f"server-{when}.log", logs / "parity.log"):
                        if kept.exists():
                            shutil.copy2(kept, kept.with_name(f"{kept.stem}-try{k}{kept.suffix}"))
                    break
            if failed is not None:
                tried.append(failed)
                log(f"  parity ({when}): `{failed['args'] or '(the default)'}` does not pass for {names}: {failed['failed']}")
                continue
            if k:
                log(f"  parity ({when}): {names} served with `{servers[0][0]}` for this generation, the "
                    f"{k} option(s) before it failing")
            # `option`: the fallback's own flags, which every other server's arguments take in place of theirs (`served`).
            return dict(args=servers[0][0], option=fallback, index=k, tried=tried, models=[m.name for m in models],
                        checked=[words for words, _ in servers]), text
        raise SystemExit(f"parity ({when}): no serving option passes for {names}; tried "
                         + "; ".join(f"`{t['args'] or '(the default)'}` ({t['failed']})" for t in tried)
                         + ". The tolerance is not loosened: see the logs' -try files in " + str(logs))

    def server_for(models: list[Path], stage_name: str, words: list[str] | None = None, logs: Path | None = None,
                   native: bool | None = None):
        """The inference server for a stage, or nothing (a stand-in whose address is unused) when it is off; serving
        with the generation's serving option (`served`) unless `words` says with what."""
        if not server:
            return contextlib.nullcontext(None)
        for m in models:
            # A model exported before export wrote the server's weights: write them from its checkpoint.
            if (m / "manifest.json").exists() and not (m / "model.pt").exists() and (m / "train" / "checkpoint.pt").exists():
                run([sys.executable, "-m", "cantrip_ai.export", "--checkpoint", m / "train" / "checkpoint.pt", "--out", m,
                     "--weights-only"], gen_dir / "logs" / f"weights-{m.name}.log", args.dry_run,
                    env=dict(PYTHONPATH=str(REPO / "ai")))
        return InferenceServer(models, (logs or gen_dir / "logs") / f"server-{stage_name}.log", args.server_device or args.device,
                               served(args.server_args).split() if words is None else words,
                               args.dry_run and logs is None, log,
                               native=args.server_impl == "native" if native is None else native,
                               ort_dylib=args.ort_dylib,
                               infer=eng.path("infer") if (args.server_impl == "native" if native is None else native) else None)

    def preflight(real: bool) -> dict:
        """The checks before a generation runs (docs/ai-training.md, "Preflight"): room on the disk and, before
        training, in memory; the workers the request names there (`real`: leased and reached through the worker
        manager, else only what it could be served by); and, `real` and with the GPU server, the champion's parity
        with the serving options, falling back as the parity stage does. A problem stops the generation before it
        runs anything; a doubt is flagged and told."""
        problems, doubts = [], []
        base = root if root.exists() else root.parent
        free = disk_free(base)
        earlier = [g for n, g in retention.generations(root) if n < gen and (g / "selfplay").is_dir()]
        need = max(args.min_free_disk * 2**30, 1.2 * tree_bytes(earlier[-1]) if earlier else 0)
        if free < need:
            problems.append(f"disk: {free / 2**30:.1f} GB free under {base}, {need / 2**30:.1f} GB wanted (--min-free-disk, "
                            "and 1.2 times the last generation's size)")
        memory = available_memory()
        if memory is not None and not rec.done("train"):
            win = [d for d, _ in window(root, gen, args.window, args.older_weight, args.imitation_weight, exclude=[held])]
            on_disk = sum(f.stat().st_size for d in win if d.is_dir() for f in d.glob("*.shard.zst"))
            need = max(args.min_free_memory * 2**30, on_disk)
            if memory < need:
                problems.append(f"memory: {memory / 2**30:.1f} GB available, {need / 2**30:.1f} GB wanted before training "
                                f"(--min-free-memory, and the window's {on_disk / 2**30:.1f} GB on disk)")
        if leases is not None:
            there: set[str] = set()
            if real:
                with leases.held() as held_now:
                    for w in held_now:
                        try:
                            p = subprocess.run([*distributed.workers_cli(), "exec", w["lease"], "--timeout", "60", "--",
                                                "echo", "ok"], capture_output=True, text=True, timeout=120,
                                               stdin=subprocess.DEVNULL)
                            if p.returncode == 0:
                                there.add(w["name"])
                            else:
                                log(f"  preflight: worker {w['name']} did not answer (exit {p.returncode}): "
                                    f"{(p.stderr or p.stdout).strip()[-200:]}")
                        except (OSError, subprocess.SubprocessError) as e:
                            log(f"  preflight: worker {w['name']} did not answer ({e})")
                    asked = {w["name"] for w in held_now} | set(leases.named)
            else:
                there = {w["name"] for w in candidate_workers()}
                asked = there | set(leases.named)
            missing = sorted(asked - there)
            if missing:
                (problems if args.require_workers else doubts).append(
                    f"workers: {', '.join(missing)} of --workers {args.workers!r} "
                    + ("could not be leased or reached" if real else "are not in the pool or are disabled")
                    + ("" if args.require_workers else ": the distributed stages are played without them"))
        if not rec.done("train"):
            try:
                doubts += check_mined()
            except SystemExit as e:
                problems.append(str(e))
        if reanalyse_settings().enabled() and not rec.done("reanalyse"):
            # The deep reanalysis needs games that can be stood in again, and a network to search them with.
            traced = args.selfplay_traces or (gen_dir / "selfplay" / "traces").is_dir()
            mines = (not args.no_mine and args.accept != "always") or rec.done("mine")
            wants = args.reanalyse_source
            if not ((wants in ("auto", "selfplay") and traced) or (wants in ("auto", "gate") and mines)):
                problems.append(f"reanalyse (--reanalyse-source {wants}): no games it could stand in again - self-play "
                                "keeps its traces with --selfplay-traces, and the gate's games are recorded by the mine "
                                "stage (not with --no-mine or --accept always)")
            try:
                reanalyse_model(check=real)
            except SystemExit as e:
                problems.append(str(e))
        made = rec.data.setdefault("serving", {})
        if real and server and "start" not in made and not rec.done("selfplay"):
            if not (champ / "manifest.json").exists():
                problems.append(f"the champion {champ} has no model")
            else:
                try:
                    made["start"], _ = serving_check([champ], "start")
                except SystemExit as e:
                    if not PARITY_FAILED.search(str(e)) and "no serving option passes" not in str(e):
                        raise  # a server that did not start for another reason: that failure, as itself
                    problems.append(str(e))
        out = dict(at=now(), disk_free_gb=round(free / 2**30, 1),
                   memory_available_gb=round(memory / 2**30, 1) if memory is not None else None, warnings=doubts)
        for d in doubts:
            log(f"  preflight WARNING: {d}")
        if problems:
            raise SystemExit("preflight failed; nothing was run:\n" + "\n".join(f"  - {x}" for x in problems))
        log(f"  preflight passed: {out['disk_free_gb']} GB of disk free"
            + (f", {out['memory_available_gb']} GB of memory available" if memory is not None else "")
            + (f"; serving the champion with `{made['start']['args']}`" if made.get("start") else ""))
        return out

    def build():
        if not eng.same():
            # The tree is another engine now: this generation's is built already, and is the one it runs.
            log(f"  not built: the working tree is not this generation's engine; it runs {eng.dir}")
            return

        def made(*names):
            # What this run's build vouches for: only these are kept as the generation's engine (`Engine.adopt`).
            if not build_dry:
                built_now.update(names)

        run(["cargo", "build", "--release", "-p", "cantrip-ai-host", "--features", "ort"], pre_dir / "logs/build-release.log",
            build_dry, stream=show_progress)
        made("selfplay", "explore", "ai-host")
        if (server and args.server_impl == "native") or args.workers:
            # On its own: the gpu feature loads onnxruntime at run time, which the other binaries are not built to.
            run(["cargo", "build", "--release", "-p", "cantrip-ai-host", "--features", "gpu", "--bin", "cantrip-infer"],
                pre_dir / "logs/build-infer.log", build_dry, stream=show_progress)
            made("infer")
        run(["cargo", "build", "--profile", "cards", "-p", "cantrip-host", "--bin", "cantrip-native-host"],
            pre_dir / "logs/build-native.log", build_dry, stream=show_progress)
        made("native-host")
        if not args.no_mine:
            # The recording native host for the pair miner's replays, in a target directory of its own (the same binary
            # with a feature on would otherwise take the place of the table's).
            run(["cargo", "build", "--profile", "cards", "-p", "cantrip-host", "--bin", "cantrip-native-host",
                 "--features", "ai-record"], pre_dir / "logs/build-native-record.log", build_dry,
                env=dict(CARGO_TARGET_DIR=str(REPO / "target" / "record")), stream=show_progress)
            made("native-host-record")
        run([*dotnet(), "build", "dotnet/tools/Cantrip.BotLab/Cantrip.BotLab.csproj", "-c", "Release", "-v", "q", "--nologo"],
            pre_dir / "logs/build-botlab.log", build_dry, env=dict(DOTNET_ROLL_FORWARD="Major"), stream=show_progress)
        made("botlab")
        if duels_distributed():
            # BotLab for the workers: self-contained, one file, for each kind of worker there is.
            for kind in sorted({w["os"] for w in candidate_workers()}):
                rid, _ = BOTLAB_RIDS[kind]
                run([*dotnet(), "publish", "dotnet/tools/Cantrip.BotLab/Cantrip.BotLab.csproj", "-c", "Release", "-r", rid,
                     "--self-contained", "-p:PublishSingleFile=true", "-p:IncludeNativeLibrariesForSelfExtract=true",
                     "-o", BOTLAB_PUBLISH / rid, "-v", "q", "--nologo"], pre_dir / f"logs/build-botlab-{rid}.log",
                    build_dry, env=dict(DOTNET_ROLL_FORWARD="Major"), stream=show_progress)
                made(f"botlab-{kind}")

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
                            ("--scale-floor", args.selfplay_scale_floor), ("--cpu-ceiling", args.selfplay_cpu_ceiling)):
            if value is not None:
                cmd += [flag, value]
        return cmd

    def selfplay():
        out = gen_dir / "selfplay"
        resume = out.exists() and any(p.suffix == ".zst" or p.suffix == ".part" for p in out.iterdir())
        if out.exists() and not args.dry_run and not resume:
            shutil.rmtree(out)
        if not args.dry_run:
            # A stop asked of an earlier run (`--stop`) is not this one's: its local run's, and the whole run's.
            (out / distributed.STOP_FILE).unlink(missing_ok=True)
            (gen_dir / "dist" / "stop").unlink(missing_ok=True)
        cmd = selfplay_command(eng.path("selfplay"), champ, out, args.lanes)
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
            with workers_held():
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
        workers = load_workers()
        dist_dir = gen_dir / "dist"
        tag = distributed.root_tag(root)
        log(f"  distributed self-play: {len(workers)} worker(s) ({', '.join(w['name'] for w in workers) or 'none'})"
            + (" and this machine" if args.selfplay_local else "") + f"; ledger {dist_dir / 'ledger.json'}")
        default_server = args.worker_server_args
        if default_server is None:
            # The native server's settings, not serve.py's (whose arguments `--server-args` holds with the python one).
            default_server = args.server_args if args.server_impl == "native" and args.server_args.strip() else distributed.WORKER_SERVER_ARGS
        # With the precision the generation's parity check settled on, where it fell back (`served`).
        default_server = served(default_server)
        # A worker's own arguments keep everything but their precision, which is the generation's where the parity
        # check fell back (`served`).
        server_words = {w["name"]: served(" ".join(distributed.worker_server_args(w, default_server))).split() for w in workers}
        if args.dry_run:
            for w in workers:
                log(f"  would push the bundle to {w['name']}:{w['root']}/bundle/<hash> and play in "
                    f"{w['root']}/gens/{tag}/gen{gen}/{w['name']}/<attempt>-<nonce>, cantrip-infer {' '.join(server_words[w['name']])}")
            log(f"$ {' '.join(str(c) for c in cmd)} --grant {dist_dir / '<attempt>.grant'} --host {distributed.local_name()} "
                f"--lease {dist_dir / '<attempt>.heartbeat'} --lease-seconds {args.dist_lease:g}")
            return None
        cache_path = root / "dist" / "hashes.json"
        cache = distributed.read_json(cache_path) or {}
        # A bundle for each kind of worker: a Windows one's carries this machine's binaries and libraries, a Linux one's
        # the source it builds its own from (`distributed.LinuxSshHost`).
        bundles: dict[str, tuple] = {}  # its files, manifest and hash, as a host takes them
        if any(w.get("os", "windows") == "windows" for w in workers):
            cuda = os.environ.get("CANTRIP_CUDA_LIBS")
            if not cuda:
                import torch  # noqa: PLC0415

                cuda = str(Path(torch.__file__).parent / "lib")
            if not args.ort_dylib:
                raise SystemExit("--workers needs onnxruntime-gpu for the workers' cantrip-infer: --ort-dylib or ORT_DYLIB_PATH")
            files = distributed.bundle_files(eng.path("selfplay"), eng.path("infer"), Path(args.ort_dylib).parent, Path(cuda), champ, decks_dir,
                                             decks_dir, listed, [n.strip() for n in args.worker_cuda_dlls.split(",") if n.strip()])
            bundle_hash, manifest = distributed.bundle_manifest(files, cache)
            bundles["windows"] = (files, manifest, bundle_hash)
        if any(w.get("os") == "linux" for w in workers):
            files = distributed.linux_bundle_files(eng.source(), champ, decks_dir,
                                                   decks_dir, listed)
            bundle_hash, manifest = distributed.bundle_manifest(files, cache)
            bundles["linux"] = (files, manifest, bundle_hash)
        for kind, (files, manifest, bundle_hash) in bundles.items():
            log(f"  {kind} worker bundle {bundle_hash}: {len(files)} files, "
                f"{sum(m['size'] for m in manifest.values()) / 2**30:.2f} GB")

        def bundle_of(w: dict) -> tuple:
            return bundles[w.get("os", "windows")]
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
        hosts_identity = {w["name"]: dict(bundle=bundle_of(w)[2], backend=w.get("backend", "gpu"),
                                          server_args=" ".join(server_words[w["name"]]),
                                          precision=distributed.precision_of(server_words[w["name"]]))
                          for w in workers}
        local_identity = dict(binary=distributed.sha256(eng.path("selfplay"), cache)[:16],
                              backend="server" if server else "cpu",
                              server_args=served(args.server_args) if server else None,
                              precision=distributed.precision_of(served(args.server_args).split()) if server else None)
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
            exe = "" if w.get("os") == "linux" else ".exe"
            remote = [str(c) for c in selfplay_command(f"{b}/bin/cantrip-selfplay{exe}", model, f"{d}/selfplay",
                                                       w.get("lanes") or args.worker_lanes)[1:]]
            if decks_dir is not None:
                remote += ["--decks", f"{b}/decks", "--prior", f"{b}/decks", "--pairings", f"{b}/pairings.txt"]
            # The worker's own self-play settings (its governor's start and gains, say), after the shared ones.
            remote += [str(x) for x in w.get("selfplay_args") or []]
            remote.append("--resume")
            # The recovery opens no evaluator: a backend that needs no server.
            recover = remote + ["--backend", "tract", "--recover"]
            remote += ["--grant", f"{d}/grant.txt", "--host", host.name]
            # A Linux worker's onnxruntime and CUDA libraries are its own (its `env.json`, which its supervisor reads).
            env = ({"CANTRIP_PROGRESS": "0"} if exe == "" else
                   {"ORT_DYLIB_PATH": f"{b}/bin/onnxruntime.dll", "CANTRIP_CUDA_LIBS": f"{b}/bin", "CANTRIP_PROGRESS": "0"})
            mine = hosts_identity[host.name]
            record = dict(server_args=mine["server_args"], precision=mine["precision"], bundle=mine["bundle"])
            if w.get("backend", "gpu") == "gpu":
                return dict(selfplay=f"{b}/bin/cantrip-selfplay{exe}", selfplay_args=remote + ["--backend", "remote"],
                            recover_args=recover, env=env, infer=f"{b}/bin/cantrip-infer{exe}", record=record,
                            infer_args=["serve", "--model", model, "--port", "0", "--ready-file", f"{d}/infer.ready",
                                        "--device", str(w.get("device", 0)), *server_words[host.name]])
            return dict(selfplay=f"{b}/bin/cantrip-selfplay{exe}", selfplay_args=remote + ["--backend", w["backend"]],
                        recover_args=recover, env=env, infer=None, infer_args=[], record=dict(record, server_args=None, precision=None))

        started = time.time()
        with distributed.LedgerLock(dist_dir / "ledger.lock"):
            ledger = distributed.Ledger.load(dist_dir / "ledger.json", args.games, seed,
                                             identity=dict({"global": global_identity, "hosts": hosts_identity}),
                                             header=header, log=log)
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
                hosts += [distributed.host_class(w)(w, tag, gen, *bundle_of(w), job, log, lease=args.dist_lease)
                          for w in workers]
                if not hosts:
                    raise SystemExit("--workers with --no-selfplay-local and no worker: no host to play on")
                pairings = (sum(1 for line in listed.read_text(encoding="utf-8").splitlines() if line.strip())
                            if listed is not None else 3)
                coordinator = distributed.Coordinator(
                    ledger, hosts, out, dist_dir, read=lambda p: distributed.read_shard(p, seed, pairings), log=log,
                    chunk=args.dist_chunk or args.selfplay_shard_games, lookahead=args.dist_lookahead,
                    initial=args.dist_chunk or args.selfplay_shard_games, poll_seconds=args.dist_poll,
                    timeout=args.worker_timeout, heldout=held, traces=args.selfplay_traces, warn=flag)
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

    def shard_header() -> dict | None:
        """The header of this generation's own self-play shards: what a corpus generation has to be read beside."""
        try:
            first = next(iter(sorted((gen_dir / "selfplay").glob("*.shard.zst"))), None)
            return reanalyse.shard_header(first) if first is not None else None
        except OSError:
            return None

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
            # The pair miner's targets of the generations before, weighted by the effort behind them, at least
            # --mine-floor of the policy targets (`mining.weighted_dirs`). Self-play's policy targets: each window
            # directory's recorded visit targets (its self-play stage's), times its weight; this generation's, its own.
            targets = 0.0
            for d, weight in w:
                stage = d.parent / "generation.json"
                if d.name == "selfplay" and stage.exists():
                    sp = json.loads(stage.read_text(encoding="utf-8")).get("stages", {}).get("selfplay", {})
                    targets += weight * sp.get("visit_samples", 0)
            extra += mining.weighted_dirs(root, gen, args.mine_window, args.full_simulations or args.simulations,
                                          args.mine_floor, targets, log)
        corpus: list[str] = []
        if args.corpus_train:
            # The deep corpus, whole: every earlier generation's deep targets (`reanalyse.corpus_dirs`), by effort
            # and age, at most --corpus-share of the policy-target weight beside the window's self-play: its policy
            # targets counted by decision (`reanalyse.policy_targets`), times each directory's weight.
            window_targets = sum(weight * reanalyse.policy_targets(d) for d, weight in w if d.name == "selfplay")
            corpus, left = reanalyse.corpus_dirs(root, gen, args.full_simulations or args.simulations, args.corpus_weight,
                                                 args.corpus_decay, shard_header(), log, share=args.corpus_share,
                                                 window_targets=window_targets)
            extra += corpus
            if not args.dry_run and (left or rec.data.get("corpus_left_out")):
                rec.data["corpus_left_out"] = left
            for text in left:
                log(f"  WARNING: {text}")
            if not corpus:
                log("  corpus: --corpus-train, and no earlier generation has made deep targets yet")
        for e in extra:
            log(f"  extra: {e}")
        # The mined targets an earlier generation of the mining window should have made (`check_mined`: refused here
        # as at the preflight, unless allowed).
        if not args.dry_run:
            for text in check_mined():
                log(f"  WARNING: {text}")
        cmd = [sys.executable, "-m", "cantrip_ai.train", "--data", *[f"{d}:{weight:g}" for d, weight in w], *extra, "--out", out,
               *length, "--lr", args.lr, "--warmup", args.warmup, "--value-mix", args.value_mix, "--seed", gen,
               "--device", args.device, "--progress", "on" if show_progress else "off"]
        if args.train_hours and not args.train_steps and uses is None:
            cmd += ["--hours", round(args.train_hours * 1.25, 3)]
        # The deep corpus's held-out games (`reanalyse.exclusions`): they stay in their generation's self-play, and
        # no window that holds it may train on them, or the held-out score is of games the network saw.
        held_games = reanalyse.exclusions(root)
        if held_games:
            cmd += ["--exclude-games", *held_games]
            log(f"  left out of every directory: the deep corpus's held-out games ({len(held_games)} generation(s))")
        for flag, value in (("--search-target-repeat", args.search_target_repeat),
                            ("--imitation-policy-weight", args.imitation_policy_weight),
                            ("--value-mix-full", args.value_mix_full), ("--value-mix-fast", args.value_mix_fast)):
            if value is not None:
                cmd += [flag, value]
        if args.schedule != "cosine":
            cmd += ["--schedule", args.schedule]
        champion_checkpoint = str(champ / "train" / "checkpoint.pt")
        given = [word.replace("{champion}", champion_checkpoint) for word in args.train_args.split()]

        def start(resume: bool) -> list:
            if resume:
                return ["--resume"]
            if init.exists() or args.dry_run:
                return ["--init", init, *(["--init-optimizer"] if args.carry_optimizer else [])]
            return []

        # The data loader's workers each attach the window; when they do not fit in memory the trainer fails, or
        # hangs waiting on workers that are gone (`LOADER_OOM`, `fatal_watch`). That one failure is tried again with
        # half the workers, down to none (the loader then runs in the trainer itself), from the last checkpoint.
        workers_now = loader_workers(given)
        fewer: list = []
        recovered = []
        for attempt in range(args.train_oom_retries + 1):
            try:
                run([*cmd, *start(resuming or (attempt > 0 and (out / "checkpoint.pt").exists())), *given, *fewer],
                    gen_dir / "logs/train.log", args.dry_run, env=dict(PYTHONPATH=str(REPO / "ai")), stream=show_progress,
                    fatal=LOADER_OOM)
                break
            except SystemExit as e:
                if not LOADER_OOM.search(str(e)) or attempt == args.train_oom_retries or workers_now == 0:
                    raise
                kept = gen_dir / "logs" / f"train-oom{attempt + 1}.log"
                if (gen_dir / "logs/train.log").exists():
                    shutil.copy2(gen_dir / "logs/train.log", kept)
                was, workers_now = workers_now, workers_now // 2
                fewer = ["--workers", workers_now]
                recovered.append(dict(workers_before=was, workers_now=workers_now, log=str(kept)))
                log(f"  training's data loader ran out of memory with {was} workers: run again with {workers_now} (its log: {kept})")
                tell("recovered", f"gen{gen}'s training ran out of memory in its data loader with {was} workers; running "
                                  f"it again with {workers_now}", "train", log_file=str(kept))
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
                    **({"loader_recovered": recovered} if recovered else {}), **({"corpus": corpus} if corpus else {}),
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
        def test(gpu, option: str) -> str:
            env = dict(CANTRIP_MODEL=str(model_dir))
            if gpu is not None:
                env["CANTRIP_AI_SERVER"] = gpu.address
                # A server below fp32 is held to the reduced bar, the one it checks itself against as it starts.
                if distributed.precision_of(option.split()) != "fp32":
                    env["CANTRIP_AI_SERVER_REDUCED"] = "1"
            return run(["cargo", "test", "--profile", "cards", "-p", "cantrip-ai", "--features", "ort", "--test", "eval",
                        "model_parity", "--", "--ignored", "--nocapture"], gen_dir / "logs/parity.log", args.dry_run,
                       env=env, stream=show_progress)

        pick = None
        if server and not args.dry_run:
            # Every model a server of this generation holds from here on, together: the candidate and the champion
            # (the gate's and the miner's servers), and the ladder's anchors. The first serving option that passes
            # all of them is the generation's (`serving_check`), and every later stage serves with it.
            models = [model_dir, champ]
            if args.ladder_every > 0:
                models += [m for m in (anchor_model(x.strip()) for x in args.ladder_anchors.split(",") if x.strip())
                           if m is not None]
            unique = list({m.resolve(): m for m in models}.values())
            pick, text = serving_check(unique, "candidate", test, duels_too=True)
            rec.data.setdefault("serving", {})["candidate"] = pick
        else:
            with server_for([model_dir], "parity") as gpu:
                text = test(gpu, args.server_args)
        said = [l for l in text.splitlines() if "parity" in l.lower() or "max" in l.lower()]
        summary = (said[-1].strip() if said else "passed").replace("|", "/")
        if pick is not None:
            summary += (f"; served with `{pick['args']}`" + (f" (fallback {pick['index']}: the options before it fail for "
                        f"{', '.join(pick['models'])})" if pick["index"] else ""))
        return dict(summary=summary, **({"serving": pick["args"], "served": pick["models"]} if pick else {}))

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

    # The run's worker machines (`distributed.Leases`): leased from the worker manager for each distributed stage.
    leases = (distributed.Leases(args.workers, f"generation:{distributed.root_tag(root)}", f"gen{gen}", log)
              if args.workers else None)
    candidates_made: list = []

    def candidate_workers() -> list[dict]:
        """The machines the request could lease, none leased: what a dry run plans with and the build publishes for."""
        if leases is None:
            return []
        if not candidates_made:
            candidates_made.append(leases.candidates())
        return candidates_made[0]

    def load_workers() -> list[dict]:
        """The workers held now (within `workers_held`); a dry run's are the request's candidates."""
        if leases is None:
            return []
        return candidate_workers() if args.dry_run else leases.workers

    def workers_held():
        """The workers leased while a distributed stage runs; a dry run leases nothing."""
        return leases.held() if leases is not None and not args.dry_run else contextlib.nullcontext()

    def duels_distributed() -> bool:
        return bool(args.workers) and args.dist_duels

    def worker_libraries() -> tuple[Path | None, Path | None]:
        """What a Windows worker's cantrip-infer loads: onnxruntime-gpu's directory and CUDA's libraries'."""
        cuda = os.environ.get("CANTRIP_CUDA_LIBS")
        if not cuda:
            with contextlib.suppress(ImportError):
                import torch  # noqa: PLC0415

                cuda = str(Path(torch.__file__).parent / "lib")
        return (Path(args.ort_dylib).parent if args.ort_dylib else None), (Path(cuda) if cuda else None)

    duel_setup_made: list = []

    def duel_setup() -> duels.Setup:
        """What every distributed duel of this generation shares (`duels.Setup`): made once."""
        if duel_setup_made:
            return duel_setup_made[0]
        workers = load_workers()
        server_args = args.duel_server_args
        if server_args is None:
            server_args = args.server_args if args.server_impl == "native" else ""
        words = distributed.worker_server_args({"name": "every host of a duel", "server_args": served(server_args)}, "")
        ort_dir, cuda_dir = worker_libraries()
        cache = distributed.read_json(root / "dist" / "hashes.json") or {}

        def local_server(models: list[Path]):
            # Every host's server of a duel is cantrip-infer with the same arguments, this machine's too.
            return InferenceServer(models, gen_dir / "logs" / "server-duel.log", args.server_device or args.device, words,
                                   args.dry_run, log, native=True, ort_dylib=args.ort_dylib, infer=eng.path("infer"))

        setup = duels.Setup(
            root=root, gen=gen, gen_dir=gen_dir, repo=REPO, workers=workers, local=args.duel_local,
            local_name=distributed.local_name(), botlab_local=[*dotnet(), eng.path("botlab")],
            botlab={kind: eng.find(f"botlab-{kind}") for kind in BOTLAB_RIDS},
            native=eng.find("native-host"), record_native=eng.find("native-host-record"), ai_host=eng.find("ai-host"), infer=eng.find("infer"), ort_dir=ort_dir, cuda_dir=cuda_dir,
            cuda_dlls=tuple(n.strip() for n in args.worker_cuda_dlls.split(",") if n.strip()), server_args=words,
            local_parallel=args.gate_parallel, worker_parallel=args.worker_duel_parallel,
            server=local_server if server else None, lease=args.dist_lease, poll=args.dist_poll,
            timeout=args.worker_timeout, log=log, cache=cache, source=eng.source)
        if args.dist_duels and not args.dry_run:
            duels.check_published(setup.botlab, workers)
        duel_setup_made.append(setup)
        return setup

    def save_hashes():
        if duel_setup_made:
            (root / "dist").mkdir(parents=True, exist_ok=True)
            distributed.write_atomic(root / "dist" / "hashes.json", json.dumps(duel_setup_made[0].cache))

    def distributed_duel(name: str, duel_args: list, stops: list, models: list[Path], games: int, at: int, kind=duels.DUEL,
                         only=None, affinity=None, record: str | None = None, env: dict | None = None,
                         restart_stale: bool = False):
        """A duel across this machine and the workers (`duels.run`): its results merged into genN/logs/NAME-results.tsv,
        as one machine writes them, and BotLab's summary of them in genN/logs/NAME.log. Returns the summary's text,
        the ledger and the results directory; a game that did not finish fails it, as it fails one machine's."""
        pairings = next((Path(duel_args[k + 1]) for k in range(len(duel_args) - 1) if duel_args[k] == "--pairings"), None)
        unique = list(dict.fromkeys(Path(m) for m in models))
        with workers_held():
            setup = duel_setup()
            # The machines held now may be of a kind the setup's first did not have (a box rented since).
            duels.check_published(setup.botlab, setup.workers)
            try:
                ledger, out, rows, seconds = duels.run(setup, name, duel_args, unique, games, at, decks_dir, pairings,
                                                       kind=kind, only=only, affinity=affinity, stops=stops, record=record,
                                                       env=env, restart_stale=restart_stale)
            finally:
                save_hashes()
        results = gen_dir / "logs" / f"{name}-results.tsv"
        duels.write_results(rows, results)
        code, text = duels.summarize(setup, [str(x) for x in duel_args], results, seconds, stops)
        hosts = duels.host_rows(ledger)
        text += "\nPlayed on " + "; ".join(f"{h}: {r['games']} games" for h, r in hosts.items()) + f" (ledger {out.parent / 'ledger.json'}).\n"
        (gen_dir / "logs" / f"{name}.log").write_text(text, encoding="utf-8")
        if code != 0:
            raise SystemExit(f"{name}: a game did not finish on any host it was given to (the summary: "
                             f"{gen_dir / 'logs' / f'{name}.log'})")
        return text, ledger, out

    def duel(name: str, a: str, b: str, models: list[Path], games: int, at: int, extra: list, b_model: Path | None) -> dict | None:
        """A BotLab duel, A playing `models[0]`, B `b_model` (or a BotLab seat by name), logged to genN/logs/NAME.log;
        its summary read by `parse_gate`, or None in a dry run. With `--workers` (and `--dist-duels`), played across
        this machine and the workers (`distributed_duel`)."""
        seats = ["duel", "--a", a, "--b", b, "--model", models[0]]
        if b_model is not None:
            seats += ["--model-b", b_model]
        decks = dealt(name, games, at)
        if duels_distributed() and decks_dir is not None:
            # The sequential stops are the driver's (each host plays what it is granted); every other argument the same.
            duel_args = [*seats, *decks, "--games", games, "--seed", at, *[x for x in extra if x not in STOP_FLAGS]]
            stops = [x for x in extra if x in STOP_FLAGS]
            if args.dry_run:
                log(f"  would play {name} across this machine and the workers: BotLab {' '.join(str(x) for x in duel_args + stops)}")
                return None
            text, ledger, _ = distributed_duel(name, duel_args, stops, models, games, at)
            return dict(parse_gate(text), hosts=duels.host_rows(ledger))
        cmd = [*dotnet(), eng.path("botlab"), *seats, *decks, "--games", games, "--parallel", args.gate_parallel, "--seed", at, *extra]
        # Every game's result beside the log (BotLab's --results): at a fixed budget a game is played again exactly from
        # its number and the duel's seed, seats and decks (--only-games), so any pair's positions can be mined later.
        cmd += ["--results", gen_dir / "logs" / f"{name}-results.tsv"]
        env = {"DOTNET_ROLL_FORWARD": "Major", "CANTRIP_NATIVE_HOST": str(eng.path("native-host")), "CANTRIP_AI_HOST": str(eng.path("ai-host"))}
        with server_for(models, name) as gpu:
            if gpu is not None:
                cmd += ["--ai-server", gpu.address]
            text = run(cmd, gen_dir / "logs" / f"{name}.log", args.dry_run, env=env, stream=show_progress)
        return None if args.dry_run else parse_gate(text)

    def mine_settings() -> mining.Settings:
        # --tiny: the judgements at a toy scale too (a few simulations in two worlds), so the whole stage runs in a test.
        tiny = dict(fork_simulations=16, fork_worlds=2, deep_simulations=32, deep_worlds=2, loser_simulations=16,
                    loser_worlds=2, batch=4) if args.tiny else {}
        return mining.Settings(lanes=args.mine_lanes, control=args.mine_control, heldout=args.mine_heldout,
                               seed=seed % 1_000_000 + 7, **tiny)

    def mine():
        """The pair miner's first half (`mining.py`): the gate's decisive pairs (and a control) replayed and recorded on
        this machine (with a distributed gate, each game on the host that played it), their forks and losers' decisions
        found and planned, and judged (here, or with --workers across this machine and the workers:
        `mining.judge_distributed`). The judge is the network the gate leaves champion."""
        p, s = mining.Paths(gen_dir), mine_settings()
        results = gen_dir / "logs" / "gate-results.tsv"
        accepted = (rec.data["stages"].get("promote") or {}).get("accepted")
        judge_model = model_dir if accepted else champ
        py = [sys.executable, "-m", "cantrip_ai.mine"]
        pyenv = dict(PYTHONPATH=str(REPO / "ai"))
        logs = gen_dir / "logs"
        if args.dry_run:
            log(f"  would mine the gate's pairs in {p.root}, judged by {judge_model.name}"
                + (" across this machine and the workers" if args.workers else " here"))
            return
        if not results.exists():
            raise SystemExit(f"no gate results to mine ({results})")
        # A recording replays only with the engine that made it: the gate's games are stood in again by this
        # generation's pinned engine, which must be the one that played them.
        played = rec.data["stages"].get("gate", {}).get("engine")
        if not played or not eng.kept():
            # Nothing to hold the replays to: said plainly, and left to the verify step, which keeps only the pairs
            # whose replayed games are the gate's.
            log("  the gate's record names no pinned engine" if not played else "  no build of this engine is pinned now")
            log("  (a gate played with --no-build, or by a driver from before): that its games replay on the binaries "
                "run now cannot be checked beforehand; a pair whose replay differs is left out by the verify step")
        if played and eng.id and played != eng.id:
            raise SystemExit(f"the gate was played by engine {played} and this generation's engine is now {eng.id} "
                             f"({engine.describe(eng.pinned)}): its games do not replay on another engine (the miner "
                             "would report MismatchedPositions). Play the gate again on this engine (remove the gate "
                             "and promote stages from generation.json), or pass --no-mine")
        # A stage run again keeps its replays when they were made and verified against these gate results: the replay is
        # this stage's longest step, and the same games again would be the same recordings.
        verified_file = p.replay / "verify.json"
        engine_file = p.replay / "engine.txt"
        resumed = (verified_file.exists() and (p.selected / "selection.json").exists()
                   and verified_file.stat().st_mtime > results.stat().st_mtime
                   and (not eng.id or not engine_file.exists() or engine_file.read_text(encoding="utf-8").strip() == eng.id))
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
        env = {"DOTNET_ROLL_FORWARD": "Major", "CANTRIP_NATIVE_HOST": str(eng.path("native-host-record")), "CANTRIP_AI_HOST": str(eng.path("ai-host")),
               "CANTRIP_AI_TRACES": "1"}
        p.replay.mkdir(parents=True, exist_ok=True)
        if eng.id and not resumed:
            engine_file.write_text(eng.id, encoding="utf-8")
        replayed = []
        # A distributed gate's games are replayed each on the host that played it (`duels.REPLAY`): a game replays
        # exactly only on that hardware.
        gate_ledger = gen_dir / "dist" / "gate" / "ledger.json"
        spread = duels_distributed() and gate_ledger.exists() and decks_dir is not None
        with server_for([model_dir, champ], "mine-replay") if not resumed and not spread else contextlib.nullcontext(None) as gpu:
            for half in ("first", "second"):
                games = p.selected / f"{half}.txt"
                if not games.read_text(encoding="utf-8").strip():
                    continue
                out = p.replay / f"{half}-results.tsv"
                replayed.append(out)
                if resumed:
                    continue
                if spread:
                    played_on = duels.hosts_of(distributed.Ledger.load(gate_ledger, args.gate_games, seed + 5_000_000),
                                               distributed.local_name())
                    wanted = [int(x) for x in games.read_text(encoding="utf-8").split()]
                    replay_args = ["duel", "--a", name, "--b", name, "--model", model_dir, "--model-b", champ, *decks,
                                   "--games", args.gate_games, "--seed", seed + 5_000_000, "--ai-record", "record",
                                   "--ai-record-mode", "decklist", "--ai-record-pairs"]
                    # Not `results`: that is the gate's results file, which `verify` below compares the replays with.
                    _, ledger, replay_results = distributed_duel(f"replay-{half}", replay_args, [], [model_dir, champ],
                                                                 args.gate_games, seed + 5_000_000, kind=duels.REPLAY,
                                                                 only=wanted, affinity=played_on, record="record",
                                                                 env={"CANTRIP_AI_TRACES": "1"}, restart_stale=True)
                    shutil.copy2(gen_dir / "logs" / f"replay-{half}-results.tsv", out)
                    shards = duels.recordings(ledger, replay_results, "record", p.recorded / half)
                    moved = sum(1 for g in wanted if played_on.get(g) and ledger.shards.get(ledger.done.get(g, ""), {}).get("host") != played_on[g])
                    log(f"  replayed {len(ledger.done)} of {len(wanted)} {half} games across the hosts ({shards} recorded shards)"
                        + (f"; {moved} on another host than the gate's (their pairs are checked as every replay is)" if moved else ""))
                    continue
                cmd = [*dotnet(), eng.path("botlab"), "duel", "--a", name, "--b", name, "--model", model_dir, "--model-b", champ, *decks,
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
        run([eng.path("explore"), "mine-forks", "--first", p.recorded / "first", "--second", p.recorded / "second", "--out", p.forks,
             "--lanes", 32], logs / "mine-forks.log", False)
        # The models as directories: mine-values loads each loser's network from its decision's `model`.
        run([*py, "losers", "--selected", p.selected, "--a-model", model_dir, "--b-model", champ],
            logs / "mine-losers.log", False, env=pyenv)
        values = []
        with server_for([model_dir, champ], "mine-values") as gpu:
            for half in ("first", "second"):
                out = p.values(half)
                run([eng.path("explore"), "mine-values", "--dir", p.recorded / half, "--games", p.selected / f"losers-{half}.jsonl",
                     "--out", out, "--lanes", 16, *(["--server", gpu.address] if gpu is not None else [])],
                    logs / f"mine-values-{half}.log", False)
                values.append(out)
        run([*py, "plan", "--selected", p.selected, "--forks", p.forks, "--values", *values, "--out", p.plan,
             "--heldout", s.heldout, "--before", s.before, "--seed", s.seed], logs / "mine-plan.log", False, env=pyenv)
        planned = mining.split_plan(p)
        log(f"  planned {planned['forks']} forks and {planned['losers']} losers' decisions")
        where = "here"
        if args.workers:
            # Across this machine and the workers (`mining.judge_distributed`): each run's plan split by game.
            with workers_held():
                setup = duel_setup()
                setup = duels.Setup(**{**setup.__dict__, "local": args.judge_local})
                try:
                    parts = mining.judge_distributed(setup, p, s, judge_model, decks_dir, eng.path("explore"), args.mine_local_lanes,
                                                     args.worker_judge_lanes or 4 * args.mine_lanes)
                finally:
                    save_hashes()
            p.job.write_text(json.dumps(dict(worker=None, distributed=parts, model=judge_model.name)), encoding="utf-8")
            where = ", ".join(sorted({h for r in parts.values() for h in r})) or "nowhere"
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
                run([eng.path("explore"), *r["args"], *(["--server", gpu.address] if gpu is not None else [])],
                    gen_dir / "logs" / f"mine-judge-{r['name']}.log", False, stream=show_progress)

    def mined():
        """The pair miner's second half: the judgements checked, the decided forks and the loser
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
            raise SystemExit(f"the mine stage started its judgements on {job['worker']} in the way before 2026-11-20 "
                             "(one worker, in the background): run the mine stage again to judge them now")
        judged = mining.check_judged(p.judge, [name for name, _, _ in mining.FORK_RUNS] + [mining.DEEP_RUN, mining.LOSERS_RUN])
        log(f"  judged: {json.dumps(judged)}")
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
            # The forks' targets from the deeper run, of the forks the verdict runs decided (the keep lists).
            for run_dir in (p.judge / mining.DEEP_RUN, p.judge / mining.LOSERS_RUN):
                if (run_dir / split).is_dir():
                    run([eng.path("explore"), "mine-filter", "--from", run_dir / split, "--keep", p.keep / f"keep-{split}.jsonl",
                         "--out", dest / run_dir.name, "--producer", producer],
                        logs / f"mine-filter-{split}-{run_dir.name}.log", False)
                    st = mine_settings()
                    deep = run_dir.name == mining.DEEP_RUN
                    if (dest / run_dir.name).is_dir():
                        mining.write_effort(dest / run_dir.name, st.deep_simulations if deep else st.loser_simulations,
                                            st.deep_worlds if deep else st.loser_worlds)
        settings = mine_settings()
        (p.root / "settings.json").write_text(json.dumps(dict(
            # The keys `mine report` reads.
            fork=f"{settings.fork_simulations} simulations x {settings.fork_worlds} worlds, two seeds pooled",
            target=f"{settings.fork_simulations} x {settings.fork_worlds}, the first seed's (training's targets: "
                   f"{settings.deep_simulations} x {settings.deep_worlds})",
            loser=f"{settings.loser_simulations} simulations x {settings.loser_worlds} worlds", kl_min=settings.kl_min)),
            encoding="utf-8")
        run([*py, "report", "--selected", p.selected, "--forks", p.forks, "--runs", *runs, "--losers",
             p.judge / mining.LOSERS_RUN, "--settings", p.root / "settings.json", "--a-name", f"gen{gen}", "--b-name",
             champ.name, "--out", p.report], logs / "mine-report.log", False, env=pyenv)
        log(f"  mined targets: {json.dumps(kept)}; report {p.report}")
        return dict(kept=kept, targets=str(p.targets), heldout=str(p.heldout), report=str(p.report))

    def reanalyse_settings() -> reanalyse.Settings:
        # --tiny: the searches at a toy scale too, on two lanes with no probe, so the whole stage runs in a test.
        tiny = dict(simulations=32, worlds=2, batch=4, lanes=args.reanalyse_lanes or 2) if args.tiny else {}
        seeds = tuple(int(x) for x in str(args.reanalyse_seeds).split(",") if x.strip()) or (1,)
        given = dict(positions=args.reanalyse_positions, hours=args.reanalyse_hours, worlds=args.reanalyse_worlds,
                     simulations=args.reanalyse_simulations, memory_mb=args.reanalyse_memory_mb, seeds=seeds,
                     batch=args.reanalyse_batch, per_game=args.reanalyse_per_game, heldout=args.reanalyse_heldout,
                     source=args.reanalyse_source, mode=args.mode, search=tuple(args.reanalyse_search),
                     lanes=args.reanalyse_lanes, max_lanes=args.reanalyse_max_lanes, local_lanes=args.reanalyse_local_lanes,
                     part_games=args.reanalyse_part_games, reprobe=args.reanalyse_reprobe, sample_seed=gen * 1000 + 11)
        return reanalyse.Settings(**{**given, **tiny})

    def reanalyse_model(check: bool = True) -> Path:
        """The network that searches: the one named (`check`: it has to be there), else the champion the gate leaves."""
        if args.reanalyse_model:
            named = [Path(args.reanalyse_model), root / "champions" / args.reanalyse_model, root / "models" / args.reanalyse_model]
            found = next((m for m in named if (m / "manifest.json").exists()), None)
            if found is None and check:
                raise SystemExit(f"--reanalyse-model {args.reanalyse_model}: no model there, nor a champion or a model of that name")
            return (found or named[0]).resolve()
        return model_dir if (rec.data["stages"].get("promote") or {}).get("accepted") else champ

    def reanalyse_stage():
        """The deep reanalysis (`reanalyse.py`): positions of this generation's recorded games, searched again far
        deeper than self-play's full search across this machine and the workers, kept in the corpus."""
        s = reanalyse_settings()
        judge_model = reanalyse_model(check=not args.dry_run)
        if args.dry_run:
            log(f"  would search {'at most ' + str(s.positions) if s.positions else 'as many as the budget buys of the'} "
                f"positions of gen{gen}'s recorded games ({s.source}) at {s.simulations} simulations over {s.worlds} worlds"
                + (f", for {s.hours:g} h at most" if s.hours else "") + f", judged by {judge_model.name}"
                + (" across the workers" + (" and this machine" if args.judge_local else "") if args.workers else " here")
                + f", into {reanalyse.Paths(root, gen).corpus}")
            return
        def call(cmd, log_path):
            return run(cmd, log_path, False)

        with workers_held():
            setup = duel_setup()
            ledger_file = gen_dir / "dist" / "ledger.json"
            if args.selfplay_traces and ledger_file.exists() and setup.workers:
                # Self-play's traces that did not come home (a fetch that failed, an attempt given up as lost: its
                # output is still on its worker): asked for again now that the workers are held.
                tag = distributed.root_tag(root)
                hosts = {w["name"]: distributed.host_class(w)(w, tag, gen, {}, {}, "", None, log) for w in setup.workers}
                came = distributed.retry_traces(ledger_file, gen_dir / "selfplay", gen_dir / "dist", hosts, log)
                if came["fetched"]:
                    log(f"  traces fetched now: {', '.join(came['fetched'])}")
                    kept = [f for f in rec.data.get("flagged") or [] if not any(f"{a}'s traces" in f for a in came["fetched"])]
                    if kept != (rec.data.get("flagged") or []):
                        rec.data["flagged"] = kept
                        rec.save()
                if came["failed"]:
                    log(f"  traces still not fetched: {', '.join(came['failed'])} (their games are not drawn from)")
            # A recording replays only with the engine that made it: the stage that made these games (self-play, or the
            # mine stage's replays) must have run this generation's engine.
            kind, _ = reanalyse.sources(gen_dir, s.source)
            made_by = rec.data["stages"].get("selfplay" if kind == "selfplay" else "mine", {}).get("engine")
            if made_by and eng.id and made_by != eng.id:
                raise SystemExit(f"the {kind} games were recorded by engine {made_by} and this generation's engine is now "
                                 f"{eng.id} ({engine.describe(eng.pinned)}): they do not replay on another engine. Give "
                                 "--reanalyse-source the other recordings, or no --reanalyse-positions/--reanalyse-hours")
            # This machine searches unless it is left out (`--no-judge-local`), and always where no workers were
            # asked for. Left out, with workers asked for and none leased, nothing searches and the stage fails: a
            # machine kept out of compute is never the fallback.
            setup = duels.Setup(**{**setup.__dict__, "local": args.judge_local or not args.workers})
            try:
                return reanalyse.Reanalysis(root, gen, s, judge_model, decks_dir if decks_dir is not None else REPO / "decks",
                                            eng.path("explore"), setup, call,
                                            engine=dict(id=eng.id, commit=(eng.pinned or {}).get("commit")), log=log).run()
            finally:
                save_hashes()

    def by_matchup(name: str) -> dict | None:
        """Duel `name` by matchup (`reports.matchups`), from its results file (one machine's, or the hosts' merged), its
        table logged and anything flagged said; None where there is no file (a duel played before BotLab kept one)."""
        results = gen_dir / "logs" / f"{name}-results.tsv"
        table = reports.matchups([results]) if results.exists() else None
        if table is not None:
            for line in reports.matchup_table(table):
                log(f"  {line}")
            for f in reports.matchup_flags(table):
                log(f"  WARNING: {name}, {f}")
        return table

    def evaluate_corpus(networks: list[str]) -> dict | None:
        """The networks' policy against the deep corpus's held-out slices (`reanalyse.heldout_dirs`: of the corpus
        generations this one trained beside, the games no training saw): whether a generation learns the deep
        targets. Cross-entropy and top-1 against the deep searches' targets, the candidate less the champion paired
        by game."""
        dirs = reanalyse.heldout_dirs(root, gen, shard_header())
        if not dirs:
            return None
        out = gen_dir / "corpus-eval.json"
        run([sys.executable, "-m", "cantrip_ai.value_eval", "--data", *dirs, "--every-game", "--policy", "--games", 1_000_000,
             "--device", args.device, "--checkpoint", *networks, "--out", out],
            gen_dir / "logs/evaluate-corpus.log", args.dry_run, env=dict(PYTHONPATH=str(REPO / "ai")), stream=show_progress)
        if args.dry_run:
            return None
        results = json.loads(out.read_text(encoding="utf-8"))
        paired = reports.heldout_pairs(results)
        keys = ("policy_ce", "top1", "top1_played", "choices")
        for r in results:
            log(f"  the deep corpus's held-out slice, {r['name']}: policy CE {_f(r.get('policy_ce'), '{:.4f}')}, top-1 "
                f"{_f(r.get('top1'))} on {r.get('choices')} deep targets")
        return dict(directories=[str(d) for d in dirs], games=results[0]["games"], choices=results[0].get("choices"),
                    networks={r["name"]: {k: r.get(k) for k in keys} for r in results},
                    paired=dict(policy_ce=paired.get("policy_ce"), of=paired["of"], against=paired["against"])
                    if paired and paired.get("policy_ce") else None)

    def scored_corpus(networks: list[str]) -> dict:
        """`evaluate_corpus` as a part of the evaluate stage's record (`corpus`), by itself: whatever fails in it is
        recorded (`corpus_failed`), flagged and logged, and never fails the stage - the permanent held-out set's
        results stand whatever becomes of the corpus's."""
        try:
            deep = evaluate_corpus(networks)
        except (Exception, SystemExit) as e:
            why = str(e) or type(e).__name__
            log(f"  WARNING: the deep corpus's held-out slice was not scored: {why.splitlines()[0][:300] if why else ''}")
            return dict(corpus_failed=why[-1500:])
        return dict(corpus=deep) if deep else {}

    def evaluate():
        # The permanent held-out set: value MSE and ECE over its games, policy CE and top-1 against its full searches'
        # targets, for the candidate and the champion on the same positions.
        networks = [f"gen{gen}={model_dir / 'train' / 'checkpoint.pt'}"]
        if (champ / "train" / "checkpoint.pt").exists() or args.dry_run:
            networks.append(f"{champ.name}={champ / 'train' / 'checkpoint.pt'}")
        if not (heldout_ready(held) or choose):
            # No permanent held-out set: the corpus's held-out slice alone.
            deep = scored_corpus(networks)
            return None if args.dry_run else deep
        out = gen_dir / "heldout-eval.json"
        run([sys.executable, "-m", "cantrip_ai.value_eval", "--data", held, "--every-game", "--policy", "--games",
             args.heldout_eval_games, "--device", args.device, "--checkpoint", *networks, "--out", out],
            gen_dir / "logs/evaluate.log", args.dry_run, env=dict(PYTHONPATH=str(REPO / "ai")), stream=show_progress)
        if args.dry_run:
            scored_corpus(networks)
            return
        results = json.loads(out.read_text(encoding="utf-8"))
        keys = ("value_mse", "ece", "sign", "policy_ce", "top1", "top1_played", "choices", "positions")
        # By the seat's deck, and the candidate less the champion by game (`reports.py`; the games' own rows stay in
        # heldout-eval.json).
        seats, paired = reports.heldout_seats(results), reports.heldout_pairs(results)
        for line in reports.seat_table(seats) if seats else []:
            log(f"  {line}")
        if paired:
            log(f"  {reports.pairs_text(paired)}")
        for f in reports.seat_flags(seats):
            log(f"  WARNING: {f}")
        return dict(directory=str(held), games=results[0]["games"], positions=results[0]["positions"],
                    networks={r["name"]: {k: r.get(k) for k in keys} for r in results}, seats=seats, paired=paired,
                    **scored_corpus(networks))

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
            lines = gate_stop_lines(args.accept, args.gate_early_stop and args.no_mine, args.gate_stop_z, args.gate_stop_min_pairs,
                                    args.gate_stop_min_decisive)
            if gate_reusable(text, champ.name, name, lines, dealt_marker()):
                try:
                    recovered = parse_gate(text)
                except ValueError:
                    pass
                else:
                    log(f"[gate] reusing the finished gate in {gate_log} "
                        f"({recovered['wins']} to {recovered['losses']}, {recovered['verdict']})")
                    return dict(recovered, champion=champ.name, simulations=gate_sims, decks=decks,
                                matchups=by_matchup("gate"))
        extra = []
        tested = ["--sprt-h0", args.gate_h0, "--sprt-h1", args.gate_h1, "--sprt-alpha", args.gate_alpha, "--sprt-beta", args.gate_beta]
        if args.accept == "sprt":
            extra = ["--sprt", *tested]
        elif args.accept == "sprt-paired":
            extra = ["--sprt-paired", *tested]
        elif args.accept == "above-half" and args.gate_early_stop and args.no_mine:
            # With pair mining the gate plays all its pairs: they are the miner's material (gen9's gate stopped at 294
            # pairs and left 58 decisive ones).
            extra = ["--interval-stop", "--interval-z", args.gate_stop_z, "--interval-min-pairs", args.gate_stop_min_pairs,
                     "--interval-min-decisive", args.gate_stop_min_decisive]
        result = duel("gate", name, name, [model_dir, champ], args.gate_games, seed + 5_000_000, extra, champ)
        if result is None:
            return
        return dict(result, champion=champ.name, simulations=gate_sims, decks=decks, h0=args.gate_h0, h1=args.gate_h1,
                    measured_only=args.accept == "always", matchups=by_matchup("gate"))

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

    def promoted() -> int:
        """Which promotion this generation's is (the count of them up to it), or 0 where it promoted nothing; a dry run
        shows the stages of one that does."""
        if args.dry_run:
            return promotions(root, gen - 1) + 1
        return promotions(root, gen) if rec.data["stages"].get("promote", {}).get("accepted") else 0

    def ladder():
        # The new champion against fixed anchors: a strength scale that does not move with the champion, which
        # candidate against champion alone cannot give. Only a generation that promoted plays it (`ladder_due`).
        now_champ = (champion(root) if not args.dry_run else None) or champ
        _, play = ladder_due(promoted(), args.ladder_every, args.ladder_play_every)
        budgets = [args.ladder_budget or args.simulations] + ([args.ladder_play_budget] if play else [])
        rows, tables = [], []
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
                tables.append(by_matchup(label))
        if args.dry_run:
            return
        ladder_record(root, rows, gen)
        # By matchup in this generation's record alone: `ladder.json` keeps the duels' totals.
        return dict(rows=[dict(r, matchups=m) for r, m in zip(rows, tables)])

    def retire():
        doomed = retention.retire(root, args.keep, dry_run=args.dry_run, log=log)
        if not args.dry_run:
            doomed += engine.prune(root, engine.named(root, args.keep) | ({eng.id} if eng.id else set()), log)
        return dict(deleted=len(doomed), keep=args.keep)

    # The optional stages: the held-out set is chosen once and scored whenever there is one, and the ladder runs when
    # asked for and the generation promoted (asked once the promotion is known: a callable).
    choose = args.heldout_games > 0 and not rec.done("heldout") and not heldout_ready(held) and not (held / HELDOUT_RECORD).exists()
    if args.heldout_games > 0 and not choose and not rec.done("heldout"):
        log(f"held-out set: {held} is chosen already, and kept")
    enabled = dict(
        build=not args.no_build,
        heldout=choose or rec.done("heldout"),
        evaluate=not args.no_heldout_eval and (heldout_ready(held) or choose or rec.done("evaluate")
                                               or bool(reanalyse.heldout_dirs(root, gen))),
        # The deep reanalysis: when asked for (positions, or a time budget).
        reanalyse=reanalyse_settings().enabled() or rec.done("reanalyse"),
        ladder=lambda: rec.done("ladder") or ladder_due(promoted(), args.ladder_every, args.ladder_play_every)[0],
        # Pair mining needs a gate played in pairs whose games were kept (`--results`).
        mine=(not args.no_mine and args.accept != "always") or rec.done("mine"),
        mined=(not args.no_mine and args.accept != "always") or rec.done("mined"),
    )
    fns = (build, selfplay, heldout, train, export, parity, gate, promote, mine, reanalyse_stage, evaluate, ladder, mined,
           retire)

    def stages() -> str:
        for name, fn in zip(STAGES, fns):
            on = enabled.get(name, True)
            if on() if callable(on) else on:
                stage(name, fn)
            if name == "build":
                # Built (or not asked to): what the tree built is this generation's from here on, and the checks that
                # need a binary can be made.
                if not args.dry_run or args.preflight:
                    taken = eng.adopt(built_now, frozen=any(name_ != "build" for name_ in
                                                            (*rec.data.get("begun", {}), *rec.data["stages"])))
                    if taken:
                        log(f"  engine {eng.id}: {len(taken)} file(s) kept in {eng.dir}")
                    rec.engine = eng.id if eng.kept() else None
                    if eng.id and not eng.kept() and not args.dry_run:
                        log(f"  engine {eng.id}: nothing is pinned (no build this run): the stages run the working "
                            "tree's files, and their records name no engine")
                if not args.no_preflight:
                    log("[preflight]")
                    for h in heart:
                        h.begin("preflight")
                    checked = preflight(real=not args.dry_run or args.preflight)
                    if not args.dry_run:
                        rec.data["preflight"] = checked
                        rec.save()
                        tell_warnings()
            if name == args.stop_after:
                log(f"stopped after {name}, as asked")
                return "idle"
        return "finished"

    if args.dry_run:
        stages()
        return
    heart.append(status.Heartbeat(root, gen, model_dir, argv=sys.argv[1:] if argv is None else list(argv)).start())
    try:
        outcome = stages()
    except KeyboardInterrupt:
        heart[0].close("stopped", "interrupted")
        raise
    except (Exception, SystemExit) as e:
        if isinstance(e, SystemExit):
            why = e.code if isinstance(e.code, str) else f"exit {e.code}"
        else:
            import traceback  # noqa: PLC0415

            why = f"{type(e).__name__}: {e}\n{traceback.format_exc()[-3000:]}"
        at = heart[0].data.get("stage")
        first = (why.strip().splitlines() or ["failed"])[0][:400]
        if status.asked_to_stop(why):
            heart[0].close("stopped", why)
            tell("stopped", f"gen{gen} stopped at {at}, as asked: {first}", at)
        else:
            heart[0].close("failed", why)
            kind = status.transient(why)
            tell("stage-failed", f"gen{gen} FAILED at {at}: {first}" + (f" (transient: {kind})" if kind else ""), at,
                 error=why[-1500:], transient=kind)
        raise
    if outcome == "finished":
        rec.data["finished"] = now()
        rec.save()
        log(readme(rec.data))
        flagged = warnings(rec.data)
        took = sum(v.get("seconds", 0) for v in rec.data["stages"].values()) / 3600
        promoted_now = rec.data["stages"].get("promote", {})
        tell("finished", f"gen{gen} finished in {took:.2f} h: " + ("accepted" if promoted_now.get("accepted") else "rejected")
             + f" ({gate_line()})" + (f"; {len(flagged)} WARNING(s): " + " | ".join(f[:160] for f in flagged) if flagged else ""),
             warnings=flagged, hours=round(took, 2), accepted=promoted_now.get("accepted"))
    heart[0].close(outcome)


if __name__ == "__main__":
    main()
