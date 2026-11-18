"""Pair mining as a stage of every generation (docs/ai-training.md, "Pair mining"): the gate's pairs judged by a deep
search, and the judgements the next generation's training targets.

The gate is played in pairs (one deal, the networks exchanging seats). After it, `prepare` (the driver's `mine`
stage):

1. chooses the pairs (`mine select`): every decisive pair (one network won both games) and a control of split pairs;
2. plays their games again, recording every position (BotLab `--only-games` with the gate's settings, through the native
   host built with `ai-record`): a game's searches are seeded by its number, so it replays exactly, on the hardware that
   played it (this machine);
3. checks every replayed game's result is the gate's (`mine verify`); a pair whose games differ is left out;
4. finds each pair's fork, the first decision its two games answer differently (`mine-forks`), and the loser's
   decisions before its largest fall of value (`mine-values`, `mine losers`), and plans them (`mine plan`), a share of
   the pairs held out;
5. starts the deep judgements (`mine-judge`): on a worker (`--workers`, over SSH as distributed self-play's) while the
   driver plays the network duel and the ladder, or here. The judge is the network the gate leaves champion.

Then `finish` (the `mined` stage, after the ladder) waits for the judgements, keeps the decided forks and the loser
decisions whose deep target is far from the network's prior (`mine keep`), writes them as policy-only shards
(`mine-filter`): `genN/mine/targets/` for training and `genN/mine/heldout/` for measuring, and the report
(`genN/mine/REPORT.md`). Outcomes only choose which games are looked at; every target is the deep search's.

`window_dirs` gives the training window the targets of the last generations (`--mine-window`), each at `--mine-weight`
halved per generation of age."""

from __future__ import annotations

import json
import shutil
import tarfile
import time
from dataclasses import dataclass
from pathlib import Path

from . import distributed

# The judgements, as the pair miner made gen5's: each fork by two independent seeds (pooled), the first also writing
# the targets; the losers' decisions once, with targets.
FORK_RUNS = (("forks-a", 1, True), ("forks-b", 2, False))
LOSERS_RUN = "losers"


@dataclass
class Settings:
    fork_simulations: int = 8192
    fork_worlds: int = 32
    loser_simulations: int = 4096
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


def judge_runs(p: Paths, s: Settings, model: str, prior: str, here: Path, judge_dir: str) -> list[dict]:
    """The `mine-judge` runs, each `name` and its arguments with the plans and recordings under `here` and its output
    under `judge_dir` (paths as the machine running them sees them); the server's address is added there."""
    dirs = ["--dir", f"first={here / 'recorded' / 'first'}", "--dir", f"second={here / 'recorded' / 'second'}"]
    common = ["--model", model, "--batch", str(s.batch), "--lanes", str(s.lanes), "--search", "RootShortcut=false",
              "--prior", prior]
    runs = []
    for name, seed, targets in FORK_RUNS:
        runs.append(dict(name=name, args=["mine-judge", "--plan", str(here / "plan-forks.jsonl"), *dirs, "--out",
                                          f"{judge_dir}/{name}", "--simulations", str(s.fork_simulations), "--worlds",
                                          str(s.fork_worlds), "--seed", str(seed), "--targets", str(targets).lower(), *common]))
    runs.append(dict(name=LOSERS_RUN, args=["mine-judge", "--plan", str(here / "plan-losers.jsonl"), *dirs, "--out",
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


def judge_data(p: Paths) -> Path:
    """The plans and the recordings, one archive for a worker."""
    out = p.root / "judge-data.tar"
    with tarfile.open(out, "w") as t:
        for f in (p.plan_forks, p.plan_losers):
            t.add(f, arcname=f.name)
        t.add(p.recorded, arcname="recorded")
    return out


# The worker's script: the inference server, every judge run at once, the judgements archived, its exit last.
JUDGE_PS1 = r"""
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$job = Get-Content -Raw job.json | ConvertFrom-Json
foreach ($p in $job.env.PSObject.Properties) { Set-Item -Path ("env:" + $p.Name) -Value $p.Value }
function Who($proc) { [ordered]@{ pid = $proc.Id; start = $proc.StartTime.ToFileTimeUtc(); path = $proc.Path } }
$pids = [ordered]@{ supervisor = (Who (Get-Process -Id $PID)) }
function Save-Pids { $pids | ConvertTo-Json | Set-Content -Encoding Ascii pids.json.tmp; Move-Item -Force pids.json.tmp pids.json }
function Save-Exit($code, $why) {
  [ordered]@{ code = $code; why = $why; at = (Get-Date -Format s) } | ConvertTo-Json | Set-Content -Encoding Ascii exit.json.tmp
  Move-Item -Force exit.json.tmp exit.json
}
$infer = $null
$runs = @()
$code = -1
$why = ''
try {
  Save-Pids
  tar -xf judge-data.tar
  if ($LASTEXITCODE -ne 0) { throw 'the judge data did not unpack' }
  Remove-Item -ErrorAction SilentlyContinue infer.ready
  $infer = Start-Process -FilePath $job.infer -ArgumentList $job.infer_args -RedirectStandardOutput infer.out.log -RedirectStandardError infer.log -PassThru -NoNewWindow
  $null = $infer.Handle
  $pids.infer = (Who $infer); Save-Pids
  $deadline = (Get-Date).AddMinutes(10)
  while (-not (Test-Path infer.ready)) {
    if ($infer.HasExited -or (Get-Date) -gt $deadline) { $code = -2; throw 'the inference server did not start (infer.log)' }
    Start-Sleep -Milliseconds 300
  }
  $addr = (Get-Content -Raw infer.ready).Trim()
  foreach ($r in $job.runs) {
    $a = @($r.args) + @('--server', $addr)
    $p = Start-Process -FilePath $job.explore -ArgumentList $a -RedirectStandardOutput "$($r.name).out.log" -RedirectStandardError "$($r.name).log" -PassThru -NoNewWindow
    $null = $p.Handle
    $runs += $p
    $pids[$r.name] = (Who $p); Save-Pids
  }
  foreach ($p in $runs) { $p.WaitForExit() }
  $failed = @($runs | Where-Object { $_.ExitCode -ne 0 })
  if ($failed.Count) { $code = 1; $why = "$($failed.Count) judge run(s) failed (their logs)" } else {
    tar -cf judge.tar judge *.log
    $code = $LASTEXITCODE
  }
} catch {
  $why = "$_"
} finally {
  foreach ($p in $runs) { if (-not $p.HasExited) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue } }
  Save-Exit $code $why
  if ($infer -and -not $infer.HasExited) { Stop-Process -Id $infer.Id -Force -ErrorAction SilentlyContinue }
}
"""


class RemoteJudge:
    """The judgements on a worker: the bundle (cantrip-explore, cantrip-infer, their libraries, the judge network, the
    decklists) pushed as distributed self-play pushes its own (`SshHost.push`: each file once), the plans and recordings
    as one archive, then `JUDGE_PS1` started detached in `ROOT/gens/<tag>/genN/<host>/mine-<nonce>`. `poll` reads its
    exit; `fetch` brings the judgements home."""

    def __init__(self, worker: dict, tag: str, gen: int, files: dict[str, Path], cache: dict, log=print):
        bundle_hash, manifest = distributed.bundle_manifest(files, cache)
        self.host = distributed.SshHost(worker, tag, gen, files, manifest, bundle_hash, job=None, log=log)
        self.log = log
        self.name = worker["name"]

    def job_dir(self, nonce: str) -> str:
        return f"{self.host.root}/gens/{self.host.tag}/gen{self.host.gen}/{self.name}/mine-{nonce}"

    def start(self, data: Path, model_name: str, s: Settings, server_args: list[str], nonce: str,
              runs: list[dict] | None = None) -> str:
        """The job started; `runs` (each `name` and its `mine-judge` arguments, the model and prior as `{model}` and
        `{prior}`) in place of the miner's own (`judge_runs`), for an experiment over the same plans."""
        self.host.push()
        d = self.job_dir(nonce)
        b = self.host.bundle_dir()
        if runs is None:
            runs = judge_runs(Paths(Path(".")), s, f"{b}/model/{model_name}", f"{b}/decks", Path("."), "judge")
        else:
            runs = [dict(r, args=[a.replace("{model}", f"{b}/model/{model_name}").replace("{prior}", f"{b}/decks")
                                  for a in r["args"]]) for r in runs]
        for r in runs:  # the plans and recordings as the job's own directory holds them
            r["args"] = [a.replace("\\", "/") for a in r["args"]]
        job = dict(explore=f"{b}/bin/cantrip-explore.exe", infer=f"{b}/bin/cantrip-infer.exe",
                   infer_args=["serve", "--model", f"{b}/model/{model_name}", "--port", "0", "--ready-file", "infer.ready",
                               *server_args],
                   env={"ORT_DYLIB_PATH": f"{b}/bin/onnxruntime.dll", "CANTRIP_CUDA_LIBS": f"{b}/bin",
                        "CANTRIP_PROGRESS": "0"}, runs=runs)
        self.host.ps(f"New-Item -ItemType Directory -Force {distributed.ps_quote(d)} | Out-Null")
        self.log(f"  [{self.name}] pushing the judge data ({data.stat().st_size / 2**20:.0f} MB)")
        self.host.scp(str(data), self.host.remote_path(f"gens/{self.host.tag}/gen{self.host.gen}/{self.name}/mine-{nonce}/judge-data.tar"))
        r = self.host.ps_json(f"""
$d = {distributed.ps_quote(d)}
function Put($name, $text) {{ [IO.File]::WriteAllText("$d/$name.tmp", [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($text))); Move-Item -Force "$d/$name.tmp" "$d/$name" }}
Put 'run.ps1' {distributed.ps_quote(distributed.b64(JUDGE_PS1))}
Put 'job.json' {distributed.ps_quote(distributed.b64(json.dumps(job, indent=1)))}
Remove-Item -ErrorAction SilentlyContinue "$d/exit.json"
$line = 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + ($d -replace '/', '\\') + '\\run.ps1"'
$r = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{{ CommandLine = $line; CurrentDirectory = ($d -replace '/', '\\') }}
if ($r.ReturnValue -ne 0) {{ throw "Win32_Process.Create returned $($r.ReturnValue)" }}
ConvertTo-Json -Compress -InputObject @{{ pid = [int]$r.ProcessId }}
""")
        self.log(f"  [{self.name}] judging in {d} (supervisor {r.get('pid')}): {', '.join(x['name'] for x in runs)}")
        return d

    def poll(self, nonce: str) -> dict | None:
        """The job's exit (`code`, `why`), or None while it runs."""
        d = self.job_dir(nonce)
        return self.host.ps_json(f"""
$d = {distributed.ps_quote(d)}
if (Test-Path "$d/exit.json") {{ Get-Content -Raw "$d/exit.json" | ConvertFrom-Json | ConvertTo-Json -Compress }} else {{ 'null' }}
""")

    def fetch(self, nonce: str, dest: Path):
        dest.mkdir(parents=True, exist_ok=True)
        tar = dest.parent / "judge.tar"
        self.host.scp(self.host.remote_path(f"gens/{self.host.tag}/gen{self.host.gen}/{self.name}/mine-{nonce}/judge.tar"), str(tar))
        with tarfile.open(tar) as t:
            t.extractall(dest.parent, filter="data")
        tar.unlink()


def bundle(explore: Path, infer: Path, ort_dir: Path, cuda_dir: Path, model: Path, decks: Path, cuda_dlls) -> dict[str, Path]:
    """The judge's bundle: self-play's (its binaries' libraries, onnxruntime's and CUDA's) with cantrip-explore."""
    files = distributed.bundle_files(explore, infer, ort_dir, cuda_dir, model, decks, None, None, cuda_dlls)
    files["bin/cantrip-explore.exe"] = files.pop("bin/cantrip-selfplay.exe")
    return files


def window_dirs(root: Path, gen: int, window: int, weight: float) -> list[str]:
    """The mined targets the training of generation `gen` takes: those of the `window` generations before it that have
    them, the newest at `weight`, each older one at half the one after it."""
    out = []
    w = weight
    for g in range(gen - 1, max(gen - 1 - window, -1), -1):
        targets = root / f"gen{g}" / "mine" / "targets"
        if targets.is_dir():
            for d in sorted(x for x in targets.iterdir() if x.is_dir() and any(x.glob("*.shard.zst"))):
                out.append(f"{d}:{w:g}")
        w /= 2
    return out


def wait(judge: RemoteJudge, nonce: str, poll_seconds: float, timeout_hours: float, log=print) -> dict:
    """The remote judgements' exit, polling; an unreachable worker is retried until the timeout."""
    until = time.time() + timeout_hours * 3600
    while True:
        try:
            exit_ = judge.poll(nonce)
        except distributed.Unreachable as e:
            log(f"  [{judge.name}] unreachable ({e}); retrying")
            exit_ = None
        if exit_ is not None:
            return exit_
        if time.time() > until:
            raise SystemExit(f"the judgements on {judge.name} did not finish in {timeout_hours:g} h")
        time.sleep(poll_seconds)


def clear(path: Path):
    if path.is_dir():
        shutil.rmtree(path)
    elif path.exists():
        path.unlink()
