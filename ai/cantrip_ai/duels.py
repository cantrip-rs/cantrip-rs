"""Distributed duels (docs/ai-training.md, "Distributed duels"): a BotLab duel - the gate, the network duel, a ladder
duel, the pair miner's replays - played by this machine and the workers, the games split among them by number, and
summarised over all of them as one machine's duel is.

A duel's game is a function of its number and the duel's settings alone (its deal from `--seed` and the number, its
seats and decks from the number, its searches seeded by the number: BotLab since 2026-11-16), so any subset of the
games played anywhere is those games, and a duel is split as distributed self-play splits its games: the same
coordinator (`distributed.Coordinator`), ledger, hosts, bundles and supervisors, with a `Kind` of its own (`DUEL`). Each
host runs BotLab's grant mode (`duel --grant FILE --out DIR`: the games its grant file grants, as it grows, until it
says `end`) beside an inference server of its own serving the duel's models with the same arguments on every host
(batch shapes and precision: a game's evaluations must not depend on where it is played, beyond the hardware's own
rounding); its results come back in closed files of a few games each (`*.results.tsv`), every line one game, each
held to the duel's header (`duel --print-header`: its seats, seeds, decks, pairings and models) and accepted once.
Pairs (games 2k and 2k + 1) are granted whole, so a pair is played on one host. A game that does not finish is played
once more elsewhere and, failing again, is the duel's failure, as on one machine.

At the end the accepted results are merged into the duel's results file (`genN/logs/NAME-results.tsv`, as one machine's
`--results` writes it, in the order the games were taken in) and BotLab summarises it (`duel --summarize`): the same
lines a duel prints at its end, from the same counting code, which the driver reads (`generation.parse_gate`). A duel's
sequential stop (`--sprt`, `--sprt-paired`, `--interval-stop`) is the driver's here: after each poll it summarises the
results so far, in the order they came, and once a test has its verdict nothing more is granted, every host is asked
to stop starting games, and the games in play finish and count, as one machine stops.

The pair miner's replays (`REPLAY`) are duels whose games are each played again on the host that played them in the
gate (`affinity`: a game replays exactly only on that hardware), the recordings beside the results in one closed
directory a host (`RUN.replay`, BotLab's grant mode with `--ai-record`); a host that is gone has its games replayed by
the others, which the miner's check of each replay against the gate then keeps or leaves out.
"""

from __future__ import annotations

import contextlib
import json
import os
import shutil
import subprocess
import time
from dataclasses import dataclass, field, replace
from pathlib import Path

from . import distributed

# A duel's part on a host: BotLab's grant mode (`Grant.cs`).
DUEL = distributed.Kind(name="duel", out="duel", outputs="*.results.tsv", status="duel-status.json",
                        report="duel-report.json", stop="duel.stop", runaways=None, unit=2)
# A replay's part: the results and the recordings in one directory, closed once the recorder has written them.
REPLAY = replace(DUEL, name="replay", outputs="*.replay", dirs=True, unit=1)
# The columns of a results file (`GrantRun.Columns`, BotLab's `--results`).
COLUMNS = "game\tpair\tshuffle\ta_seat\tdeck1\tdeck2\twinner\ta_points\tstarter\tturns"
# What a Linux worker builds for a duel (`distributed.LINUX_BUILD`), and for a replay the recording native host.
LINUX_BINARIES = ("cantrip-ai-host", "cantrip-native-host", "cantrip-infer")
LINUX_RECORD = ("cantrip-native-host-record",)


# --- Results ------------------------------------------------------------------------------------------------------


def results_file(path: Path) -> Path:
    """A results output's file: the output itself, or a replay directory's `results.tsv`."""
    return path / "results.tsv" if path.is_dir() else path


def read_rows(path: Path) -> tuple[dict | None, list[str]]:
    """A results output's header (None without one) and its result lines, in order."""
    header, rows = None, []
    for line in results_file(path).read_text(encoding="utf-8").splitlines():
        if line.startswith("# "):
            header = json.loads(line[2:])
        elif line.strip() and line != COLUMNS:
            if len(line.split("\t")) < 10:
                raise ValueError(f"not a result line: {line!r}")
            rows.append(line)
    return header, rows


def game_of(row: str) -> int:
    return int(row.split("\t", 1)[0])


def finished(row: str) -> bool:
    return row.split("\t")[6] != "-"


def read_results(path: Path) -> tuple[dict, list[int], dict]:
    """A results output as the coordinator takes it (`Coordinator.take`): its header, the games that finished in it
    (one that did not is its run's to answer for: it goes back once, as self-play's failed game), and what the stage
    counts of it."""
    header, rows = read_rows(path)
    if header is None:
        raise ValueError("no header line")
    done = [game_of(r) for r in rows if finished(r)]
    return header, done, dict(games=len(done), failed=len(rows) - len(done))


def merged(ledger: distributed.Ledger, directory: Path) -> list[str]:
    """The duel's results, every game once: each accepted output's finished games in the order the outputs were
    accepted, and each game dropped (it did not finish twice) by its last unfinished line, as one machine writes it."""
    rows, failed = [], {}
    for name in ledger.shards:
        _, lines = read_rows(directory / name)
        for r in lines:
            g = game_of(r)
            if finished(r):
                if ledger.done.get(g) == name:
                    rows.append(r)
            else:
                failed[g] = r
    rows += [failed[g] for g in sorted(ledger.dropped) if g in failed]
    return rows


def write_results(rows: list[str], path: Path):
    """A results file as BotLab's `--results` writes it: the columns, then a line a game."""
    distributed.write_atomic(path, COLUMNS + "\n" + "".join(r + "\n" for r in rows))


# --- Bundles ------------------------------------------------------------------------------------------------------


def windows_files(botlab: Path, ai_host: Path, native: Path, infer: Path, ort_dir: Path | None, cuda_dir: Path | None,
                  cuda_dlls, models: list[Path], decks: Path | None, pairings: Path | None,
                  record_native: Path | None = None, serve: bool = True) -> dict[str, Path]:
    """A Windows worker's duel bundle: BotLab (self-contained, `botlab/`), the native host (and the recording one, for
    a replay); with neural seats (`models`) the AI host and the DLLs beside it and onnxruntime's, and with a server
    (`serve`) cantrip-infer and CUDA's libraries, as self-play's bundle has them; the models, decklists and pairings."""
    files = {"botlab/cantrip-botlab.exe": botlab, "bin/cantrip-native-host.exe": native}
    if record_native is not None:
        files["bin/cantrip-native-host-record.exe"] = record_native
    if models:
        if ort_dir is None:
            raise SystemExit("a Windows worker's neural duel needs onnxruntime (--ort-dylib or ORT_DYLIB_PATH)")
        files["bin/cantrip-ai-host.exe"] = ai_host
        for p in sorted(ai_host.parent.glob("*.dll")):
            files[f"bin/{p.name}"] = p
        for n in distributed.ORT_DLLS:
            files[f"bin/{n}"] = ort_dir / n
    if models and serve:
        if cuda_dir is None:
            raise SystemExit("a Windows worker's inference server needs CUDA's libraries (CANTRIP_CUDA_LIBS, or PyTorch's)")
        files["bin/cantrip-infer.exe"] = infer
        for n in cuda_dlls:
            files[f"bin/{n}"] = cuda_dir / n
    return distributed._with_data(files, models, decks, None, pairings)


def linux_files(source: Path, botlab: Path, models: list[Path], decks: Path | None, pairings: Path | None) -> dict[str, Path]:
    """A Linux worker's duel bundle: the engine's source, which it builds its binaries from (`LINUX_BINARIES`), and
    BotLab self-contained for linux-x64; the same models, decklists and pairings."""
    return distributed._with_data({"src.tar.gz": source, "botlab/cantrip-botlab": botlab}, models, decks, None, pairings)


# --- A duel -------------------------------------------------------------------------------------------------------


@dataclass
class Setup:
    """What every distributed duel of a generation shares: where it is, the workers, the binaries and libraries a
    bundle is made of, and the settings each host plays at."""
    root: Path
    gen: int
    gen_dir: Path
    repo: Path
    workers: list[dict]
    local: bool
    local_name: str
    botlab_local: list            # the command that runs BotLab here (dotnet and its dll)
    botlab: dict                  # {"windows": exe, "linux": exe}: self-contained, for the workers
    native: Path
    record_native: Path
    ai_host: Path
    infer: Path
    ort_dir: Path | None
    cuda_dir: Path | None
    cuda_dlls: tuple
    server_args: list[str]        # every host's cantrip-infer of a duel: the same
    local_parallel: int
    worker_parallel: int
    server: object = None         # given the models, a context whose value has `.address`: this machine's server
    lease: float = 900.0
    poll: float = 30.0
    timeout: float = 600.0
    chunk: int = 20
    log: object = print
    cache: dict = field(default_factory=dict)
    source: object = None         # the engine's source tar for a Linux worker, when the generation pins one (a callable)


def substitute(args: list, paths: dict[str, str]) -> list[str]:
    """`args` as a worker runs them: every path of this machine's (`paths`' keys) in its bundle's place."""
    out = []
    for a in args:
        s = str(a)
        out.append(paths.get(s, s))
    return out


def run(setup: Setup, name: str, args: list, models: list[Path], games: int, seed: int, decks: Path | None,
        pairings: Path | None, kind: distributed.Kind = DUEL, only=None, affinity: dict[int, str] | None = None,
        stops: list | None = None, record: str | None = None, env: dict | None = None,
        restart_stale: bool = False) -> tuple[distributed.Ledger, Path, list[str], float]:
    """Duel `name` (`args`: BotLab's duel arguments, with `--games` and `--seed`, but no `--parallel`, `--results` or
    `--ai-server`) played across the hosts to the end: its ledger, its results directory, its merged result lines and
    the seconds this run took. `stops` are the duel's sequential-stop arguments (the driver's here, not BotLab's);
    `only` the games to play of the `games`; `affinity` each game's host; `record` (a replay) the recordings' name
    within each host's output. With neural seats every host evaluates on an inference server of its own (this
    machine's `setup.server`), or, without one here, none does: a duel's hosts evaluate alike. With `restart_stale`
    (a replay, made again for a new selection) a ledger of another run's is started over rather than refused."""
    log = setup.log
    dist_dir = setup.gen_dir / "dist" / name
    out = dist_dir / "results"
    out.mkdir(parents=True, exist_ok=True)
    local_args = [str(a) for a in args]
    header = print_header(setup, local_args, env)
    tag = distributed.root_tag(setup.root)
    by_os = {w.get("os", "windows") for w in setup.workers}
    bundles: dict[str, tuple] = {}
    serve = bool(models) and setup.server is not None
    if "windows" in by_os:
        files = windows_files(setup.botlab["windows"], setup.ai_host, setup.native, setup.infer, setup.ort_dir,
                              setup.cuda_dir, setup.cuda_dlls, models, decks, pairings,
                              setup.record_native if record else None, serve)
        bundles["windows"] = (files, *reversed(distributed.bundle_manifest(files, setup.cache)))
    if "linux" in by_os:
        source = setup.source() if setup.source is not None else distributed.source_tar(setup.repo, setup.root / "dist" / "src")
        files = linux_files(source, setup.botlab["linux"],
                            models, decks, pairings)
        bundles["linux"] = (files, *reversed(distributed.bundle_manifest(files, setup.cache)))
    identity = dict(header=header, server_args=" ".join(setup.server_args) if serve else None, games=games, seed=seed,
                    models=distributed.files_hash([p for m in models for p in m.iterdir() if p.is_file()
                                                   and (p.name == "manifest.json" or p.suffix == ".onnx")], setup.cache),
                    stops=[str(s) for s in stops or []])
    hosts_identity = {w["name"]: dict(bundle=bundles[w.get("os", "windows")][2]) for w in setup.workers}
    if setup.local:
        hosts_identity[setup.local_name] = dict(botlab=distributed.sha256(Path(setup.botlab_local[-1]), setup.cache)[:16]
                                                if Path(str(setup.botlab_local[-1])).exists() else None)

    def job(host, attempt, d) -> dict:
        """A worker's part: BotLab's grant mode from the bundle, beside cantrip-infer serving the duel's models."""
        w, b = host.w, host.bundle_dir()
        linux = w.get("os") == "linux"
        exe = "" if linux else ".exe"
        paths = {str(m): f"{b}/model/{m.name}" for m in models}
        if decks is not None:
            paths[str(decks)] = f"{b}/decks"
        if pairings is not None:
            paths[str(pairings)] = f"{b}/pairings.txt"
        parallel = w.get("duel_parallel") or setup.worker_parallel
        remote = substitute(local_args, paths) + ["--parallel", str(parallel), "--grant", f"{d}/grant.txt",
                                                    "--host", host.name, "--out", f"{d}/{kind.out}"]
        native = "cantrip-native-host-record" if record else "cantrip-native-host"
        environment = {"CANTRIP_ROOT": b, "CANTRIP_NATIVE_HOST": f"{b}/bin/{native}{exe}",
                       "CANTRIP_AI_HOST": f"{b}/bin/cantrip-ai-host{exe}", "CANTRIP_PROGRESS": "0", **(env or {})}
        if not linux and models:
            environment.update(ORT_DYLIB_PATH=f"{b}/bin/onnxruntime.dll", CANTRIP_CUDA_LIBS=f"{b}/bin")
        out_job = dict(selfplay=f"{b}/botlab/cantrip-botlab{exe}", selfplay_args=remote,
                       recover_args=["duel", "--recover", "--out", f"{d}/{kind.out}"], env=environment,
                       infer=f"{b}/bin/cantrip-infer{exe}" if serve else None, server_flag="--ai-server",
                       lanes_flag="--parallel",
                       infer_args=["serve", *[x for m in models for x in ("--model", f"{b}/model/{m.name}")], "--port", "0",
                                   "--ready-file", f"{d}/infer.ready", "--device", str(w.get("device", 0)),
                                   *setup.server_args] if serve else [],
                       record=dict(server_args=" ".join(setup.server_args) if serve else None, bundle=host.hash))
        # On Linux, its games at once a pinned core's share (`distributed.pin_plan`) unless it names them.
        out_job["lanes_per_core"] = None if w.get("duel_parallel") else w.get("lanes_per_core", distributed.LANES_PER_CORE)
        return out_job

    binaries = LINUX_BINARIES + (LINUX_RECORD if record else ())
    whole = {"global": identity, "hosts": hosts_identity}
    if restart_stale and (why := distributed.stale(dist_dir / "ledger.json", games, seed, whole, header, only)):
        log(f"  {name}: its ledger is another run's ({why.splitlines()[0]}): started over")
        shutil.rmtree(dist_dir)
        out.mkdir(parents=True)
    with distributed.LedgerLock(dist_dir / "ledger.lock"):
        ledger = distributed.Ledger.load(dist_dir / "ledger.json", games, seed, header=header, only=only, identity=whole,
                                         log=log)
        ledger.save()
        if not ledger.complete():
            # A stop asked of an earlier run of this duel is not this one's: its local run's, and the whole run's.
            (out / kind.stop).unlink(missing_ok=True)
            stop_file(setup).unlink(missing_ok=True)
        with (setup.server(models) if serve and setup.local and not ledger.complete() else contextlib.nullcontext(None)) as gpu:
            hosts = []
            if setup.local:
                command = [*setup.botlab_local, *local_args, "--parallel", setup.local_parallel, "--out", out]
                if gpu is not None:
                    command += ["--ai-server", gpu.address]
                local_env = {"CANTRIP_NATIVE_HOST": str(setup.record_native if record else setup.native),
                             "CANTRIP_AI_HOST": str(setup.ai_host), "DOTNET_ROLL_FORWARD": "Major", **(env or {})}
                hosts.append(distributed.LocalHost(setup.local_name, command, out, dist_dir, setup.gen_dir / "logs",
                                                   env=local_env, cwd=setup.repo, lease=setup.lease, kind=kind))
            hosts += [distributed.host_class(w)(w, tag, setup.gen, *bundles[w.get("os", "windows")], job, log,
                                                lease=setup.lease, kind=kind, task=name, binaries=binaries)
                      for w in setup.workers]
            if not hosts:
                raise SystemExit(f"{name}: no host to play on")
            coordinator = distributed.Coordinator(
                ledger, hosts, out, dist_dir, read=read_results, log=log, chunk=setup.chunk,
                initial=max(setup.chunk, setup.local_parallel, setup.worker_parallel),
                poll_seconds=setup.poll, timeout=setup.timeout, kind=kind, affinity=affinity, stop_file=stop_file(setup),
                stop_check=sequential_stop(setup, local_args, stops, out) if stops else None)
            started = time.time()
            if not ledger.complete():
                log(f"  {name}: {len(ledger.wanted())} games on {', '.join(h.name for h in hosts)}; ledger {dist_dir / 'ledger.json'}")
                if coordinator.run() == "stopped":
                    raise SystemExit(f"{name} stopped on every host: {len(ledger.done)} of {len(ledger.wanted())} games "
                                     f"are in; run the driver again with the same arguments to go on (the ledger: "
                                     f"{dist_dir / 'ledger.json'})")
    return ledger, out, merged(ledger, out), time.time() - started


def stop_file(setup: Setup) -> Path:
    """The stop asked of a generation's distributed duels and judgements (`generation.py --stop` writes it): apart from
    each run's own, which its coordinator writes."""
    return setup.gen_dir / "dist" / "stop"


def print_header(setup: Setup, args: list[str], env: dict | None = None) -> dict:
    """The header the duel's results carry, asked of BotLab before any game (`duel --print-header`)."""
    p = subprocess.run([str(c) for c in setup.botlab_local] + args + ["--print-header"], cwd=setup.repo,
                       capture_output=True, text=True, env={**os.environ, "DOTNET_ROLL_FORWARD": "Major", **(env or {})})
    if p.returncode != 0:
        raise SystemExit(f"BotLab duel --print-header failed:\n{p.stdout}{p.stderr}")
    return json.loads(p.stdout.strip().splitlines()[-1])


def summarize(setup: Setup, args: list[str], results: Path, seconds: float = 0.0, stops: list | None = None) -> tuple[int, str]:
    """BotLab's summary of a results file (`duel --summarize`): its exit code (1 when a game did not finish, as the
    duel's) and its text, the duel's own lines."""
    p = subprocess.run([str(c) for c in setup.botlab_local] + args + [str(s) for s in stops or []]
                       + ["--summarize", str(results), "--elapsed", f"{seconds:.1f}"], cwd=setup.repo, capture_output=True,
                       text=True, env={**os.environ, "DOTNET_ROLL_FORWARD": "Major"})
    if p.returncode not in (0, 1):
        raise SystemExit(f"BotLab duel --summarize failed ({p.returncode}):\n{p.stdout}{p.stderr}")
    return p.returncode, p.stdout


def stopped_by(text: str) -> str | None:
    """The sequential test that stopped a duel, by its summary's verdict line, or None."""
    for line in text.splitlines():
        if line.startswith(("SPRT verdict after", "Paired SPRT verdict after", "Interval stop after")):
            return line.strip()
    return None


def sequential_stop(setup: Setup, args: list[str], stops: list, out: Path):
    """The coordinator's `stop_check` for a duel's sequential tests: its results so far summarised, in the order they
    came, whenever new ones came; the verdict line once a test has one."""
    seen = {"n": -1}

    def check(ledger: distributed.Ledger) -> str | None:
        if len(ledger.shards) == seen["n"]:
            return None
        seen["n"] = len(ledger.shards)
        path = out.parent / "so-far.tsv"
        write_results(merged(ledger, out), path)
        _, text = summarize(setup, args, path, stops=stops)
        return stopped_by(text)

    return check


def hosts_of(ledger: distributed.Ledger, local: str) -> dict[int, str]:
    """Which host played each game of a duel: what its replays are held to (`affinity`); `local` this machine's name,
    for an output recorded with none."""
    out = {}
    for name, s in ledger.shards.items():
        for i in distributed.from_ranges(s["indices"]):
            out[i] = s.get("host") or local
    return out


def check_published(botlab: dict, workers: list[dict]):
    """BotLab published for every kind of worker there is (the build stage's `dotnet publish`), or why not."""
    for kind in sorted({w.get("os", "windows") for w in workers}):
        if not Path(botlab[kind]).exists():
            raise SystemExit(f"a distributed duel needs BotLab published for its {kind} workers, and {botlab[kind]} is "
                             "not there: run the driver without --no-build (its build stage publishes it), or "
                             "--no-dist-duels to play the duels here")


def host_rows(ledger: distributed.Ledger) -> dict[str, dict]:
    """Each host's part of a duel: its games and its games an hour."""
    return {h: {k: r.get(k) for k in ("games", "games_per_hour", "attempts")}
            for h, r in distributed.host_rows(ledger).items()}


def recordings(ledger: distributed.Ledger, out: Path, record: str, dest: Path) -> int:
    """A replay's recordings, every accepted output's, gathered into `dest` (the miner's `recorded/<half>`): its shards
    and its traces (`traces/*.jsonl`, the AI host's), each named after its output so none meets another's. How many
    shards."""
    (dest / "traces").mkdir(parents=True, exist_ok=True)
    n = 0
    for name in ledger.shards:
        for p in sorted((out / name / record).glob("*.shard.zst")):
            shutil.copy2(p, dest / f"{name}-{p.name}")
            n += 1
        for p in sorted((out / name / record / "traces").glob("*.jsonl")):
            shutil.copy2(p, dest / "traces" / f"{name}-{p.name}")
    return n
