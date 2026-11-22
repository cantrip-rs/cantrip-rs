"""Pair mining as a stage of every generation (docs/ai-training.md, "Pair mining"): the gate's pairs judged by a deep
search, and the judgements the next generation's training targets.

The gate is played in pairs (one deal, the networks exchanging seats). After it, `prepare` (the driver's `mine`
stage):

1. chooses the pairs (`mine select`): every decisive pair (one network won both games) and a control of split pairs;
2. plays their games again, recording every position (BotLab `--only-games` with the gate's settings, through the native
   host built with `ai-record`): a game's searches are seeded by its number, so it replays exactly, on the hardware that
   played it (this machine, or with a distributed gate each game on its host: `duels.REPLAY`);
3. checks every replayed game's result is the gate's (`mine verify`); a pair whose games differ is left out;
4. finds each pair's fork, the first decision its two games answer differently (`mine-forks`), and the loser's
   decisions before its largest fall of value (`mine-values`, `mine losers`), and plans them (`mine plan`), a share of
   the pairs held out;
5. judges them (`mine-judge`): here, or with `--workers` across this machine and the workers (`judge_distributed`:
   each run's plan's games split among the hosts by the self-play coordinator, the parts merged). The judge is the
   network the gate leaves champion.

Then `finish` (the `mined` stage, after the ladder) keeps the decided forks and the loser
decisions whose deep target is far from the network's prior (`mine keep`), writes them as policy-only shards
(`mine-filter`): `genN/mine/targets/` for training and `genN/mine/heldout/` for measuring, and the report
(`genN/mine/REPORT.md`). Outcomes only choose which games are looked at; every target is the deep search's.

`weighted_dirs` gives the training window the targets of the last generations (`--mine-window`), each weighted by the
effort of the search that made it over self-play's full search (`effort.json`), halved per generation of age, and all of
them scaled up, if need be, to at least `--mine-floor` of the policy-target weight."""

from __future__ import annotations

import contextlib
import json
import shutil
from dataclasses import dataclass
from pathlib import Path

from . import distributed

# The judgements. Deciding apart from teaching (2026-11-18): each fork's verdict from two independent seeds at the
# fork budget (pooled), which decides which forks are kept; the targets from one deeper run (`DEEP_RUN`); the losers'
# decisions once, with targets. The first verdict run also searches the position unfloored (the report's agreement).
FORK_RUNS = (("forks-a", 1, True), ("forks-b", 2, False))
DEEP_RUN = "forks-deep"
LOSERS_RUN = "losers"


@dataclass
class Settings:
    # The verdicts at 8192 x 32 a seed, two seeds pooled (64 paired worlds: about two thirds of gen8's forks decided,
    # in 23 minutes on a worker); at 32768 x 16 a seed gen9's were decided at 45% and took about two hours.
    fork_simulations: int = 8192
    fork_worlds: int = 32
    # The targets deeper: on gen8's 397 decided forks a search picked the pooled verdict's better move 69.5% of the
    # time at 64 simulations, 75.8% at 512, 79.1% at 4096 x 8 and 82.9% at 32768 x 8, still rising (2026-11-18).
    deep_simulations: int = 32768
    deep_worlds: int = 8
    # The losers' decisions at 8192 x 16.
    loser_simulations: int = 8192
    loser_worlds: int = 16
    batch: int = 32
    lanes: int = 24
    control: int = 200
    heldout: float = 0.15
    before: int = 3
    kl_min: float = 0.1
    seed: int = 5


class Paths:
    def __init__(self, gen_dir: Path):
        self.root = gen_dir / "mine"
        self.selected = self.root / "selected"
        self.replay = self.root / "replay"
        self.recorded = self.root / "recorded"
        self.forks = self.root / "forks.jsonl"
        self.plan = self.root / "plan.jsonl"
        self.plan_forks = self.root / "plan-forks.jsonl"
        self.plan_losers = self.root / "plan-losers.jsonl"
        self.judge = self.root / "judge"
        self.keep = self.root / "keep"
        self.targets = self.root / "targets"
        self.heldout = self.root / "heldout"
        self.report = self.root / "REPORT.md"
        self.job = self.root / "job.json"

    def values(self, d: str) -> Path:
        return self.root / f"values-{d}.jsonl"


def judge_runs(p: Paths, s: Settings, model: str, prior: str, here, judge_dir: str) -> list[dict]:
    """The `mine-judge` runs, each `name` and its arguments with the plans and recordings under `here` and its output
    under `judge_dir` (paths as the machine running them sees them, a worker's as it names them); the server's address
    is added there."""
    h = str(here).rstrip("/\\")
    dirs = ["--dir", f"first={h}/recorded/first", "--dir", f"second={h}/recorded/second"]
    common = ["--model", model, "--batch", str(s.batch), "--lanes", str(s.lanes), "--search", "RootShortcut=false",
              "--prior", prior]
    runs = []
    for name, seed, targets in FORK_RUNS:
        runs.append(dict(name=name, args=["mine-judge", "--plan", f"{h}/plan-forks.jsonl", *dirs, "--out",
                                          f"{judge_dir}/{name}", "--simulations", str(s.fork_simulations), "--worlds",
                                          str(s.fork_worlds), "--seed", str(seed), "--targets", str(targets).lower(), *common]))
    runs.append(dict(name=DEEP_RUN, args=["mine-judge", "--plan", f"{h}/plan-forks.jsonl", *dirs, "--out",
                                          f"{judge_dir}/{DEEP_RUN}", "--simulations", str(s.deep_simulations), "--worlds",
                                          str(s.deep_worlds), "--seed", "3", "--targets", "true", *common]))
    runs.append(dict(name=LOSERS_RUN, args=["mine-judge", "--plan", f"{h}/plan-losers.jsonl", *dirs, "--out",
                                            f"{judge_dir}/{LOSERS_RUN}", "--simulations", str(s.loser_simulations),
                                            "--worlds", str(s.loser_worlds), "--seed", "1", "--targets", "true", *common]))
    return runs


def selection(p: Paths) -> dict:
    return json.loads((p.selected / "selection.json").read_text(encoding="utf-8"))


def prune_selection(p: Paths, verified: dict) -> int:
    """The selection less every pair a game of which was replayed with another result than the gate's, or not replayed
    at all (`mine verify`): only pairs both of whose games are the gate's are mined. How many pairs were left out."""
    from . import mine  # noqa: PLC0415

    sel = mine.load_selection(p.selected)
    differ = {int(d["game"]) for d in verified.get("differ", [])}
    replayed = set()
    for f in sorted(p.replay.glob("*-results.tsv")):
        replayed |= {int(r["game"]) for r in mine.read_results(f)}
    bad = {pair for pair, v in sel["pairs"].items() if any(g in differ or g not in replayed for g in v["games"])}
    if bad:
        sel["pairs"] = {k: v for k, v in sel["pairs"].items() if k not in bad}
        sel["decisive"] = [x for x in sel["decisive"] if x not in bad]
        sel["control"] = [x for x in sel["control"] if x not in bad]
        sel["ids"] = {g: i for g, i in sel["ids"].items() if any(g in v["games"] for v in sel["pairs"].values())}
        sel["left_out"] = sorted(bad)
        mine.write_selection(sel, p.selected)
    return len(bad)


def split_plan(p: Paths) -> dict:
    """The plan's forks and losers' decisions apart (`mine-judge` takes one plan a run)."""
    forks, losers = [], []
    for line in p.plan.read_text(encoding="utf-8").splitlines():
        if line.strip():
            (forks if '"candidates"' in line else losers).append(line)
    p.plan_forks.write_text("".join(x + "\n" for x in forks), encoding="utf-8")
    p.plan_losers.write_text("".join(x + "\n" for x in losers), encoding="utf-8")
    return dict(forks=len(forks), losers=len(losers))


# --- The judgements, distributed ----------------------------------------------------------------------------------

# A judgement's part on a host (docs/ai-training.md, "Distributed judgements"): `cantrip-explore mine-judge --grant`,
# the plan's games it is granted (read once), its output closed by a rename into one directory (`ATTEMPT.judged`).
JUDGE = distributed.Kind(name="judge", out="judge", outputs="*.judged", dirs=True, status="judge-status.json",
                         report="judge-report.json", stop="judge.stop", runaways=None, lease=False, recovers=False)
# What a Linux worker builds for the judgements (`distributed.LINUX_BUILD`).
LINUX_BINARIES = ("cantrip-explore", "cantrip-infer")


def plan_games(plan: Path) -> list[tuple[str, int]]:
    """A plan's games in the order `mine-judge` numbers them for `--grant` (`mine::run`'s work: by recorded
    directory, then seed)."""
    games = set()
    for line in plan.read_text(encoding="utf-8").splitlines():
        if line.strip():
            d = json.loads(line)
            games.add((d["dir"], int(d["seed"])))
    return sorted(games)


def judge_header(games: int, args: list[str]) -> dict:
    """What a part's output must say of itself: the plan's games, and the search it judged them with."""
    def arg(k):
        return args[args.index(k) + 1] if k in args[:-1] else None
    return dict(seeds=f"0..{games}", simulations=arg("--simulations"), worlds=arg("--worlds"), seed=arg("--seed"),
                targets=arg("--targets") != "false")


def read_judged(path: Path, games: int) -> tuple[dict, list[int], dict]:
    """A part's output (`ATTEMPT.judged`) as the coordinator takes it: its header (from its report), the plan's games
    it judged (`items.txt`), and what it holds. A part that lost a position to its inference server is no result: it
    is refused, and its games are judged again."""
    report = json.loads((path / "judge-report.json").read_text(encoding="utf-8"))
    items = [int(x) for x in (path / "items.txt").read_text(encoding="utf-8").split()]
    lines = [json.loads(x) for x in (path / "judged.jsonl").read_text(encoding="utf-8").splitlines() if x.strip()]
    served = [x for x in lines if "inference" in str((x.get("judgement") or {}).get("error") or "")]
    if served:
        raise ValueError(f"{len(served)} of {len(lines)} positions lost to its inference server: "
                         f"{(served[0].get('judgement') or {}).get('error')}")
    header = dict(seeds=f"0..{games}", simulations=str(report.get("Simulations")), worlds=str(report.get("Worlds")),
                  seed=str(report.get("Seed")), targets=bool(report.get("Targets")))
    return header, items, dict(games=len(items), positions=len(lines), samples=report.get("Samples", 0))


def merge_judged(ledger: distributed.Ledger, out: Path, dest: Path):
    """A run's judgements as one machine writes them (`dest`, the miner's `judge/<run>`): every accepted part's lines in
    `judged.jsonl`, by position id, and its target shards under `<split>/<directory>/`, each named after its part."""
    clear(dest)
    dest.mkdir(parents=True)
    lines = []
    for name in ledger.shards:
        part = out / name
        lines += [x for x in (part / "judged.jsonl").read_text(encoding="utf-8").splitlines() if x.strip()]
        for split in ("train", "heldout"):
            for d in sorted(x for x in (part / split).glob("*") if x.is_dir()):
                (dest / split / d.name).mkdir(parents=True, exist_ok=True)
                for f in sorted(d.glob("*.shard.zst")):
                    shutil.copy2(f, dest / split / d.name / f"{name}-{f.name}")
    lines.sort(key=lambda x: json.loads(x)["id"])
    (dest / "judged.jsonl").write_text("".join(x + "\n" for x in lines), encoding="utf-8")


def judge_data_files(p: Paths) -> dict[str, Path]:
    """The plans and the recordings (shards and traces), by their place in a worker's bundle (`data/`)."""
    files = {"data/plan-forks.jsonl": p.plan_forks, "data/plan-losers.jsonl": p.plan_losers}
    for half in ("first", "second"):
        d = p.recorded / half
        for f in sorted(d.glob("*.shard.zst")):
            files[f"data/recorded/{half}/{f.name}"] = f
        for f in sorted((d / "traces").glob("*.jsonl")):
            files[f"data/recorded/{half}/traces/{f.name}"] = f
    return files


def with_arg(args: list[str], flag: str, value: str) -> list[str]:
    """`args` with `flag` set to `value` (added if it is not there)."""
    out = list(args)
    if flag in out[:-1]:
        out[out.index(flag) + 1] = value
    else:
        out += [flag, value]
    return out


def judge_distributed(setup, p: Paths, s: Settings, judge_model: Path, decks: Path, explore: Path, local_lanes: int,
                      worker_lanes: int, parts: int = 3, runs=None, data: dict[str, Path] | None = None,
                      chunk: int | None = None, stop_check=None) -> dict:
    """The judgements across this machine and the workers (`setup`: a `duels.Setup`): each run of `judge_runs` in
    turn, its plan's games split among the hosts by the self-play coordinator (`JUDGE`), each host's part a
    `mine-judge --grant` beside an inference server of its own (every host's with the same arguments), a part about
    a `parts`-th of a host's share; a host lost or failing has its parts judged by the others. The accepted parts are
    merged into `p.judge/<run>` as one machine writes them. Returns each run's hosts' parts.

    Another caller's judgements than the pair miner's (`reanalyse.py`) are the same with their own `runs` (given the
    model, the decklists, where the plans and recordings are and where the output goes, as `judge_runs` is: the runs
    to make, in turn), their own `data` (a worker's bundle's `data/`, as `judge_data_files` names it), a part's size
    in games (`chunk`) and a `stop_check` (the coordinator's: given the ledger after each poll, a reason to close the
    run or None - a time budget: nothing more is granted, the parts in flight are finished and taken, and the games
    not judged are skipped)."""
    log = setup.log
    tag = distributed.root_tag(setup.root)
    data = judge_data_files(p) if data is None else data
    if runs is None:
        def runs(model, prior, here, judge_dir):
            return judge_runs(p, s, model, prior, here, judge_dir)
    bundles: dict[str, tuple] = {}
    by_os = {w.get("os", "windows") for w in setup.workers}
    if "windows" in by_os:
        if setup.ort_dir is None or setup.cuda_dir is None:
            raise SystemExit("a Windows worker's judgements need onnxruntime-gpu (--ort-dylib) and CUDA's libraries")
        files = {**bundle(explore, setup.infer, setup.ort_dir, setup.cuda_dir, judge_model, decks, setup.cuda_dlls), **data}
        bundles["windows"] = (files, *reversed(distributed.bundle_manifest(files, setup.cache)))
    if "linux" in by_os:
        files = {**distributed.linux_bundle_files(setup.source() if setup.source is not None
                                                        else distributed.source_tar(setup.repo, setup.root / "dist" / "src"),
                                                  judge_model, decks, None, None), **data}
        bundles["linux"] = (files, *reversed(distributed.bundle_manifest(files, setup.cache)))
    local_runs = {r["name"]: r["args"] for r in runs(str(judge_model), str(decks), p.root, str(p.judge))}
    done = {}
    for name, local_args in local_runs.items():
        plan = Path(local_args[local_args.index("--plan") + 1])
        games = len(plan_games(plan))
        if games == 0:
            merge_judged(distributed.Ledger(None, 0, 0), p.root, p.judge / name)
            done[name] = {}
            continue
        task = f"judge-{name}"
        dist_dir = setup.gen_dir / "dist" / task
        out = dist_dir / "results"
        out.mkdir(parents=True, exist_ok=True)
        header = judge_header(games, local_args)
        identity = dict(header=header, server_args=" ".join(setup.server_args),
                        model=distributed.files_hash([x for x in judge_model.iterdir() if x.is_file() and
                                                      (x.name == "manifest.json" or x.suffix == ".onnx")], setup.cache),
                        plan=distributed.files_hash([plan], setup.cache))

        def job(host, a, d, name=name):
            w, b = host.w, host.bundle_dir()
            linux = w.get("os") == "linux"
            exe = "" if linux else ".exe"
            remote = {r["name"]: r["args"] for r in runs(f"{b}/model/{judge_model.name}", f"{b}/decks", f"{b}/data",
                                                           f"{d}/judge")}[name]
            remote = with_arg(remote, "--out", f"{d}/judge/{a['id']}.judged")
            remote = with_arg(remote, "--lanes", str(w.get("judge_lanes") or worker_lanes))
            remote += ["--grant", f"{d}/grant.txt"]
            env = {"CANTRIP_PROGRESS": "0"}
            if not linux:
                env.update(ORT_DYLIB_PATH=f"{b}/bin/onnxruntime.dll", CANTRIP_CUDA_LIBS=f"{b}/bin")
            return dict(selfplay=f"{b}/bin/cantrip-explore{exe}", selfplay_args=remote, recover_args=None, env=env,
                        infer=f"{b}/bin/cantrip-infer{exe}", server_flag="--server", lanes_per_core=None,
                        infer_args=["serve", "--model", f"{b}/model/{judge_model.name}", "--port", "0", "--ready-file",
                                    f"{d}/infer.ready", "--device", str(w.get("device", 0)), *setup.server_args],
                        record=dict(server_args=" ".join(setup.server_args), bundle=host.hash))

        whole = {"global": identity, "hosts": {w["name"]: dict(bundle=bundles[w.get("os", "windows")][2]) for w in setup.workers}}
        if why := distributed.stale(dist_dir / "ledger.json", games, 0, whole, header):
            # Another plan's (the mine stage made again): started over.
            log(f"  {task}: its ledger is another plan's ({why.splitlines()[0]}): started over")
            shutil.rmtree(dist_dir)
            out.mkdir(parents=True)
        with distributed.LedgerLock(dist_dir / "ledger.lock"):
            ledger = distributed.Ledger.load(dist_dir / "ledger.json", games, 0, header=header, identity=whole)
            ledger.save()
            serving = setup.server is not None and setup.local and not ledger.complete()
            with (setup.server([judge_model]) if serving else contextlib.nullcontext(None)) as gpu:
                hosts = []
                if setup.local:
                    command = [explore, *with_arg(local_args, "--lanes", str(local_lanes))]
                    command = [c for k, c in enumerate(command) if not (c == "--out" or (k and command[k - 1] == "--out"))]
                    if gpu is not None:
                        command += ["--server", gpu.address]
                    hosts.append(distributed.LocalHost(setup.local_name, command, out, dist_dir, setup.gen_dir / "logs",
                                                       env={"CANTRIP_PROGRESS": "0"}, cwd=setup.repo, kind=JUDGE,
                                                       extra=lambda a, out=out: ["--out", str(out / f"{a['id']}.judged")]))
                hosts += [distributed.host_class(w)(w, tag, setup.gen, *bundles[w.get("os", "windows")], job, log,
                                                    kind=JUDGE, task=task, binaries=LINUX_BINARIES)
                          for w in setup.workers]
                if not hosts:
                    raise SystemExit("the judgements: no host to judge on")
                part = chunk or max(1, -(-games // (len(hosts) * max(1, parts))))
                stop = setup.gen_dir / "dist" / "stop"
                coordinator = distributed.Coordinator(
                    ledger, hosts, out, dist_dir, read=lambda path, games=games: read_judged(path, games), log=log,
                    chunk=part, initial=part, lookahead=0.0, poll_seconds=setup.poll, timeout=setup.timeout, kind=JUDGE,
                    stop_file=stop, stop_check=stop_check)
                if not ledger.complete():
                    stop.unlink(missing_ok=True)
                    log(f"  {task}: {games} games' positions on {', '.join(h.name for h in hosts)}, parts of {part}")
                    if coordinator.run() == "stopped":
                        raise SystemExit(f"the judgements stopped: run the driver again to go on ({dist_dir / 'ledger.json'})")
        merge_judged(ledger, out, p.judge / name)
        done[name] = {h: r["games"] for h, r in distributed.host_rows(ledger).items()}
        if ledger.dropped:
            log(f"  {task}: {len(ledger.dropped)} games' positions failed twice and are left unjudged")
    return done


def bundle(explore: Path, infer: Path, ort_dir: Path, cuda_dir: Path, model: Path, decks: Path, cuda_dlls) -> dict[str, Path]:
    """The judge's bundle: self-play's (its binaries' libraries, onnxruntime's and CUDA's) with cantrip-explore."""
    files = distributed.bundle_files(explore, infer, ort_dir, cuda_dir, model, decks, None, None, cuda_dlls)
    files["bin/cantrip-explore.exe"] = files.pop("bin/cantrip-selfplay.exe")
    return files


def check_judged(judge_dir: Path, names) -> dict:
    """Each run's verdicts (`judged.jsonl`); a run that judged none of its positions (its server gone, say: every one
    comes back unjudged) is a failure, not an empty result."""
    counts = {}
    for name in names:
        lines = [json.loads(x) for x in (judge_dir / name / "judged.jsonl").read_text(encoding="utf-8").splitlines() if x.strip()]
        verdicts = [((x.get("judgement") or {}).get("verdict") or ("target" if x.get("target") else "unjudged")) for x in lines]
        judged = sum(v != "unjudged" for v in verdicts)
        # A position left unjudged because the inference server failed is lost work, not a position the deep search
        # cannot stand at (a mulligan after a redraw): any such loss fails the run.
        served = [x for x in lines if "inference" in str((x.get("judgement") or {}).get("error") or "")]
        counts[name] = dict(positions=len(lines), judged=judged, server_failures=len(served))
        if lines and judged == 0:
            raise SystemExit(f"the judge run {name} judged none of its {len(lines)} positions (its log: {judge_dir / name})")
        if served:
            raise SystemExit(f"the judge run {name} lost {len(served)} of {len(lines)} positions to its inference server: "
                             f"{(served[0].get('judgement') or {}).get('error')}")
    return counts


EFFORT_FILE = "effort.json"


def write_effort(directory: Path, simulations: int, worlds: int):
    """The search that made a target directory's targets, beside its shards: what `weighted_dirs` weights them by."""
    (directory / EFFORT_FILE).write_text(json.dumps(dict(simulations=simulations, worlds=worlds)), encoding="utf-8")


def target_samples(directory: Path) -> int:
    """How many positions a target directory's shards hold."""
    import sys  # noqa: PLC0415

    sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
    import shards  # noqa: PLC0415

    return sum(len(game.samples) for path in shards.shards(directory) for game in shards.read(path)[1])


def weighted_dirs(root: Path, gen: int, window: int, full_simulations: int, floor: float, policy_targets: float,
                  log=print) -> list[str]:
    """The mined targets the training of generation `gen` takes, weighted by the effort behind them (a network should
    hold what a deep search finds, so a deeper search's targets count for more): those of
    the `window` generations before it that have them, each directory at its search's simulations over self-play's
    full search's (`effort.json`), halved per generation of age; then all of them scaled up together, if need be, so
    they make at least `floor` of the policy-target weight beside self-play's (`policy_targets`: its directories' policy
    targets times their weights). A directory with no `effort.json` counts as a full search's."""
    dirs = []
    for age, g in enumerate(range(gen - 1, max(gen - 1 - window, -1), -1)):
        targets = root / f"gen{g}" / "mine" / "targets"
        if not targets.is_dir():
            continue
        for d in sorted(x for x in targets.iterdir() if x.is_dir() and any(x.glob("*.shard.zst"))):
            effort = json.loads((d / EFFORT_FILE).read_text(encoding="utf-8")) if (d / EFFORT_FILE).exists() else {}
            w = effort.get("simulations", full_simulations) / max(full_simulations, 1) * 0.5 ** age
            dirs.append((d, w, target_samples(d)))
    mined = sum(w * n for _, w, n in dirs)
    scale = 1.0
    if mined > 0 and floor > 0 and mined < floor / (1 - floor) * policy_targets:
        scale = floor / (1 - floor) * policy_targets / mined
    for d, w, n in dirs:
        log(f"  mined: {d} - {n} positions at {w * scale:.3g} (effort {w:.3g} x {scale:.3g})")
    if dirs:
        share = mined * scale / (mined * scale + policy_targets) if policy_targets else 1.0
        log(f"  mined targets: {share:.1%} of the policy-target weight ({policy_targets:.0f} self-play targets weighted)")
    return [f"{d}:{w * scale:.10g}" for d, w, _ in dirs]


def clear(path: Path):
    if path.is_dir():
        shutil.rmtree(path)
    elif path.exists():
        path.unlink()
