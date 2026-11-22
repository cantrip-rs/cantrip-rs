"""Deep reanalysis as a stage of a generation (docs/ai-training.md, "The deep corpus"): positions drawn at random from
the generation's recorded games, each searched once far deeper and wider than self-play's full search, and the
searches kept as policy targets in a corpus that every later generation trains on and that retention never prunes.

Self-play's full searches are the line's ordinary targets, and they age out of the training window. Playing whole
games at a deep budget for better ones costs a game to get a handful of deep targets (measured 2026-11-22: 16 games an
hour on a worker at 131,072 simulations over 16 worlds, and the moves played come from the fast searches anyway);
searching recorded positions again costs only the searches (about 380 an hour on the same worker). So the games are
played as always, and this stage (`reanalyse`, after `mine`; a data stage) makes the deep targets from them:

1. `games`: the recordings that replay on the generation's engine - its own self-play, where it was played with
   `--selfplay-traces` (`genN/selfplay` and its `traces/`), else the gate's games the pair miner recorded
   (`genN/mine/recorded`) - listed (a game needs its shard and its trace) and put in one random order.
2. `probe` (once a line, `ROOT/corpus/deep/probe.json`; `--reanalyse-lanes` skips it): each host by itself, on games
   from the end of that order. One lane first, on the latest turns: `mine-judge` reports the most memory its process
   held, and the most lanes a host may run is what fits in the memory it had free less a headroom, within its CPUs
   (`pick_lanes`). Then up a ladder of lanes below that, measured in simulations a second: the host's lanes are the
   last step that still made it faster (`climb`).
3. `plan` (`genN/reanalyse/data/plan.jsonl`): games from the front of the order, a few decisions of each
   (`cantrip-explore mine-decisions`: every seat's decisions with a choice; no network), a share of the games held
   out (every tenth, say); with a time budget, no more positions than the probe says it buys (and a fifth).
4. `judge`: the plan searched once a seed, each seed a run of `mining.judge_distributed` (the self-play coordinator,
   its ledger `genN/dist/judge-deep-s<seed>/ledger.json`, parts of a host's lanes of games) on the workers and, unless
   it is left out, this machine. A time budget closes the first seed's ledger when it is spent: nothing more is
   granted, the parts in flight finish and count, the rest is skipped. A later seed searches what the first did.
5. `corpus` (`ROOT/corpus/deep/genN/`): every searched position as a policy-only shard (`mine-filter`), by seed and
   split, each directory with the `effort.json` of its search, and `CORPUS.json` written last: the judge, the engine,
   the search and what was made.

Every step is kept and a stage run again goes on where it stopped (the ledger keeps the accepted parts).

Training (`corpus_dirs`) takes every finished generation's `train` directories at the search's simulations over
self-play's full search's, times `--corpus-weight`, times `--corpus-decay` for each generation of age; the evaluate
stage scores the networks' policy against the corpus's held-out slices (`heldout_dirs`)."""

from __future__ import annotations

import collections
import json
import math
import os
import random
import re
import shutil
import statistics
import sys
import time
from dataclasses import dataclass
from pathlib import Path

from . import distributed, mining, status

CORPUS = Path("corpus") / "deep"
RECORD = "CORPUS.json"
PROBE = "probe.json"
# A corpus generation's held-out games, by `dataset.source_key`: what no later training window may read
# (`train.py --exclude-games`), since the games themselves stay in their generation's self-play.
KEYS = "heldout-games.json"
# How a failure begins when nothing could be searched for want of a host that answers (a worker out after the gate, a
# probe that searched nothing) and nothing accepted is lost by running the stage again: `status.TRANSIENT` knows it,
# and the supervisor runs the generation again after a wait instead of holding the loop.
NO_SEARCH = "the reanalysis could not search"
# A host's free memory this far from what its probe saw (a share of it) is another machine to the probe: probed again.
FREE_DRIFT = 0.125
SOURCES = ("auto", "selfplay", "gate")
# A search's tree was measured at 33 KB a simulation in the middle and 66 KB at the most (2026-11-22, 984 searches at
# 131,072 over 16 worlds; a bound of 40 KB a simulation stopped 18% of them short): the bound leaves room for the
# largest seen, and never less than the search's own default.
SIMULATION_BYTES = 64 << 10
# What a lane does before anything is measured (the same searches: a lane about 700 simulations a second once a
# worker's GPU is shared by its lanes, which serves about 17,000).
LANE_EVALS = 700.0
SERVER_EVALS = 17000.0
SERVER_CORES = distributed.PIN_SERVER_CORES
LOCAL = "local"
# A shard's and a trace file's name end in the lane and process that wrote them (`selfplay-l<lane>-<pid>`).
WRITER = re.compile(r"(selfplay-l\d+-\d+)(?:-\d+-\d+\.shard\.zst|\.jsonl)$")
TRACE_SEED = re.compile(r'\s*\{\s*"seed"\s*:\s*(\d+)')


@dataclass
class Settings:
    """What the stage is asked for (the driver's `--reanalyse-*`)."""
    positions: int = 0            # at most this many positions (0: as many as the time budget buys)
    hours: float = 0.0            # the judgements' time budget (0: none)
    worlds: int = 16
    simulations: int = 131072     # a search's simulations in all, shared by its worlds
    memory_mb: int = 0            # a search's memory bound (0: from the simulations, `memory_mb`)
    seeds: tuple = (1,)
    batch: int = 32
    per_game: int = 2             # decisions drawn from a game
    heldout: float = 0.1          # the share of the games held out of training, for measuring
    source: str = "auto"
    mode: str = "decklist"
    search: tuple = ()            # more search settings, KEY=VALUE
    lanes: int = 0                # every host's lanes, with no probe (0: probed)
    max_lanes: int = 0            # the most lanes a worker runs (0: what fits)
    local_lanes: int = 0          # the most lanes this machine runs (0: what fits)
    headroom: float = 0.3         # the share of a host's free memory left unused
    lanes_per_core: float = 1.0
    part_games: int = 0           # the games a part judges (0: a host's lanes)
    probe_positions: int = 2
    probe_ladder: tuple = (8, 16, 24, 32, 48, 64)
    probe_games: int = 3
    probe_gain: float = 0.1
    reprobe: bool = False
    sample_seed: int = 5
    sample_lanes: int = 4

    def enabled(self) -> bool:
        return self.positions > 0 or self.hours > 0

    def bound_mb(self) -> int:
        return self.memory_mb or memory_mb(self.simulations)

    def search_words(self) -> str:
        return ",".join(["RootShortcut=false", f"MemoryMb={self.bound_mb()}", *self.search])

    def what(self) -> dict:
        """What the searches and the sample are: a stage run again with another of these is another reanalysis."""
        return dict(positions=self.positions, hours=self.hours, worlds=self.worlds, simulations=self.simulations,
                    search=self.search_words(), seeds=list(self.seeds), batch=self.batch, per_game=self.per_game,
                    heldout=self.heldout, source=self.source, mode=self.mode, sample_seed=self.sample_seed)

    def probed(self) -> dict:
        """What a probe's lanes hold for: the search and the caps on its lanes, whatever the sample."""
        return dict(worlds=self.worlds, simulations=self.simulations, search=self.search_words(), batch=self.batch,
                    headroom=self.headroom, lanes_per_core=self.lanes_per_core, max_lanes=self.max_lanes,
                    local_lanes=self.local_lanes)


def model_signature(model: Path) -> str:
    """What of the network decides how fast it is served and searched: its size and shape (its manifest's
    `parameters` and `config`), so a new champion of the same size is the same to a probe; its name where the
    manifest says neither."""
    m = read_json(model / "manifest.json", {}) if model is not None else {}
    if isinstance(m, dict) and (m.get("parameters") or m.get("config")):
        return f"{m.get('parameters')}:{m.get('config')}"
    return model.name if model is not None else ""


def run_name(seed: int) -> str:
    """A seed's run: its judgements in `genN/reanalyse/judge/<name>`, its ledger `genN/dist/judge-<name>` - the same
    name in every generation, so what a game of it cost before is found (`status.pace`)."""
    return f"deep-s{seed}"


class Paths:
    def __init__(self, root: Path, gen: int):
        self.gen_dir = root / f"gen{gen}"
        self.work = self.gen_dir / "reanalyse"
        self.judge = self.work / "judge"
        self.settings = self.work / "settings.json"
        self.state = self.work / "state.json"
        self.games = self.work / "games.json"
        self.corpus = root / CORPUS / f"gen{gen}"
        self.probe = root / CORPUS / PROBE


class Data:
    """A run's plans and recordings as `mining.judge_distributed` takes them (`root`: where they are here, laid out
    as a worker's bundle's `data/`; `judge`: where the merged judgements go)."""

    def __init__(self, root: Path, judge: Path):
        self.root, self.judge = root, judge

    def files(self) -> dict[str, Path]:
        return {f"data/{f.relative_to(self.root).as_posix()}": f for f in sorted(self.root.rglob("*")) if f.is_file()}


def read_json(path: Path, default=None):
    d = distributed.read_json(path) if path.exists() else None
    return default if d is None else d


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(x) for x in path.read_text(encoding="utf-8").splitlines() if x.strip()] if path.exists() else []


def write_lines(path: Path, lines: list[dict]):
    path.parent.mkdir(parents=True, exist_ok=True)
    distributed.write_atomic(path, "".join(json.dumps(x) + "\n" for x in lines))


def _shards():
    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import shards  # noqa: PLC0415

    return shards


# --- The games --------------------------------------------------------------------------------------------------------

def sources(gen_dir: Path, source: str) -> tuple[str, list[tuple[str, Path]]]:
    """Where the positions come from, as `(kind, [(name, directory)])`: the generation's own self-play when it kept
    its traces (`--selfplay-traces`), the gate's games the pair miner recorded otherwise; `source` says which, or
    `auto` the first there is. Both were recorded by the generation's engine, which is the one that stands them in
    again."""
    selfplay = gen_dir / "selfplay"
    has_traces = (selfplay / "traces").is_dir() and any((selfplay / "traces").glob("*.jsonl"))
    recorded = gen_dir / "mine" / "recorded"
    halves = [(h, recorded / h) for h in ("first", "second")
              if (recorded / h / "traces").is_dir() and any((recorded / h).glob("*.shard.zst"))]
    if source in ("auto", "selfplay") and has_traces:
        return "selfplay", [("selfplay", selfplay)]
    if source in ("auto", "gate") and halves:
        return "gate", halves
    if source == "selfplay":
        raise SystemExit(f"--reanalyse-source selfplay: {selfplay} has no traces (self-play is played with "
                         "--selfplay-traces for its games to be stood in again)")
    if source == "gate":
        raise SystemExit(f"--reanalyse-source gate: no recorded gate games in {recorded} (the mine stage records them)")
    raise SystemExit(f"nothing to reanalyse: {selfplay} has no traces (--selfplay-traces) and {recorded} holds no "
                     "recorded gate games (the mine stage's)")


def trace_seed(line: str) -> int | None:
    """A trace line's deal (`{"seed":N,...`), without parsing the game."""
    m = TRACE_SEED.match(line)
    if m:
        return int(m.group(1))
    try:
        return json.loads(line).get("seed")
    except ValueError:
        return None


def whole(line: str) -> bool:
    """Whether a trace line is a whole game's: a run killed while it wrote leaves its last line cut short."""
    return line.rstrip().endswith("}")


def traced(directory: Path) -> set[int]:
    """The deals of the games `directory` holds a trace of (`traces/*.jsonl`, a line a game; a line cut short is
    none)."""
    seeds = set()
    for f in sorted((directory / "traces").glob("*.jsonl")):
        with open(f, encoding="utf-8", errors="replace") as text:
            for line in text:
                if line.strip() and whole(line) and (seed := trace_seed(line)) is not None:
                    seeds.add(seed)
    return seeds


def recorded_games(directory: Path) -> dict[int, dict]:
    """The games `directory`'s shards hold, by deal: its shard, its turns and its decks. A shard that does not read is
    left out (the judge would not read it either)."""
    shards = _shards()
    games = {}
    for path in shards.shards(directory):
        try:
            for g in shards.scan(path)[1]:
                if g.meta.get("answers", 1) > 0:
                    games[int(g.meta["seed"])] = dict(shard=path.name, turns=g.meta.get("turns"), decks=g.meta.get("decks"))
        except Exception:  # noqa: BLE001
            continue
    return games


def list_games(srcs: list[tuple[str, Path]], scan=None, traces=None) -> list[dict]:
    """Every game that can be stood in again: in a shard and with a trace; by directory and deal."""
    out = []
    for name, directory in srcs:
        have = (traces or traced)(directory)
        for seed, g in sorted((scan or recorded_games)(directory).items()):
            if seed in have:
                out.append(dict(dir=name, seed=seed, **g))
    return out


def ordered(games: list[dict], seed: int) -> list[dict]:
    """The games in the one random order the stage draws from: the plan from its front, the probe from its end."""
    out = sorted(games, key=lambda g: (g["dir"], g["seed"]))
    random.Random(f"{seed}:games").shuffle(out)
    return out


def materialise(dest: Path, srcs: list[tuple[str, Path]], games: list[dict], link=None, log=None):
    """The recordings of `games` under `dest/recorded/<dir>/`, as the judge reads a directory: the shards that hold
    one of them (hard links where the file system has them, copies otherwise) and their traces in one file
    (`traces/games.jsonl`, the lines as they are). A judge then opens only what holds its games, here and on a worker,
    which is pushed these files and not a generation's self-play. Where several lines are one deal's (a game played
    again by another attempt), the line of the process that wrote the game's shard is the one kept. A line that is
    not a whole game's (cut short by a run that was killed, or otherwise no JSON) is skipped and said: the judge reads
    every line of the file it is given, and one bad line would lose it all."""
    link = link or _link
    by_dir = collections.defaultdict(dict)
    for g in games:
        by_dir[g["dir"]][int(g["seed"])] = g
    for name, directory in srcs:
        wanted = by_dir.get(name)
        if not wanted:
            continue
        out = dest / "recorded" / name
        (out / "traces").mkdir(parents=True, exist_ok=True)
        for shard in sorted({g["shard"] for g in wanted.values()}):
            if not (out / shard).exists():
                link(directory / shard, out / shard)
        kept: dict[int, tuple[bool, str]] = {}
        for f in sorted((directory / "traces").glob("*.jsonl")):
            writer = WRITER.search(f.name)
            with open(f, encoding="utf-8", newline="") as text:
                for line in text:
                    seed = trace_seed(line) if line.strip() else None
                    if seed not in wanted:
                        continue
                    if not sound(line):
                        if log is not None:
                            log(f"  WARNING: {f.name}: the trace of game {seed} is cut short or unreadable: skipped")
                        continue
                    by = WRITER.search(wanted[seed]["shard"])
                    own = bool(writer and by and writer.group(1) == by.group(1))
                    if own or not kept.get(seed, (False, ""))[0]:
                        kept[seed] = (own, line.rstrip("\r\n"))
        distributed.write_atomic(out / "traces" / "games.jsonl", "".join(kept[s][1] + "\n" for s in sorted(kept)))


def sound(line: str) -> bool:
    """Whether a trace line parses as the JSON object it has to be."""
    if not whole(line):
        return False
    try:
        return isinstance(json.loads(line), dict)
    except ValueError:
        return False


def _link(src: Path, dst: Path):
    try:
        os.link(src, dst)
    except OSError:
        shutil.copy2(src, dst)


# --- The plan ---------------------------------------------------------------------------------------------------------

def held_out(share: float, k: int) -> bool:
    """Whether the plan's `k`-th game (from 0; the games are in a random order already) is of the held-out slice: by
    the game, so a game's positions are all on one side, and evenly through the plan, so the slice is the share asked
    of any stretch of it (a tenth: every tenth game), however few the games."""
    return share > 0 and int(round((k + 1) * share, 9)) > int(round(k * share, 9))


def plan_positions(games: list[dict], decisions: dict, per_game: int, positions: int, heldout: float, seed: int,
                   latest: bool = False) -> list[dict]:
    """The plan: for each of `games` in turn, `per_game` of its decisions with a choice (`decisions`: by
    `(dir, deal)`, each `[decision, seat, turn]`) drawn at random (`latest`: its latest instead, the largest states),
    until there are `positions`; each with its seat and turn, and its split by the game (`held_out`). In the games'
    order."""
    out, k = [], 0
    for g in games:
        if len(out) >= positions:
            break
        have = sorted(tuple(d) for d in decisions.get((g["dir"], int(g["seed"])), []))
        if not have:
            continue
        if latest:
            picked = sorted(have, key=lambda d: (-d[2], d[0]))[:per_game]
        else:
            picked = random.Random(f"{seed}:{g['dir']}:{g['seed']}:decisions").sample(have, min(per_game, len(have)))
        split, k = "heldout" if held_out(heldout, k) else "train", k + 1
        for decision, seat, turn in sorted(picked)[: positions - len(out)]:
            out.append(dict(id=f"{g['dir']}-{int(g['seed']):012d}-{decision:04d}", dir=g["dir"], seed=int(g["seed"]),
                            decision=decision, split=split, seat=seat, turn=turn, decks=g.get("decks")))
    return out


def plan_games(plan: list[dict]) -> list[tuple[str, int]]:
    return sorted({(x["dir"], int(x["seed"])) for x in plan})


# --- Lanes ------------------------------------------------------------------------------------------------------------

def memory_mb(simulations: int) -> int:
    """The tree's bound for a search of `simulations` (`MemoryMb`): room for the largest tree measured a simulation
    (`SIMULATION_BYTES`), and never under the search's default 2 GiB. A search that reaches its bound stops short, so
    the bound is above what the budget needs; the stage counts the searches that reached it."""
    return max(2048, math.ceil(simulations * SIMULATION_BYTES / (1 << 20)))


def pick_lanes(lane_bytes: float, available_bytes: float | None, cpus: float | None, headroom: float = 0.3,
               lanes_per_core: float = 1.0, server_cores: int = SERVER_CORES, most: int | None = None) -> dict:
    """A host's lanes: as many as fit in its free memory less `headroom` at `lane_bytes` a lane, and no more than
    `lanes_per_core` a CPU less the inference server's; at least one. What limited them is named."""
    by_memory = None if not available_bytes or lane_bytes <= 0 else int(available_bytes * (1 - headroom) // lane_bytes)
    by_cpu = None if not cpus else max(1, int((cpus - server_cores) * lanes_per_core))
    limits = {k: v for k, v in (("memory", by_memory), ("cpu", by_cpu), ("the most asked for", most)) if v is not None}
    if not limits:
        return dict(lanes=1, limiter="nothing known of the host", by_memory=None, by_cpu=None)
    limiter = min(limits, key=lambda k: limits[k])
    return dict(lanes=max(1, limits[limiter]), limiter=limiter, by_memory=by_memory, by_cpu=by_cpu)


def climb(ladder, cap: int, measure, gain: float = 0.1) -> tuple[int, str]:
    """The lanes a host runs, climbing `ladder` (and `cap`, the most it may) from the bottom: `measure(lanes)` is its
    simulations a second there, and the climb stops at the first step that is not `gain` faster than the best below
    it. The best step's lanes, and what stopped the climb ("throughput", or "cap": every step was faster)."""
    steps = sorted({n for n in ladder if 1 < n < cap} | {cap})
    best, best_rate = None, 0.0
    for n in steps:
        r = measure(n)
        if best is not None and r < best_rate * (1 + gain):
            return best, "throughput"
        best, best_rate = n, r
    return best, "cap"


def gb(x) -> str:
    return "-" if x is None else f"{x / (1 << 30):.1f} GB"


# --- What was searched --------------------------------------------------------------------------------------------------

def searched(line: dict | None) -> bool:
    """Whether a judged line holds a deep search's target."""
    t = (line or {}).get("target")
    return isinstance(t, dict) and t.get("kl") is not None


def summarise(plan: list[dict], judged: dict[str, dict]) -> dict:
    """What a run's searches came to: how many of the plan's positions were searched, how often the deep search's
    first move is not the network's (`top_agree`), its KL from the prior, and how many searches stopped at their
    memory bound (their budget not spent)."""
    done = [judged[x["id"]]["target"] for x in plan if searched(judged.get(x["id"]))]
    kls = sorted(t["kl"] for t in done)
    errors = collections.Counter(str((judged.get(x["id"]) or {}).get("error") or
                                     ((judged.get(x["id"]) or {}).get("target") or {}).get("error") or "not judged")[:100]
                                 for x in plan if not searched(judged.get(x["id"])))
    return dict(planned=len(plan), searched=len(done), disagree=sum(1 for t in done if not t.get("top_agree")),
                kl_median=kls[len(kls) // 2] if kls else None, kl_mean=sum(kls) / len(kls) if kls else None,
                memory_bound=sum(1 for t in done if t.get("memory_bound")),
                simulations_median=statistics.median([t.get("simulations") or 0 for t in done]) if done else None,
                not_searched=dict(errors))


# --- The runner -------------------------------------------------------------------------------------------------------

class Reanalysis:
    """The stage for one generation. `setup` is the generation's `duels.Setup` with the workers held and whether this
    machine judges; `call(cmd, log_path)` runs one of the engine's commands and returns what it printed; `explore`
    is the generation's `cantrip-explore`."""

    def __init__(self, root: Path, gen: int, settings: Settings, model: Path, decks: Path | None, explore: Path, setup,
                 call, engine: dict | None = None, log=print, clock=time.time, judge=None):
        self.root, self.gen, self.s, self.model, self.decks, self.explore = root, gen, settings, model, decks, explore
        self.setup, self.call, self.engine, self.log, self.clock = setup, call, engine, log, clock
        self.p = Paths(root, gen)
        self.logs = self.p.gen_dir / "logs"
        self.main = Data(self.p.work / "data", self.p.judge)
        self.probing = Data(self.p.work / "probe", self.p.judge)
        self.judge_distributed = judge or (lambda *a, **k: mining.judge_distributed(*a, **k))
        self.names = [run_name(seed) for seed in self.s.seeds]
        self.kind, self.srcs = None, []
        # The information mode the positions are searched in: the recordings' own (the gate's games are recorded in
        # decklist mode whatever the line's is), which `games` reads from their shards.
        self.mode = settings.mode

    # What the stage is: one made again with other settings is another, and is not mixed into this one.

    def check_settings(self):
        now = self.s.what()
        was = read_json(self.p.settings)
        if was is not None and was != now:
            changed = [k for k in now if was.get(k) != now[k]]
            judged = any(read_json(self.p.gen_dir / "dist" / f"judge-{n}" / "ledger.json", {}).get("shards") for n in self.names)
            if judged:
                raise SystemExit(f"{self.p.work} holds a reanalysis made with other settings ({', '.join(changed)}), and "
                                 "searches of it are in: run the stage with the settings it began with, or remove that "
                                 f"directory and {self.p.gen_dir / 'dist'}'s judge-deep-* to start it over")
            self.log(f"  the settings changed ({', '.join(changed)}) and nothing was searched yet: started over")
            mining.clear(self.p.work)
        self.p.work.mkdir(parents=True, exist_ok=True)
        distributed.write_atomic(self.p.settings, json.dumps(now, indent=1))

    def state(self) -> dict:
        return read_json(self.p.state, {})

    def save_state(self, **more):
        distributed.write_atomic(self.p.state, json.dumps({**self.state(), **more}, indent=1))

    # 1. The games.

    def games(self) -> list[dict]:
        self.kind, self.srcs = sources(self.p.gen_dir, self.s.source)
        kept = read_json(self.p.games)
        if kept is not None and kept.get("source") == self.kind:
            self.mode = kept.get("mode") or self.s.mode
            return kept["games"]
        games = ordered(list_games(self.srcs), self.s.sample_seed)
        if not games:
            raise SystemExit(f"nothing to reanalyse: no game of {', '.join(str(d) for _, d in self.srcs)} has both its "
                             "shard and its trace")
        first = dict(self.srcs)[games[0]["dir"]] / games[0]["shard"]
        self.mode = (shard_header(first) or {}).get("mode") or self.s.mode
        distributed.write_atomic(self.p.games, json.dumps(dict(source=self.kind, mode=self.mode, games=games)))
        self.log(f"  {len(games)} games to draw from ({self.kind}, recorded in {self.mode} mode: "
                 f"{', '.join(str(d) for _, d in self.srcs)})")
        return games

    def decisions(self, data: Data, games: list[dict], tag: str) -> dict:
        """Each of `games`' decisions with a choice (`mine-decisions`, on their recordings under `data`)."""
        materialise(data.root, self.srcs, games, log=self.log)
        out, errors = {}, collections.Counter()
        for name in sorted({g["dir"] for g in games}):
            asked = self.p.work / f"games-{tag}-{name}.jsonl"
            said = self.p.work / f"decisions-{tag}-{name}.jsonl"
            write_lines(asked, [dict(seed=int(g["seed"])) for g in games if g["dir"] == name])
            self.call([self.explore, "mine-decisions", "--dir", data.root / "recorded" / name, "--games", asked, "--out",
                       said, "--lanes", self.s.sample_lanes, "--mode", self.mode], self.logs / f"reanalyse-decisions-{tag}-{name}.log")
            for line in read_jsonl(said):
                if "decisions" in line:
                    out[(name, int(line["seed"]))] = line["decisions"]
                else:
                    errors[str(line.get("error"))[:100]] += 1
        if errors:
            self.log(f"  games with no decisions to draw: {dict(errors)}")
        return out

    # The judgements, through the miner's distribution.

    def hosts(self) -> list[tuple[str, dict | None]]:
        """The hosts that judge, by the key their lanes are kept under: each worker by its name, this machine `local`."""
        return [(w["name"], w) for w in self.setup.workers] + ([(LOCAL, None)] if self.setup.local else [])

    def runs_of(self, name: str, plan: str, seed: int):
        """`mining.judge_distributed`'s `runs`: the one run `name`, the plan `plan` searched with `seed`, each
        position alone (no candidates: no judgement, only its target)."""
        s = self.s

        def runs(model, prior, here, judge_dir):
            h = str(here).replace("\\", "/").rstrip("/")
            dirs = [x for name_, _ in self.srcs for x in ("--dir", f"{name_}={h}/recorded/{name_}")]
            return [dict(name=name, args=[
                "mine-judge", "--plan", f"{h}/{plan}", *dirs, "--out", f"{judge_dir}/{name}", "--simulations",
                str(s.simulations), "--worlds", str(s.worlds), "--seed", str(seed), "--targets", "true", "--model", model,
                "--batch", str(s.batch), "--lanes", "1", "--mode", self.mode, "--search", s.search_words(), "--prior", prior])]
        return runs

    def judge(self, data: Data, only: str | None, name: str, plan: str, seed: int, lanes: dict[str, int], part: int,
              stop_check=None) -> dict:
        """One run across the hosts (`only`: that host alone, a probe's step): each worker at its `lanes` (by its
        name), this machine at `lanes["local"]`; its hosts' parts."""
        from . import duels  # noqa: PLC0415

        workers = [dict(w, judge_lanes=lanes.get(w["name"]) or 1) for w in self.setup.workers if only in (None, w["name"])]
        local = self.setup.local and only in (None, LOCAL)
        setup = duels.Setup(**{**self.setup.__dict__, "workers": workers, "local": local})
        try:
            done = self.judge_distributed(setup, data, None, self.model, self.decks, self.explore, lanes.get(LOCAL) or 1,
                                          1, runs=self.runs_of(name, plan, seed), data=data.files(), chunk=part,
                                          stop_check=stop_check)
        except SystemExit as e:
            # No host that answers (its worker out, every attempt failed): what the ledger accepted is kept, and a
            # later run goes on from it - a failure to run again, not one to hold the loop for (`NO_SEARCH`).
            if "no host to judge on" in str(e) or "every host has failed" in str(e):
                raise SystemExit(f"{NO_SEARCH}: {e}") from e
            raise
        return done[name]

    def ledger(self, name: str) -> dict:
        return read_json(self.p.gen_dir / "dist" / f"judge-{name}" / "ledger.json", {})

    def part_reports(self, name: str) -> list[dict]:
        """What each accepted part of run `name` said of itself (`judge-report.json`), with its host."""
        dist = self.p.gen_dir / "dist" / f"judge-{name}"
        out = []
        for part, shard in sorted((self.ledger(name).get("shards") or {}).items()):
            r = read_json(dist / "results" / part / "judge-report.json")
            if r:
                out.append(dict(r, host=shard.get("host"), part=part))
        return out

    def judged(self, name: str) -> dict[str, dict]:
        return {j["id"]: j for j in read_jsonl(self.p.judge / name / "judged.jsonl")}

    # 2. The probe.

    def probe(self, games: list[dict]) -> dict:
        """Each host's lanes: given (`--reanalyse-lanes`), or measured on the host alone (`probe_host`) and kept for
        the line (`ROOT/corpus/deep/probe.json`): a host probed for these searches is not probed again."""
        s = self.s
        hosts = self.hosts()
        if s.lanes:
            return dict(lanes={key: s.lanes for key, _ in hosts}, hosts={}, why="--reanalyse-lanes")
        kept = read_json(self.p.probe)
        again = s.reprobe and not self.state().get("reprobed")
        # What the lanes were measured for: the search and its caps, the network (by its size), the engine.
        key = dict(s.probed(), model=model_signature(self.model), engine=(self.engine or {}).get("id"))
        if kept is None or kept.get("settings") != key or again:
            # Other searches, another size of network or another engine than the ones probed for (or
            # `--reanalyse-reprobe`, once a generation): probed afresh.
            kept = dict(settings=key, lanes={}, hosts={})
            self.p.probe.parent.mkdir(parents=True, exist_ok=True)
            distributed.write_atomic(self.p.probe, json.dumps(kept, indent=1))
            self.save_state(reprobed=True)
        todo = [(key, w) for key, w in hosts if key not in kept["hosts"]]
        if todo:
            self.probe_plans(games)
        for key, w in todo:
            kept["hosts"][key] = h = self.probe_host(key, w)
            kept["lanes"][key] = h["lanes"]
            distributed.write_atomic(self.p.probe, json.dumps(kept, indent=1))
        for key, _ in hosts:
            if not any(key == k for k, _ in todo):
                self.log(f"  probe [{key}]: kept ({kept['lanes'][key]} lanes, {self.p.probe})")
        return dict(kept, lanes={key: kept["lanes"][key] for key, _ in hosts})

    def probe_games(self, games: list[dict]) -> list[dict]:
        """The games the probe draws from: the end of the order, enough for the ladder's largest step."""
        s = self.s
        return list(reversed(games))[: max(max(s.probe_ladder, default=1) * s.probe_games, 4 * s.probe_positions)]

    def probe_plans(self, games: list[dict]):
        """The probe's positions, from the end of the games' order (the plan draws from its front): the latest turns
        for the one-lane step (the largest states, so the most a lane holds), and for each step of the ladder a
        position of each of a few games a lane."""
        s = self.s
        if (self.probing.root / "plan-probe.jsonl").exists():
            return
        chosen = self.probe_games(games)
        decisions = self.decisions(self.probing, chosen, "probe")
        latest = plan_positions(chosen, decisions, 1, len(chosen), 0.0, s.sample_seed, latest=True)
        write_lines(self.probing.root / "plan-probe.jsonl",
                    sorted(latest, key=lambda x: (-x["turn"], x["id"]))[: s.probe_positions])
        each = plan_positions(chosen, decisions, 1, len(chosen), 0.0, s.sample_seed)
        for n in s.probe_ladder:
            write_lines(self.probing.root / f"plan-probe-{n}.jsonl", each[: n * s.probe_games])

    def probe_step(self, key: str, lanes: int, plan: str) -> dict:
        """One judgement of the probe's on one host alone at `lanes` (the plan `plan`, a single part): what its
        process reported and what its searches ran."""
        name = f"probe-{key}-{lanes}"
        games = len(mining.plan_games(self.probing.root / plan))
        self.judge(self.probing, key, name, plan, self.s.seeds[0], {key: lanes}, max(1, games))
        reports = self.part_reports(name)
        lines = list(self.judged(name).values())
        targets = [j["target"] for j in lines if searched(j)]
        if not reports or not targets:
            raise SystemExit(f"{NO_SEARCH}: the probe on {key} searched nothing ({self.p.judge / name / 'judged.jsonl'}): "
                             f"{[j.get('error') or (j.get('target') or {}).get('error') for j in lines][:3]}")
        seconds = sum(r.get("Seconds") or 0 for r in reports)
        simulations = sum(t.get("simulations") or 0 for t in targets)
        return dict(lanes=lanes, positions=len(targets), seconds=seconds, simulations=simulations,
                    evals=simulations / seconds if seconds else 0.0,
                    peak_bytes=max((r.get("PeakBytes") or 0 for r in reports), default=0),
                    available_bytes=next((r["AvailableBytes"] for r in reports if r.get("AvailableBytes")), None),
                    cpus=next((r["Cpus"] for r in reports if r.get("Cpus")), None),
                    tree_bytes_max=max((t.get("tree_bytes") or 0 for t in targets), default=0),
                    memory_bound=sum(1 for t in targets if t.get("memory_bound")),
                    mismatched=sum(r.get("MismatchedPositions") or 0 for r in reports))

    def probe_host(self, key: str, w: dict | None) -> dict:
        """One host's lanes. First one lane (the latest turns): a lane's memory is that process's peak, and the most
        lanes the host may run is what fits in the memory it had free less the headroom, within its CPUs
        (`pick_lanes`). Then up the ladder below that (`climb`), each step a judgement of a few positions a lane,
        measured in simulations a second: the lanes are the last step that still made the host faster - past it the
        GPU (or the CPU) is the bound, and more lanes only hold more memory."""
        s = self.s
        self.log(f"  probe [{key}]: one lane, at {s.simulations} over {s.worlds} worlds")
        one = self.probe_step(key, 1, "plan-probe.jsonl")
        if one["mismatched"]:
            raise SystemExit(f"the probe on {key}: {one['mismatched']} of its positions were stood in as another position "
                             "than the recording's (MismatchedPositions): the engine that judges is not the one that "
                             "recorded the games, and no target would be written")
        if not one["peak_bytes"]:
            raise SystemExit(f"{NO_SEARCH}: the probe's judge on {key} did not report its memory (PeakBytes: a "
                             "cantrip-explore from before it did, or a host that could not say): give --reanalyse-lanes")
        free = one["available_bytes"] or ((w or {}).get("ram_gb") or 0) * (1 << 30) or None
        cpus = one["cpus"] or (w or {}).get("cores")
        # A Linux worker's judge is pinned to one thread of each of its cores less the server's
        # (`distributed.pin_plan`), and counts only those as its CPUs: nothing more is left out for the server.
        server = 0 if w is not None and w.get("os") == "linux" and w.get("pin", True) and one["cpus"] else SERVER_CORES
        most = (s.local_lanes if w is None else s.max_lanes) or None
        cap = pick_lanes(one["peak_bytes"], free, cpus, s.headroom, s.lanes_per_core, server, most=most)
        self.log(f"    [{key}] a lane held {gb(one['peak_bytes'])} (its largest tree {gb(one['tree_bytes_max'])}, bound "
                 f"{s.bound_mb()} MB), {one['seconds'] / max(1, one['positions']):.0f} s a position; {gb(free)} free, "
                 f"{cpus} CPUs: at most {cap['lanes']} lanes (by {cap['limiter']})")
        steps = [one]
        rungs = [n for n in s.probe_ladder if (self.probing.root / f"plan-probe-{n}.jsonl").exists()]

        def measure(lanes: int) -> float:
            # The step's positions: the rung's own, or (the cap, between two rungs) the next rung's.
            rung = min((n for n in rungs if n >= lanes), default=max(rungs, default=None))
            step = self.probe_step(key, lanes, f"plan-probe-{rung}.jsonl" if rung else "plan-probe.jsonl")
            steps.append(step)
            self.log(f"    [{key}] {lanes} lanes: {step['evals']:.0f} simulations a second ({step['positions']} positions "
                     f"in {step['seconds']:.0f} s), peak {gb(step['peak_bytes'])}")
            return step["evals"]

        lanes, limiter = climb(rungs, cap["lanes"], measure, s.probe_gain) if cap["lanes"] > 1 else (1, "cap")
        best = next(x for x in reversed(steps) if x["lanes"] == lanes)
        self.log(f"  probe [{key}]: {lanes} lanes ({limiter if limiter != 'cap' else cap['limiter']}), "
                 f"{best['evals']:.0f} simulations a second")
        return dict(lanes=lanes, limiter=limiter if limiter != "cap" else cap["limiter"], evals=best["evals"], cap=cap,
                    # A lane's memory, as the largest any step's lane held: what the judgements' own parts are held to.
                    lane_bytes=max(x["peak_bytes"] / x["lanes"] for x in steps), available_bytes=free, cpus=cpus,
                    server_cores=server, steps=steps, memory_mb=s.bound_mb(), probed_at=self.gen)

    def rate(self, probe: dict) -> float:
        """The hosts' simulations a second together: measured by the probe, or (lanes given) what a lane is taken to
        run and a host's GPU to serve."""
        total = 0.0
        for key, lanes in probe["lanes"].items():
            measured = (probe.get("hosts") or {}).get(key, {}).get("evals")
            total += measured or min(lanes * LANE_EVALS, SERVER_EVALS)
        return total

    # 3. The plan.

    def plan(self, games: list[dict], probe: dict) -> list[dict]:
        s = self.s
        path = self.main.root / "plan.jsonl"
        if path.exists():
            plan = read_jsonl(path)
            self.log(f"  plan: kept ({len(plan)} positions, {path})")
            return plan
        positions = s.positions or 1 << 60
        if s.hours > 0:
            # What the budget buys at the probe's rate, and a fifth: the budget is what stops the judging.
            buys = max(1, math.ceil(1.2 * s.hours * 3600 * self.rate(probe) / s.simulations))
            if buys < positions:
                self.log(f"  the {s.hours:g} h budget buys about {buys / 1.2:.0f} searches at {self.rate(probe):.0f} "
                         f"simulations a second: {buys} positions planned" + (f", not {s.positions}" if s.positions else ""))
                positions = buys
        # The games drawn from: what the positions need, and some more (a game with fewer decisions, one that does
        # not replay); never the probe's.
        probed = set()
        if (self.probing.root / "plan-probe.jsonl").exists():
            probed = {(g["dir"], g["seed"]) for g in self.probe_games(games)}
        pool = [g for g in games if (g["dir"], g["seed"]) not in probed] or games
        chosen = pool[: math.ceil(positions / max(1, s.per_game) * 1.15) + 2]
        plan = plan_positions(chosen, self.decisions(self.main, chosen, "plan"), s.per_game, positions, s.heldout, s.sample_seed)
        if not plan:
            raise SystemExit(f"no position to reanalyse: none of {len(chosen)} games has a decision with a choice that "
                             f"replays (their logs: {self.logs}/reanalyse-decisions-*.log)")
        write_lines(path, plan)
        split = collections.Counter(x["split"] for x in plan)
        self.log(f"  plan: {len(plan)} positions of {len(plan_games(plan))} games ({dict(split)}), each searched at "
                 f"{s.simulations} over {s.worlds} worlds ({s.search_words()}), seeds {list(s.seeds)}")
        if len(plan) < positions < 1 << 60:
            self.log(f"  only {len(plan)} of the {positions} positions wanted: the games hold no more")
        return plan

    # 4. The judgements.

    def judge_all(self, plan: list[dict], probe: dict) -> dict:
        s = self.s
        lanes = dict(probe["lanes"])
        hosts = probe.get("hosts") or {}
        spent = {}
        for k, (seed, name) in enumerate(zip(s.seeds, self.names)):
            plan_file = "plan.jsonl"
            mine_ = plan
            if k > 0:
                # A later seed searches what the first did: the positions of the games it searched.
                first = self.judged(self.names[0])
                games = {(x["dir"], x["seed"]) for x in plan if searched(first.get(x["id"]))}
                mine_ = [x for x in plan if (x["dir"], x["seed"]) in games]
                plan_file = f"plan-{name}.jsonl"
                write_lines(self.main.root / plan_file, mine_)
            if name in self.state().get("judged", {}):
                self.log(f"  judge {name}: kept ({len(self.judged(name))} positions)")
            elif not mine_:
                self.save_state(judged={**self.state().get("judged", {}), name: dict(seconds=0.0)})
            else:
                part = s.part_games or max(1, max(lanes.values()))
                rate = self.rate(dict(probe, lanes=lanes))
                about = len(mine_) * s.simulations / rate / 3600 if rate else None
                budget = s.hours * 3600 if s.hours > 0 and k == 0 else None
                before = float(self.state().get("spent", {}).get(name, 0.0))
                self.log(f"  judge {name}: {len(mine_)} positions, lanes {lanes}, parts of {part} games"
                         + (f"; about {about:.1f} h" if about else "")
                         + (f"; budget {s.hours:g} h ({before / 3600:.2f} h spent)" if budget else ""))
                started = self.clock()

                def spend(_ledger=None, name=name, before=before, started=started, budget=budget):
                    now = before + self.clock() - started
                    self.save_state(spent={**self.state().get("spent", {}), name: now})
                    if budget is not None:
                        # Beside the ledger, for whoever projects the run's end (`status.BUDGET`): it ends with
                        # its budget, whatever is left of its games then.
                        said = self.p.gen_dir / "dist" / f"judge-{name}" / status.BUDGET
                        said.parent.mkdir(parents=True, exist_ok=True)
                        distributed.write_atomic(said, json.dumps(dict(seconds=budget, spent=now, at=self.clock())))
                    if budget is not None and now >= budget:
                        return f"the time budget ({s.hours:g} h) is spent"
                    return None

                try:
                    parts = self.judge(self.main, None, name, plan_file, seed, lanes, part, stop_check=spend)
                finally:
                    spend()
                took = float(self.state().get("spent", {}).get(name, 0.0))
                self.save_state(judged={**self.state().get("judged", {}), name: dict(seconds=took, hosts=parts)})
                self.log(f"  judge {name}: done in {took / 3600:.2f} h on {parts}")
            reports = self.part_reports(name)
            closed = self.ledger(name).get("closed")
            spent[name] = dict(seconds=self.state().get("judged", {}).get(name, {}).get("seconds"),
                               hosts=self.state().get("judged", {}).get(name, {}).get("hosts"),
                               parts=len(reports), closed=closed,
                               mismatched=sum(r.get("MismatchedPositions") or 0 for r in reports),
                               peak_bytes=max((r.get("PeakBytes") or 0 for r in reports), default=0),
                               **summarise(mine_, self.judged(name)))
            # A host whose lanes held more over a whole part than the probe's did runs fewer from here on, and in
            # the generations after (the probe's record is the line's).
            for key, h in hosts.items():
                host_name = self.setup.local_name if key == LOCAL else key
                # A host whose free memory is no longer what its probe saw (other work on it, memory added) is
                # probed again the next time: the lanes that fit are another number.
                free = [r["AvailableBytes"] for r in reports if r.get("AvailableBytes") and r.get("host") == host_name]
                was = h.get("available_bytes")
                if free and was and abs(min(free) - was) > FREE_DRIFT * was:
                    kept = read_json(self.p.probe)
                    if kept and key in kept.get("hosts", {}):
                        self.log(f"    [{key}] had {gb(min(free))} free, {gb(was)} when it was probed: probed again next time")
                        kept["hosts"].pop(key)
                        kept["lanes"].pop(key, None)
                        distributed.write_atomic(self.p.probe, json.dumps(kept, indent=1))
                held = max((r["PeakBytes"] / r["Lanes"] for r in reports
                            if r.get("PeakBytes") and r.get("Lanes") and r.get("host") == host_name), default=0)
                if key in lanes and held > (h.get("lane_bytes") or held):
                    fit = pick_lanes(held, h.get("available_bytes"), h.get("cpus"), s.headroom, s.lanes_per_core,
                                     h.get("server_cores", SERVER_CORES))["lanes"]
                    if fit < lanes[key]:
                        self.log(f"    [{key}] a lane of {name} held {gb(held)}, more than the probe's "
                                 f"{gb(h.get('lane_bytes'))}: {fit} lanes from here, not {lanes[key]}")
                        lanes[key] = fit
                        kept = read_json(self.p.probe)
                        if kept and key in kept.get("lanes", {}):
                            kept["lanes"][key] = fit
                            kept["hosts"][key] = dict(kept["hosts"][key], lanes=fit, lane_bytes=held)
                            distributed.write_atomic(self.p.probe, json.dumps(kept, indent=1))
        return spent

    # 5. The corpus.

    def corpus(self, plan: list[dict], spent: dict, probe: dict) -> dict:
        s, dest = self.s, self.p.corpus
        producer = f"deep:{self.model.name}@gen{self.gen}"
        by_id = {x["id"]: x for x in plan}
        made = {}
        mining.clear(dest)
        dest.mkdir(parents=True)
        header = None
        for seed, name in zip(s.seeds, self.names):
            judged = self.judged(name)
            made[f"seed-{seed}"] = counts = {}
            for split in ("train", "heldout"):
                keep = [dict(dir=by_id[i]["dir"], seed=by_id[i]["seed"], decision=by_id[i]["decision"], id=i)
                        for i, j in sorted(judged.items()) if i in by_id and by_id[i]["split"] == split and searched(j)]
                source = self.p.judge / name / split
                counts[split] = 0
                if not keep or not source.is_dir():
                    continue
                listed = self.p.work / f"keep-{name}-{split}.jsonl"
                write_lines(listed, keep)
                out = dest / f"seed-{seed}" / split
                text = self.call([self.explore, "mine-filter", "--from", source, "--keep", listed, "--out", out, "--producer",
                                  producer], self.logs / f"reanalyse-filter-{name}-{split}.log")
                written = _decisions(text)
                if out.is_dir() and any(out.glob("*.shard.zst")):
                    mining.write_effort(out, s.simulations, s.worlds)
                    counts[split] = written if written is not None else len(keep)
                    header = header or shard_header(next(iter(sorted(out.glob("*.shard.zst")))))
        total = sum(c["train"] + c["heldout"] for c in made.values())
        if total == 0:
            accepted = any(self.ledger(name).get("shards") for name in self.names)
            raise SystemExit(("" if accepted else f"{NO_SEARCH}: ") + f"the reanalysis wrote no target: {json.dumps({n: x.get('not_searched') for n, x in spent.items()})} "
                             f"(mismatched positions: {sum(x.get('mismatched') or 0 for x in spent.values())}; a replay that "
                             "is not the recording writes none - the engine that judges has to be the recordings')")
        # The held-out games, by key: what later windows leave out of the self-play they hold (`KEYS`).
        keys = heldout_keys(split_dirs(dest, "heldout"))
        distributed.write_atomic(dest / KEYS, json.dumps(sorted(keys)))
        first = spent.get(self.names[0], {})
        record = dict(
            generation=self.gen, complete=True, made=time.strftime("%Y-%m-%d %H:%M:%S"), producer=producer,
            judge=self.model.name, engine=self.engine, source=self.kind, mode=self.mode, simulations=s.simulations,
            worlds=s.worlds, memory_mb=s.bound_mb(), search=s.search_words(), batch=s.batch, seeds=list(s.seeds),
            per_game=s.per_game, heldout=s.heldout, planned=len(plan), positions=total, directories=made,
            heldout_games=len(keys),
            schema=(header or {}).get("schema"), encoder_hash=(header or {}).get("encoder_hash"),
            seconds=sum(x.get("seconds") or 0 for x in spent.values()),
            seconds_a_position=(first.get("seconds") or 0) / first["searched"] if first.get("searched") else None,
            runs=spent, lanes=probe.get("lanes"),
            note="Policy-only targets of deep searches of recorded positions (ai/cantrip_ai/reanalyse.py), a directory a "
                 "seed and split, each with its search's effort.json. Kept for good: retention never prunes it.")
        distributed.write_atomic(dest / RECORD, json.dumps(record, indent=1))
        self.log(f"  corpus: {total} targets in {dest} ({json.dumps(made)}; producer {producer})")
        return record

    def run(self) -> dict:
        """The whole stage, each step kept where it was made; what the stage's record says."""
        s = self.s
        kept = read_json(self.p.corpus / RECORD)
        self.p.work.mkdir(parents=True, exist_ok=True)
        if kept and kept.get("complete") and read_json(self.p.settings) in (None, s.what()):
            self.log(f"  the corpus of this generation is made already ({self.p.corpus}): kept")
            return numbers(kept)
        self.check_settings()
        games = self.games()
        if not self.hosts():
            raise SystemExit(f"{NO_SEARCH}: no host to judge on (no worker held, and this machine left out)")
        probe = self.probe(games)
        plan = self.plan(games, probe)
        spent = self.judge_all(plan, probe)
        for name, x in spent.items():
            self.log(f"  {name}: searched {x['searched']} of {x['planned']}"
                     + (f" ({x['closed']})" if x.get("closed") else "")
                     + f"; the deep choice is not the network's in {x['disagree']}"
                     + (f", KL median {x['kl_median']:.3f}" if x.get("kl_median") is not None else "")
                     + (f"; {x['memory_bound']} searches stopped at their memory bound (raise --reanalyse-memory-mb)"
                        if x.get("memory_bound") else "")
                     + (f"; {x['mismatched']} positions were not the recording's" if x.get("mismatched") else ""))
        return numbers(self.corpus(plan, spent, probe))


def _decisions(text: str) -> int | None:
    """How many decisions `mine-filter` said it wrote (its summary: a JSON object, the last thing it prints)."""
    at = len(text)
    while (at := text.rfind("{", 0, at)) >= 0:
        try:
            return int(json.JSONDecoder().raw_decode(text[at:])[0]["Decisions"])
        except (ValueError, KeyError, TypeError):
            continue
    return None


def shard_header(shard: Path) -> dict | None:
    try:
        return _shards().header(shard)
    except Exception:  # noqa: BLE001
        return None


def numbers(record: dict) -> dict:
    """The stage's record in `generation.json`, from the corpus's own."""
    first = next(iter((record.get("runs") or {}).values()), {})
    return dict(corpus=(CORPUS / f"gen{record['generation']}").as_posix(), judge=record.get("judge"), source=record.get("source"),
                simulations=record.get("simulations"), worlds=record.get("worlds"), seeds=record.get("seeds"),
                planned=record.get("planned"), positions=record.get("positions"), directories=record.get("directories"),
                searched=first.get("searched"), disagree=first.get("disagree"), kl_median=first.get("kl_median"),
                memory_bound=first.get("memory_bound"), mismatched=first.get("mismatched"), closed=first.get("closed"),
                seconds_judging=record.get("seconds"), seconds_a_position=record.get("seconds_a_position"),
                lanes=record.get("lanes"), judged_on=first.get("hosts"))


# --- The corpus, read ---------------------------------------------------------------------------------------------------

def corpus_records(root: Path, before: int | None = None) -> list[tuple[int, Path, dict]]:
    """The corpus's finished generations (`ROOT/corpus/deep/genN/CORPUS.json`, complete), oldest first; with `before`,
    only those of earlier generations than it."""
    out = []
    base = root / CORPUS
    for d in base.iterdir() if base.is_dir() else []:
        m = re.fullmatch(r"gen(\d+)", d.name)
        record = read_json(d / RECORD) if m and d.is_dir() else None
        if record and record.get("complete") and (before is None or int(m.group(1)) < before):
            out.append((int(m.group(1)), d, record))
    return sorted(out, key=lambda x: x[0])


def split_dirs(directory: Path, split: str) -> list[Path]:
    """A corpus generation's directories of one split, a seed each, that hold shards."""
    return [d / split for d in sorted(directory.glob("seed-*")) if (d / split).is_dir() and any((d / split).glob("*.shard.zst"))]


def heldout_keys(directories: list[Path]) -> set[int]:
    """The games (`dataset.source_key`: a reanalysed game keeps its source's) the shards of `directories` hold. A
    shard that does not read is an error: a held-out game left unnamed would be trained on."""
    from . import dataset  # noqa: PLC0415 (numpy: the driver's stages import it only where they need it)

    shards = _shards()
    keys = set()
    for d in directories:
        for path in shards.shards(d):
            try:
                keys.update(dataset.source_key(g.meta) for g in shards.scan(path)[1])
            except Exception as e:  # noqa: BLE001
                raise SystemExit(f"{path}: the held-out games' keys could not be read ({e})") from e
    return keys


def exclusions(root: Path) -> list[Path]:
    """Every corpus generation's list of held-out games (`KEYS`): what a training run leaves out of every directory
    it reads. All of them, this generation's own too where it exists (a retrain's): a game held out is held out of
    every training from the generation that made it on."""
    return [d / KEYS for _, d, _ in corpus_records(root) if (d / KEYS).exists()]


def policy_targets(directory: Path) -> int:
    """How many policy targets a directory's games hold, counted by decision: each game's searched decisions
    (`root_values`) that are no fast search's (`fast_decisions`, playout-cap randomisation: value targets alone). A
    reanalysed game's are all its decisions. What the corpus's share of the policy-target weight is counted against
    - decisions on both sides, never a self-play's `visit_samples` (every sub-step of every search, the fast ones
    too). A shard that does not read counts nothing."""
    shards = _shards()
    total = 0
    for path in shards.shards(directory) if Path(directory).is_dir() else []:
        try:
            for g in shards.scan(path)[1]:
                fast = {int(d) for d in g.meta.get("fast_decisions") or []}
                total += len({int(p[0]) for p in g.meta.get("root_values") or []} - fast)
        except Exception:  # noqa: BLE001
            continue
    return total


def readable(record: dict, like: dict | None = None) -> str | None:
    """Why training cannot read a corpus generation, or None. Its encoding by the loader's own rule (`dataset.py`:
    the encoding of `ai/encoding.json`, or an earlier one it only added to, `dataset.UPGRADABLE`, which it reads as
    this one); its information mode against this generation's own data (`like`: a shard header of it - a window
    holds one mode)."""
    from . import dataset  # noqa: PLC0415

    schema, encoder = record.get("schema"), record.get("encoder_hash")
    now = (dataset.LAYOUT["schema"], dataset.encoder_hash())
    if schema is not None and encoder is not None and (schema, encoder) != now \
            and dataset.UPGRADABLE.get((schema, encoder)) != now[0]:
        return f"its encoding is {schema}/{encoder}, which does not read as this one ({now[0]}/{now[1]})"
    if like and record.get("mode") is not None and like.get("mode") is not None and record["mode"] != like["mode"]:
        return f"its mode is {record['mode']}, this generation's {like['mode']}"
    return None


def corpus_dirs(root: Path, gen: int, full_simulations: int, weight: float = 1.0, decay: float = 0.85, like: dict | None = None,
                log=print, share: float = 0.25, window_targets: float | None = None) -> tuple[list[str], list[str]]:
    """The corpus generation `gen` trains on, as `train.py --data` takes it (`DIR:WEIGHT`), and what was left out
    (a line each, to be flagged): every finished generation's before it, whole, each `train` directory at its
    search's simulations over self-play's full search's (`effort.json`: a deeper search's targets count for more, as
    the mined targets do), times `weight`, times `decay` for each generation of age (1: none).

    Then all of them scaled down together, if need be, so the corpus makes at most `share` of the policy-target
    weight beside the window's (`window_targets`: its self-play's policy targets times their weights, counted by
    decision as the corpus's are, `policy_targets`) - the mined floor's normalisation, as a cap: the corpus's weighted
    targets over the corpus's and the window's together. Within the cap the generations keep their proportions. With
    nothing known of the window (None or 0) there is no cap."""
    found, left = [], []
    for g, directory, record in corpus_records(root, before=gen):
        if why := readable(record, like):
            left.append(f"the deep corpus of gen{g} is not trained on: {why}")
            continue
        age = gen - 1 - g
        for d in split_dirs(directory, "train"):
            effort = read_json(d / mining.EFFORT_FILE, {})
            w = effort.get("simulations", full_simulations) / max(full_simulations, 1) * weight * decay ** age
            n = (record.get("directories") or {}).get(d.parent.name, {}).get("train")
            found.append((d, w, n if n is not None else policy_targets(d), age))
    total = sum(w * n for _, w, n, _ in found)
    scale = 1.0
    if window_targets and 0 < share < 1 and total > share / (1 - share) * window_targets:
        scale = share / (1 - share) * window_targets / total
    for d, w, n, age in found:
        log(f"  corpus: {d} - {n} targets at {w * scale:.4g} (effort and weight {w / decay ** age if decay else w:.4g}"
            + (f", age {age}: x{decay ** age:.3g}" if decay != 1.0 else "") + (f", capped x{scale:.3g}" if scale < 1 else "") + ")")
    if found:
        of = f"{total * scale / (total * scale + window_targets):.1%} of the policy-target weight" if window_targets else \
            "the window's policy targets are not known: no cap"
        log(f"  corpus: {sum(n for _, _, n, _ in found)} targets, {of}"
            + (f" (capped at --corpus-share {share:g}: every weight x{scale:.3g})" if scale < 1 else "")
            + (f" ({window_targets:.0f} window targets weighted)" if window_targets else ""))
    return [f"{d}:{w * scale:.10g}" for d, w, _, _ in found if w * scale > 0], left


def heldout_dirs(root: Path, gen: int, like: dict | None = None) -> list[Path]:
    """The corpus's held-out slices of the generations before `gen`: the deep targets of games no training reads -
    not as corpus (they are in no `train` directory) and not as the self-play they were drawn from, which the
    windows that hold their generation leave them out of (`exclusions`, `train.py --exclude-games`). A corpus
    generation made before its games were named (no `KEYS`) is no clean measure and is left out."""
    return [d for _, directory, record in corpus_records(root, before=gen)
            if readable(record, like) is None and (directory / KEYS).exists() for d in split_dirs(directory, "heldout")]
